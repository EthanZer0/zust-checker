//! CAS + Wisedu 双步认证。
//!
//! 除了传统的账号密码流程，这里还保留了浏览器登录中间态，用于处理：
//!
//! - CAS 按风险策略要求输入图片验证码；
//! - CAS 登录成功后要求可信设备短信验证；
//! - 短信验证后再继续原有的 Wisedu 教务系统登录。

use crate::crypto::{cas_encrypt_password, wisedu_encrypt_password};
use crate::error::{Result, ZustError};
use crate::session::HttpSession;
use crate::types::SessionData;
use base64::Engine;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

const CAS_BASE: &str = "https://authserver.zust.edu.cn/authserver";
const CAS_SVC: &str = "https://newjwxt.zust.edu.cn/jwglxt/xtgl/login_slogin.html";
const JW_BASE: &str = "https://newjwxt.zust.edu.cn/jwglxt";
const JW_LOGIN: &str = "https://newjwxt.zust.edu.cn/jwglxt/xtgl/login_slogin.html";
const REAUTH_SMS_TYPE: &str = "reAuthDynamicCodeType";

static RE_PWD_SALT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"id="pwdEncryptSalt"\s+value="([^"]+)""#).unwrap());
static RE_EXECUTION: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"name="execution"\s+value="([^"]+)""#).unwrap());
static RE_FORM_ACTION: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"id="pwdFromId"\s+action="([^"]+)""#).unwrap());
static RE_CSRF: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"csrftoken[^>]*value="([^"]+)""#).unwrap());
static RE_CAS_ERROR: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"<span id="msg"[^>]*>([^<]+)</span>"#).unwrap());
static RE_WISEDU_TIPS: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"<p id="tips"[^>]*>([^<]+)</p>"#).unwrap());

fn ajax_headers(referer: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert(
        "Accept".into(),
        "application/json, text/javascript, */*; q=0.01".into(),
    );
    headers.insert("X-Requested-With".into(), "XMLHttpRequest".into());
    headers.insert("Referer".into(), referer.into());
    headers
}

/// 登录步骤事件（发给前端）。
#[derive(Debug, Clone, Serialize)]
pub struct LoginStep {
    pub step: u8,
    pub message: String,
}

impl LoginStep {
    pub fn new(step: u8, message: impl Into<String>) -> Self {
        Self {
            step,
            message: message.into(),
        }
    }
}

/// 图片验证码挑战。图片内容是 data URL 可直接用于 `<img src>`。
#[derive(Debug, Clone, Serialize)]
pub struct CaptchaChallenge {
    pub kind: String,
    pub image_base64: String,
    pub message: String,
}

/// 可信设备认证挑战。
#[derive(Debug, Clone, Serialize)]
pub struct ReauthChallenge {
    pub kind: String,
    pub reauth_type: String,
    pub is_multifactor: bool,
    pub message: String,
}

/// 认证完成后的内部结果。凭据只在进程内传递，不通过 IPC 序列化。
#[derive(Debug)]
pub struct AuthSuccess {
    pub cookies: HashMap<String, String>,
    pub username: String,
    pub cas_password: String,
    pub wisedu_password: String,
}

pub enum AuthFlowResult {
    Complete(AuthSuccess),
    CaptchaRequired(CaptchaChallenge),
    ReauthRequired(ReauthChallenge),
}

#[derive(Clone)]
enum PendingStage {
    Captcha,
    Reauth {
        reauth_type: String,
        is_multifactor: bool,
    },
}

#[derive(Clone)]
struct PendingLogin {
    session: HttpSession,
    username: String,
    cas_password: String,
    wisedu_password: String,
    service: String,
    salt_key: String,
    execution: String,
    form_action: String,
    stage: PendingStage,
}

/// 认证管理器。
///
/// `pending` 让多步认证跨越多个 Tauri command 保持同一个 HTTP cookie 会话，
/// 这点不能用新的 reqwest Client 替代，否则短信校验后的 CAS ticket 会丢失。
#[derive(Clone)]
pub struct AuthManager {
    pub config_dir: PathBuf,
    pending: Arc<Mutex<Option<PendingLogin>>>,
}

impl AuthManager {
    pub fn new() -> Self {
        let config_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("zust-grades");
        Self {
            config_dir,
            pending: Arc::new(Mutex::new(None)),
        }
    }

    fn session_path(&self) -> PathBuf {
        self.config_dir.join("newjwxt-session.json")
    }

    fn fingerprint_path(&self) -> PathBuf {
        self.config_dir.join("browser-fingerprint")
    }

    pub fn load_session(&self) -> Option<HashMap<String, String>> {
        let path = self.session_path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        let data: SessionData = serde_json::from_str(&content).ok()?;
        Some(data.cookies)
    }

    pub fn save_session(&self, cookies: &HashMap<String, String>) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let data = SessionData {
            cookies: cookies.clone(),
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        };
        std::fs::write(self.session_path(), serde_json::to_string_pretty(&data)?)?;
        Ok(())
    }

    pub fn clear_session(&self) -> Result<()> {
        let path = self.session_path();
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        self.clear_pending();
        Ok(())
    }

    fn clear_pending(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            *pending = None;
        }
    }

    fn take_pending(&self) -> Option<PendingLogin> {
        self.pending.lock().ok()?.take()
    }

    fn put_pending(&self, pending: PendingLogin) -> Result<()> {
        self.pending
            .lock()
            .map_err(|_| ZustError::LoginFailed("认证状态锁定失败".into()))?
            .replace(pending);
        Ok(())
    }

    /// 开始登录。若服务端要求额外验证，会返回挑战并保存中间态。
    pub async fn login_start(
        &self,
        username: &str,
        cas_password: &str,
        wisedu_password: &str,
        captcha: Option<&str>,
        on_step: impl Fn(LoginStep),
    ) -> Result<AuthFlowResult> {
        // 图片验证码提交必须复用显示验证码时的会话、salt 和 execution。
        if let Some(captcha_value) = captcha.filter(|value| !value.trim().is_empty()) {
            if let Some(pending) = self.take_pending() {
                let can_resume =
                    pending.username == username && matches!(pending.stage, PendingStage::Captcha);
                if can_resume {
                    let mut pending = pending;
                    pending.cas_password = cas_password.to_string();
                    pending.wisedu_password = wisedu_password.to_string();
                    return self.submit_cas(pending, captcha_value, on_step).await;
                }
                self.put_pending(pending)?;
            }
        } else {
            self.clear_pending();
        }

        on_step(LoginStep::new(1, "连接 CAS 认证服务器..."));
        let session = HttpSession::new()
            .map_err(|e| ZustError::LoginFailed(format!("无法创建 HTTP 会话: {e}")))?;
        let cas_login_url = cas_login_url(CAS_SVC)?;
        self.ensure_browser_fingerprint(&session, &cas_login_url)
            .await;

        let captcha_url = format!(
            "{CAS_BASE}/checkNeedCaptcha.htl?username={}",
            url_escape(username)
        );
        let mut captcha_headers = HashMap::new();
        captcha_headers.insert("X-Requested-With".into(), "XMLHttpRequest".into());
        captcha_headers.insert("Referer".into(), cas_login_url.clone());

        let (page_result, captcha_result) = tokio::join!(
            session.get(&cas_login_url),
            session.get_with_headers(&captcha_url, &captcha_headers),
        );
        let (html, page_url, _) =
            page_result.map_err(|e| ZustError::LoginFailed(format!("无法连接 CAS 服务器: {e}")))?;
        on_step(LoginStep::new(2, "解析 CAS 登录页..."));

        let salt_key = capture_required(&RE_PWD_SALT, &html, "pwdEncryptSalt")?;
        let execution = capture_required(&RE_EXECUTION, &html, "execution")?;
        let form_action = parse_form_action(&html, &page_url)?;
        let has_captcha = match captcha_result {
            Ok((body, _, _)) => captcha_required(&body),
            Err(error) => {
                log::warn!("验证码检测请求失败，将由 CAS 登录结果决定是否需要验证码: {error}");
                false
            }
        };

        let pending = PendingLogin {
            session,
            username: username.to_string(),
            cas_password: cas_password.to_string(),
            wisedu_password: wisedu_password.to_string(),
            service: CAS_SVC.to_string(),
            salt_key,
            execution,
            form_action,
            stage: PendingStage::Captcha,
        };

        if has_captcha && captcha.map(str::trim).filter(|v| !v.is_empty()).is_none() {
            return self.require_captcha(pending).await;
        }

        self.submit_cas(pending, captcha.unwrap_or_default(), on_step)
            .await
    }

    /// 重新拉取当前登录流程的图片验证码。
    pub async fn refresh_captcha(&self) -> Result<CaptchaChallenge> {
        let pending = self
            .take_pending()
            .ok_or_else(|| ZustError::LoginFailed("没有等待中的验证码登录流程".into()))?;
        if !matches!(pending.stage, PendingStage::Captcha) {
            self.put_pending(pending)?;
            return Err(ZustError::LoginFailed(
                "当前登录流程不需要图片验证码".into(),
            ));
        }
        match self.require_captcha(pending).await? {
            AuthFlowResult::CaptchaRequired(challenge) => Ok(challenge),
            _ => unreachable!("require_captcha always returns CaptchaRequired"),
        }
    }

    async fn require_captcha(&self, mut pending: PendingLogin) -> Result<AuthFlowResult> {
        let image_base64 = match fetch_captcha(&pending.session).await {
            Ok(image) => image,
            Err(error) => {
                self.put_pending(pending)?;
                return Err(error);
            }
        };
        pending.stage = PendingStage::Captcha;
        let challenge = CaptchaChallenge {
            kind: "captcha".into(),
            image_base64,
            message: "请输入图片验证码".into(),
        };
        self.put_pending(pending)?;
        Ok(AuthFlowResult::CaptchaRequired(challenge))
    }

    async fn submit_cas(
        &self,
        pending: PendingLogin,
        captcha: &str,
        on_step: impl Fn(LoginStep),
    ) -> Result<AuthFlowResult> {
        on_step(LoginStep::new(3, "CAS 登录中..."));
        let encrypted_pw = cas_encrypt_password(&pending.cas_password, &pending.salt_key)?;
        let form_action = with_service(&pending.form_action, &pending.service)?;
        let form_data = vec![
            ("username", pending.username.as_str()),
            ("password", encrypted_pw.as_str()),
            ("captcha", captcha.trim()),
            ("lt", ""),
            ("execution", pending.execution.as_str()),
            ("_eventId", "submit"),
            ("cllt", "userNameLogin"),
            ("dllt", "generalLogin"),
        ];
        let mut headers = ajax_headers(&form_action);
        headers.insert("Origin".into(), "https://authserver.zust.edu.cn".into());
        let (body, final_url, status) = pending
            .session
            .post_follow_redirects(&form_action, &form_data, &headers, 12)
            .await
            .map_err(|e| ZustError::LoginFailed(format!("CAS 提交失败: {e}")))?;

        // 不可信设备会在 CAS 302 链中落到这个页面，而不是直接发放 ticket。
        if final_url.contains("/reAuthCheck/reAuthLoginView.do") {
            let pending = PendingLogin {
                stage: PendingStage::Reauth {
                    // 日志和当前 reAuth.js 均确认 3 是手机动态码分支。
                    reauth_type: "3".into(),
                    is_multifactor: true,
                },
                ..pending
            };
            let challenge = ReauthChallenge {
                kind: "sms".into(),
                reauth_type: "3".into(),
                is_multifactor: true,
                message: "当前设备不在可信设备列表，请验证手机验证码".into(),
            };
            self.put_pending(pending)?;
            return Ok(AuthFlowResult::ReauthRequired(challenge));
        }

        if !pending.session.has_cookie("MOD_AUTH_CAS") {
            // 服务器可能是在提交后才决定要求验证码，此时把新验证码交给前端。
            if final_url.contains("/authserver/login")
                && (body.contains("captcha") || body.contains("验证码"))
            {
                let pending = PendingLogin {
                    stage: PendingStage::Captcha,
                    ..pending
                };
                return self.require_captcha(pending).await;
            }
            let error = RE_CAS_ERROR
                .captures(&body)
                .and_then(|capture| capture.get(1))
                .map(|value| value.as_str().trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("CAS 登录失败，请检查账号或密码 (status={status})"));
            return Err(ZustError::LoginFailed(error));
        }

        self.finish_wisedu_login(
            &pending.session,
            &pending.username,
            &pending.cas_password,
            &pending.wisedu_password,
            on_step,
        )
        .await
        .map(AuthFlowResult::Complete)
    }

    /// 请求可信设备手机验证码。当前 UI 选择的是手机动态码认证（reAuthType=3）。
    pub async fn send_reauth_code(&self) -> Result<String> {
        let pending = self
            .take_pending()
            .ok_or_else(|| ZustError::LoginFailed("没有等待中的可信设备认证流程".into()))?;
        let reauth_type = match &pending.stage {
            PendingStage::Reauth { reauth_type, .. } => reauth_type.clone(),
            PendingStage::Captcha => {
                self.put_pending(pending)?;
                return Err(ZustError::LoginFailed("当前尚未完成 CAS 登录".into()));
            }
        };
        if reauth_type != "3" {
            self.put_pending(pending)?;
            return Err(ZustError::LoginFailed("当前认证方式不是手机动态码".into()));
        }

        let url = format!("{CAS_BASE}/dynamicCode/getDynamicCodeByReauth.do");
        let data = vec![
            ("userName", pending.username.as_str()),
            ("authCodeTypeName", REAUTH_SMS_TYPE),
        ];
        let headers = ajax_headers(&format!("{CAS_BASE}/reAuthCheck/reAuthLoginView.do"));
        let response = pending
            .session
            .post_with_headers(&url, &data, &headers)
            .await;
        let (body, _, _) = match response {
            Ok(result) => result,
            Err(error) => {
                self.put_pending(pending)?;
                return Err(ZustError::LoginFailed(format!(
                    "获取手机验证码失败: {error}"
                )));
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(error) => {
                self.put_pending(pending)?;
                return Err(ZustError::LoginFailed(format!(
                    "验证码接口返回异常: {error}"
                )));
            }
        };
        let code = value
            .get("res")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let message = response_message(&value).unwrap_or_default();
        let success = code.eq_ignore_ascii_case("success")
            || code.eq_ignore_ascii_case("wechat_success")
            || code.eq_ignore_ascii_case("cpdaily_success");
        self.put_pending(pending)?;
        if success {
            Ok(if message.is_empty() {
                "验证码已发送，请查收手机短信".into()
            } else {
                message
            })
        } else {
            Err(ZustError::LoginFailed(if message.is_empty() {
                "手机验证码发送失败".into()
            } else {
                message
            }))
        }
    }

    /// 提交可信设备手机验证码，并在成功后继续 Wisedu 登录。
    pub async fn verify_reauth_code(
        &self,
        dynamic_code: &str,
        trust_device: bool,
        on_step: impl Fn(LoginStep),
    ) -> Result<AuthSuccess> {
        let pending = self
            .take_pending()
            .ok_or_else(|| ZustError::LoginFailed("没有等待中的可信设备认证流程".into()))?;
        let (reauth_type, is_multifactor) = match &pending.stage {
            PendingStage::Reauth {
                reauth_type,
                is_multifactor,
            } => (reauth_type.clone(), *is_multifactor),
            PendingStage::Captcha => {
                self.put_pending(pending)?;
                return Err(ZustError::LoginFailed("当前尚未进入可信设备认证".into()));
            }
        };
        let dynamic_code = dynamic_code.trim();
        if dynamic_code.len() != 6 || !dynamic_code.chars().all(|c| c.is_ascii_digit()) {
            self.put_pending(pending)?;
            return Err(ZustError::LoginFailed("请输入 6 位数字手机验证码".into()));
        }

        let url = format!("{CAS_BASE}/reAuthCheck/reAuthSubmit.do");
        let data = vec![
            ("service", pending.service.as_str()),
            ("reAuthType", reauth_type.as_str()),
            (
                "isMultifactor",
                if is_multifactor { "true" } else { "false" },
            ),
            ("password", ""),
            ("dynamicCode", dynamic_code),
            ("uuid", ""),
            ("answer1", ""),
            ("answer2", ""),
            ("otpCode", ""),
            ("skipTmpReAuth", if trust_device { "true" } else { "false" }),
        ];
        let headers = ajax_headers(&format!(
            "{CAS_BASE}/reAuthCheck/reAuthLoginView.do?isMultifactor=true"
        ));
        let (body, _, _) = match pending
            .session
            .post_with_headers(&url, &data, &headers)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                self.put_pending(pending)?;
                return Err(ZustError::LoginFailed(format!(
                    "提交可信设备验证码失败: {error}"
                )));
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(error) => {
                self.put_pending(pending)?;
                return Err(ZustError::LoginFailed(format!(
                    "可信设备接口返回异常: {error}"
                )));
            }
        };
        let response_code = value
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if response_code == "reAuth_failed" || response_code == "reAuth_unauthorized" {
            let message =
                response_message(&value).unwrap_or_else(|| "手机验证码错误或已过期".into());
            self.put_pending(pending)?;
            return Err(ZustError::LoginFailed(message));
        }

        // reAuthSubmit 成功后，浏览器会重新访问 CAS /login?service=...，
        // 再沿 ticket 重定向链进入教务系统。
        on_step(LoginStep::new(4, "可信设备验证通过，继续教务系统登录..."));
        let cas_url = cas_login_url(&pending.service)?;
        let (body_after, final_url, _) = match pending
            .session
            .get_follow_redirects(&cas_url, &ajax_headers(&cas_url), 12)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                self.put_pending(pending)?;
                return Err(ZustError::LoginFailed(format!(
                    "继续 CAS 登录失败: {error}"
                )));
            }
        };
        if !pending.session.has_cookie("MOD_AUTH_CAS") {
            self.put_pending(pending)?;
            return Err(ZustError::LoginFailed(
                "可信设备验证成功，但 CAS 会话未建立".into(),
            ));
        }

        let result = self
            .finish_wisedu_login(
                &pending.session,
                &pending.username,
                &pending.cas_password,
                &pending.wisedu_password,
                on_step,
            )
            .await;
        match result {
            Ok(success) => {
                log::debug!(
                    "CAS re-auth redirect completed at {} ({} bytes)",
                    final_url,
                    body_after.len()
                );
                Ok(success)
            }
            Err(error) => {
                self.put_pending(pending)?;
                Err(error)
            }
        }
    }

    /// 兼容原有自动登录调用方。需要验证码或可信设备验证时返回可读错误，
    /// 因为抢课后台没有交互式 UI 来完成多步认证。
    pub async fn full_login(
        &self,
        username: &str,
        cas_password: &str,
        wisedu_password: &str,
        on_step: impl Fn(LoginStep),
    ) -> Result<HashMap<String, String>> {
        match self
            .login_start(username, cas_password, wisedu_password, None, on_step)
            .await?
        {
            AuthFlowResult::Complete(success) => Ok(success.cookies),
            AuthFlowResult::CaptchaRequired(_) => Err(ZustError::LoginFailed(
                "登录需要图片验证码，请在登录窗口完成验证".into(),
            )),
            AuthFlowResult::ReauthRequired(_) => Err(ZustError::LoginFailed(
                "登录需要手机验证码，请在登录窗口完成可信设备验证".into(),
            )),
        }
    }

    async fn finish_wisedu_login(
        &self,
        session: &HttpSession,
        username: &str,
        cas_password: &str,
        wisedu_password: &str,
        on_step: impl Fn(LoginStep),
    ) -> Result<AuthSuccess> {
        on_step(LoginStep::new(5, "Wisedu 教务系统登录中..."));
        let pubkey_url = format!("{JW_BASE}/xtgl/login_getPublicKey.html");
        let mut jw_headers = HashMap::new();
        jw_headers.insert("Referer".into(), "https://newjwxt.zust.edu.cn/".into());
        let pubkey_headers = ajax_headers(JW_LOGIN);
        let (jw_result, pubkey_result) = tokio::join!(
            session.get_with_headers(JW_LOGIN, &jw_headers),
            session.get_with_headers(&pubkey_url, &pubkey_headers),
        );
        let (jw_html, _, _) =
            jw_result.map_err(|e| ZustError::LoginFailed(format!("无法连接教务系统: {e}")))?;
        let csrf_full = capture_required(&RE_CSRF, &jw_html, "csrftoken")?;
        let csrftoken = csrf_full.split(',').next().unwrap_or("").to_string();
        let (pubkey_body, _, _) =
            pubkey_result.map_err(|e| ZustError::LoginFailed(format!("获取 RSA 公钥失败: {e}")))?;
        let pubkey_json: serde_json::Value = serde_json::from_str(&pubkey_body)?;
        let modulus = pubkey_json["modulus"]
            .as_str()
            .ok_or_else(|| ZustError::LoginFailed("未找到 RSA modulus".into()))?;
        let exponent = pubkey_json["exponent"]
            .as_str()
            .ok_or_else(|| ZustError::LoginFailed("未找到 RSA exponent".into()))?;
        let rsa_pw = wisedu_encrypt_password(wisedu_password, modulus, exponent)?;
        let login_data = vec![
            ("csrftoken", csrftoken.as_str()),
            ("yhm", username),
            ("mm", rsa_pw.as_str()),
        ];
        let mut headers = ajax_headers(JW_LOGIN);
        headers.insert("Referer".into(), JW_LOGIN.into());
        headers.insert("Origin".into(), "https://newjwxt.zust.edu.cn".into());
        let (body, final_url, status) = session
            .post_follow_redirects(JW_LOGIN, &login_data, &headers, 12)
            .await
            .map_err(|e| ZustError::LoginFailed(format!("教务系统登录失败: {e}")))?;
        if final_url.contains("login_slogin") && !final_url.contains("index") {
            let message = RE_WISEDU_TIPS
                .captures(&body)
                .and_then(|capture| capture.get(1))
                .map(|value| value.as_str().trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| format!("教务系统登录失败 (status={status})"));
            return Err(ZustError::LoginFailed(message));
        }

        on_step(LoginStep::new(6, "验证 API 访问权限..."));
        let api_url = format!("{JW_BASE}/xsxxxggl/xsxxwh_cxCkDgxsxx.html?gnmkdm=N100801");
        let (api_body, _, _) = session
            .get_with_headers(&api_url, &ajax_headers(&final_url))
            .await
            .map_err(|e| ZustError::LoginFailed(format!("API 验证失败: {e}")))?;
        if api_body.contains("没有访问权限") {
            return Err(ZustError::LoginFailed("没有访问权限".into()));
        }
        let mut cookies = session.cookies_as_dict();
        let csrf_main = RE_CSRF
            .captures(&body)
            .and_then(|capture| capture.get(1))
            .map(|value| value.as_str().split(',').next().unwrap_or("").to_string())
            .unwrap_or(csrftoken);
        cookies.insert("_csrf".into(), csrf_main);
        cookies.insert("_main_url".into(), final_url);
        self.save_session(&cookies)?;
        self.clear_pending();
        Ok(AuthSuccess {
            cookies,
            username: username.into(),
            cas_password: cas_password.into(),
            wisedu_password: wisedu_password.into(),
        })
    }

    async fn ensure_browser_fingerprint(&self, session: &HttpSession, referer: &str) {
        let fingerprint = std::fs::read_to_string(self.fingerprint_path())
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                let value = format!(
                    "{:016X}{:016X}",
                    rand::random::<u64>(),
                    rand::random::<u64>()
                );
                if let Err(error) = std::fs::create_dir_all(&self.config_dir)
                    .and_then(|_| std::fs::write(self.fingerprint_path(), &value))
                {
                    log::warn!("无法保存浏览器指纹: {error}");
                }
                value
            });
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let url = format!(
            "{CAS_BASE}/bfp/info?bfp={}&_={timestamp}",
            url_escape(&fingerprint)
        );
        if let Err(error) = session.get_with_headers(&url, &ajax_headers(referer)).await {
            log::warn!("浏览器指纹登记失败，将继续登录: {error}");
        }
    }
}

async fn fetch_captcha(session: &HttpSession) -> Result<String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let url = format!("{CAS_BASE}/getCaptcha.htl?timestamp={timestamp}");
    let (bytes, _, _) = session
        .get_bytes_with_headers(&url, &HashMap::new())
        .await?;
    if bytes.is_empty() {
        return Err(ZustError::LoginFailed("验证码图片为空".into()));
    }
    let mime = if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        "image/png"
    };
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn capture_required(regex: &regex::Regex, html: &str, name: &str) -> Result<String> {
    regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().to_string())
        .ok_or_else(|| ZustError::LoginFailed(format!("未找到 {name}")))
}

fn parse_form_action(html: &str, page_url: &str) -> Result<String> {
    let raw_action = RE_FORM_ACTION
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| "/authserver/login".into());
    Url::parse(page_url)
        .map_err(|e| ZustError::LoginFailed(format!("CAS 页面地址异常: {e}")))?
        .join(&raw_action)
        .map(|url| url.to_string())
        .map_err(|e| ZustError::LoginFailed(format!("CAS 表单地址异常: {e}")))
}

fn captcha_required(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("isNeed").cloned())
        .and_then(|value| value.as_bool())
        .unwrap_or_else(|| body.contains("\"isNeed\":true") || body.contains("isNeed=true"))
}

fn response_message(value: &serde_json::Value) -> Option<String> {
    ["returnMessage", "message", "msg", "errorMessage"]
        .iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToOwned::to_owned)
}

fn cas_login_url(service: &str) -> Result<String> {
    let mut url = Url::parse(&format!("{CAS_BASE}/login"))
        .map_err(|e| ZustError::LoginFailed(format!("CAS 地址异常: {e}")))?;
    url.query_pairs_mut().append_pair("service", service);
    Ok(url.to_string())
}

fn with_service(action: &str, service: &str) -> Result<String> {
    let mut url =
        Url::parse(action).map_err(|e| ZustError::LoginFailed(format!("CAS 表单地址异常: {e}")))?;
    if !url.query_pairs().any(|(key, _)| key == "service") {
        url.query_pairs_mut().append_pair("service", service);
    }
    Ok(url.to_string())
}

fn url_escape(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_escape() {
        let escaped = url_escape("https://example.com/path?a=1&b=2");
        assert!(!escaped.contains("://"));
    }

    #[test]
    fn test_auth_manager_new() {
        let manager = AuthManager::new();
        assert!(manager.config_dir.ends_with("zust-grades"));
    }

    #[test]
    fn test_login_step() {
        let step = LoginStep::new(1, "测试");
        assert_eq!(step.step, 1);
        assert_eq!(step.message, "测试");
    }

    #[test]
    fn test_captcha_required() {
        assert!(captcha_required(r#"{"isNeed":true}"#));
        assert!(!captcha_required(r#"{"isNeed":false}"#));
    }

    #[test]
    fn test_with_service_adds_query_parameter() {
        let url = with_service("https://authserver.zust.edu.cn/authserver/login", CAS_SVC).unwrap();
        assert!(url.contains("service="));
        assert!(url.contains("newjwxt.zust.edu.cn"));
    }
}
