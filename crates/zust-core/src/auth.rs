//! CAS + Wisedu 双步认证
//!
//! 对应 Python newjwxt/auth.py
//!
//! 登录流程:
//!   1. GET CAS 登录页 + 检测验证码（并行）
//!   2. 提取 pwdEncryptSalt, execution, form action
//!   3. AES 加密密码 → POST CAS 表单 → 获取 MOD_AUTH_CAS cookie
//!   4. GET Wisedu 登录页 + RSA 公钥（并行）
//!   5. RSA 加密密码 → POST Wisedu 登录 → 获取 JSESSIONID
//!   6. 验证 API 可用性 → 提取 _csrf token → 保存 session

use crate::crypto::{cas_encrypt_password, wisedu_encrypt_password};
use crate::error::{Result, ZustError};
use crate::session::HttpSession;
use crate::types::SessionData;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

// ── 常量 ──

const CAS_BASE: &str = "https://authserver.zust.edu.cn/authserver";
const CAS_SVC: &str =
    "https://newjwxt.zust.edu.cn/jwglxt/xtgl/login_slogin.html";
const JW_BASE: &str = "https://newjwxt.zust.edu.cn/jwglxt";
const JW_LOGIN: &str =
    "https://newjwxt.zust.edu.cn/jwglxt/xtgl/login_slogin.html";

// ── 预编译正则（编译一次，全局复用）──

static RE_PWD_SALT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"id="pwdEncryptSalt"\s+value="([^"]+)""#).unwrap()
});
static RE_EXECUTION: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"name="execution"\s+value="([^"]+)""#).unwrap()
});
static RE_FORM_ACTION: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"id="pwdFromId"\s+action="([^"]+)""#).unwrap()
});
static RE_CSRF: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"csrftoken[^>]*value="([^"]+)""#).unwrap()
});
static RE_CAS_ERROR: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"<span id="msg"[^>]*>([^<]+)</span>"#).unwrap()
});
static RE_WISEDU_TIPS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"<p id="tips"[^>]*>([^<]+)</p>"#).unwrap()
});

/// Ajax 请求的标准 headers
fn ajax_headers(referer: &str) -> HashMap<String, String> {
    let mut h = HashMap::new();
    h.insert("Accept".into(), "application/json, text/javascript, */*; q=0.01".into());
    h.insert("X-Requested-With".into(), "XMLHttpRequest".into());
    h.insert("Referer".into(), referer.into());
    h
}

/// 登录步骤事件（发给前端）
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

/// 认证管理器
pub struct AuthManager {
    pub config_dir: PathBuf,
}

impl AuthManager {
    pub fn new() -> Self {
        let config_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("zust-grades");
        Self { config_dir }
    }

    /// Session 文件路径
    fn session_path(&self) -> PathBuf {
        self.config_dir.join("newjwxt-session.json")
    }

    /// 加载缓存的 session cookies
    pub fn load_session(&self) -> Option<HashMap<String, String>> {
        let path = self.session_path();
        if !path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&path).ok()?;
        let data: SessionData = serde_json::from_str(&content).ok()?;
        Some(data.cookies)
    }

    /// 保存 session cookies 到文件
    pub fn save_session(&self, cookies: &HashMap<String, String>) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;

        let data = SessionData {
            cookies: cookies.clone(),
            saved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        };

        let json = serde_json::to_string_pretty(&data)?;
        std::fs::write(self.session_path(), json)?;
        Ok(())
    }

    /// 删除 session 文件（登出）
    pub fn clear_session(&self) -> Result<()> {
        let path = self.session_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// 完整登录流程
    pub async fn full_login(
        &self,
        username: &str,
        cas_password: &str,
        wisedu_password: &str,
        on_step: impl Fn(LoginStep),
    ) -> Result<HashMap<String, String>> {
        on_step(LoginStep::new(1, "连接 CAS 认证服务器..."));
        log::info!("Login step 1: Creating HTTP session, fetching CAS page + captcha check");

        let session = match HttpSession::new() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create HTTP session: {e}");
                return Err(ZustError::LoginFailed(format!("无法创建 HTTP 会话: {e}")));
            }
        };

        let cas_login_url = format!("{CAS_BASE}/login?service={}", url_escape(CAS_SVC));
        let captcha_url = format!(
            "{CAS_BASE}/checkNeedCaptcha.htl?username={}",
            url_escape(username)
        );

        // ═══ Step 1+2 并行: 获取 CAS 登录页 + 检查验证码 ═══
        let (page_result, captcha_result) = tokio::join!(
            session.get(&cas_login_url),
            async {
                let mut h = HashMap::new();
                h.insert("X-Requested-With".into(), "XMLHttpRequest".into());
                h.insert("Referer".into(), cas_login_url.clone());
                session.get_with_headers(&captcha_url, &h).await
            }
        );

        let (html, _url, _status) = match page_result {
            Ok(r) => {
                log::info!("CAS login page fetched, {} bytes", r.0.len());
                r
            }
            Err(e) => {
                log::error!("Failed to fetch CAS login page: {e}");
                return Err(ZustError::LoginFailed(format!("无法连接 CAS 服务器: {e}")));
            }
        };

        on_step(LoginStep::new(2, "解析 CAS 登录页..."));

        // 提取 pwdEncryptSalt
        let salt_key = RE_PWD_SALT
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| {
                log::error!("pwdEncryptSalt not found in CAS HTML");
                ZustError::LoginFailed("未找到 pwdEncryptSalt".into())
            })?;
        log::info!("Extracted salt_key: {}...", &salt_key[..4.min(salt_key.len())]);

        // 提取 execution
        let execution = RE_EXECUTION
            .captures(&html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| {
                log::error!("execution not found in CAS HTML");
                ZustError::LoginFailed("未找到 execution".into())
            })?;

        // 提取 form action URL
        let form_action = {
            let raw_action = RE_FORM_ACTION
                .captures(&html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| "/authserver/login".to_string());
            let base = url::Url::parse("https://authserver.zust.edu.cn/").unwrap();
            url::Url::options()
                .base_url(Some(&base))
                .parse(&raw_action)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| format!("https://authserver.zust.edu.cn{raw_action}"))
        };
        log::info!("Form action: {form_action}");

        // 解析 captcha 结果
        let captcha_body = match &captcha_result {
            Ok((body, _, _)) => body.clone(),
            Err(e) => {
                log::warn!("Captcha check failed (non-fatal): {e}");
                return Err(ZustError::LoginFailed(format!("验证码检测失败: {e}")));
            }
        };
        let has_captcha = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&captcha_body) {
            v.get("isNeed").and_then(|x| x.as_bool()).unwrap_or(false)
        } else {
            captcha_body.contains("true")
        };
        log::info!("Captcha required: {has_captcha}");

        on_step(LoginStep::new(3, "CAS 登录中..."));

        // ═══ Step 3: AES 加密密码 + 提交 CAS 表单 ═══
        let encrypted_pw = cas_encrypt_password(cas_password, &salt_key)?;

        let form_data: Vec<(&str, &str)> = vec![
            ("username", username),
            ("password", &encrypted_pw),
            ("execution", &execution),
            ("_eventId", "submit"),
            ("cllt", "userNameLogin"),
            ("dllt", "generalLogin"),
            ("service", CAS_SVC),
        ];
        let _ = has_captcha; // 验证码字段预留

        let mut cas_post_headers = ajax_headers(&cas_login_url);
        cas_post_headers.insert("Origin".into(), "https://authserver.zust.edu.cn".into());

        let (cas_result, _jw_url, cas_status) = match session
            .post_follow_redirects(&form_action, &form_data, &cas_post_headers, 10)
            .await
        {
            Ok(r) => {
                log::info!("CAS POST chain ended status={}, final_url={}", r.2, r.1);
                r
            }
            Err(e) => {
                log::error!("CAS POST failed: {e}");
                return Err(ZustError::LoginFailed(format!("CAS 提交失败: {e}")));
            }
        };

        let cas_cookies = session.cookies_as_dict();
        log::info!("Session cookies after CAS: {cas_cookies:?}");
        if !session.has_cookie("MOD_AUTH_CAS") {
            let err_msg = RE_CAS_ERROR
                .captures(&cas_result)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| format!("CAS 登录失败，请检查密码 (status={cas_status})"));
            log::error!("CAS login failed: {err_msg}");
            return Err(ZustError::LoginFailed(err_msg));
        }
        log::info!("CAS login successful! MOD_AUTH_CAS cookie obtained.");

        on_step(LoginStep::new(4, "Wisedu 登录中..."));

        // ═══ Step 4+5 并行: 获取 Wisedu 登录页 + RSA 公钥 ═══
        let pubkey_url = format!("{JW_BASE}/xtgl/login_getPublicKey.html");
        let mut jw_headers = HashMap::new();
        jw_headers.insert("Referer".into(), "https://newjwxt.zust.edu.cn/".into());
        let pubkey_headers = ajax_headers(JW_LOGIN);

        let (jw_result, pubkey_result) = tokio::join!(
            session.get_with_headers(JW_LOGIN, &jw_headers),
            session.get_with_headers(&pubkey_url, &pubkey_headers),
        );

        let (jw_html, _, _) = match jw_result {
            Ok(r) => {
                log::info!("Wisedu login page fetched, {} bytes", r.0.len());
                r
            }
            Err(e) => {
                log::error!("Failed to fetch Wisedu login page: {e}");
                return Err(ZustError::LoginFailed(format!("无法连接教务系统: {e}")));
            }
        };

        // 提取 csrftoken
        let csrf_full = RE_CSRF
            .captures(&jw_html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| {
                log::error!("csrftoken not found in Wisedu HTML");
                ZustError::LoginFailed("未找到 csrftoken".into())
            })?;
        let csrftoken = csrf_full.split(',').next().unwrap_or("").to_string();
        log::info!("Extracted csrftoken: {}...", &csrftoken[..8.min(csrftoken.len())]);

        // 解析 RSA 公钥
        let (pubkey_body, _, _) = match pubkey_result {
            Ok(r) => {
                log::info!("Public key fetched");
                r
            }
            Err(e) => {
                log::error!("Failed to fetch RSA public key: {e}");
                return Err(ZustError::LoginFailed(format!("获取 RSA 公钥失败: {e}")));
            }
        };

        let pubkey_json: serde_json::Value = serde_json::from_str(&pubkey_body)?;
        let modulus = pubkey_json["modulus"]
            .as_str()
            .ok_or_else(|| ZustError::LoginFailed("未找到 RSA modulus".into()))?;
        let exponent = pubkey_json["exponent"]
            .as_str()
            .ok_or_else(|| ZustError::LoginFailed("未找到 RSA exponent".into()))?;

        // ═══ Step 5: RSA 加密密码 + POST Wisedu 登录 ═══
        let rsa_pw = wisedu_encrypt_password(wisedu_password, modulus, exponent)?;

        let login_data: Vec<(&str, &str)> = vec![
            ("csrftoken", &csrftoken),
            ("yhm", username),
            ("mm", &rsa_pw),
        ];

        let mut wisedu_login_headers = ajax_headers(JW_LOGIN);
        wisedu_login_headers.insert("Referer".into(), JW_LOGIN.into());
        wisedu_login_headers.insert("Origin".into(), "https://newjwxt.zust.edu.cn".into());

        let (_body, final_url, wisedu_status) = match session
            .post_follow_redirects(JW_LOGIN, &login_data, &wisedu_login_headers, 10)
            .await
        {
            Ok(r) => {
                log::info!("Wisedu POST returned status={}, final_url={}", r.2, r.1);
                r
            }
            Err(e) => {
                log::error!("Wisedu POST failed: {e}");
                return Err(ZustError::LoginFailed(format!("教务系统登录失败: {e}")));
            }
        };

        if final_url.contains("login_slogin") && !final_url.contains("index") {
            let err_msg = RE_WISEDU_TIPS
                .captures(&_body)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_else(|| format!("教务系统登录失败 (status={wisedu_status})"));
            log::error!("Wisedu login failed: {err_msg}");
            return Err(ZustError::LoginFailed(err_msg));
        }
        log::info!("Wisedu login successful!");

        on_step(LoginStep::new(5, "验证 API 访问权限..."));

        // ═══ Step 6: API 验证 ═══
        let api_url =
            format!("{JW_BASE}/xsxxxggl/xsxxwh_cxCkDgxsxx.html?gnmkdm=N100801");
        let (api_body, _, _) = match session
            .get_with_headers(&api_url, &ajax_headers(&final_url))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                log::error!("API access check failed: {e}");
                return Err(ZustError::LoginFailed(format!("API 验证失败: {e}")));
            }
        };

        if api_body.contains("没有访问权限") {
            log::error!("API reports no access permission");
            return Err(ZustError::LoginFailed("没有访问权限".into()));
        }

        // ═══ 保存所有 session cookies ═══
        let mut cookies = session.cookies_as_dict();
        let csrf_main = RE_CSRF
            .captures(&_body)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().split(',').next().unwrap_or("").to_string())
            .unwrap_or(csrftoken);
        cookies.insert("_csrf".to_string(), csrf_main);
        cookies.insert("_main_url".to_string(), final_url.clone());

        on_step(LoginStep::new(6, "保存会话..."));
        log::info!(
            "Login complete, saving {} cookies: {:?}",
            cookies.len(),
            cookies.keys().collect::<Vec<_>>()
        );

        self.save_session(&cookies)?;

        Ok(cookies)
    }
}

/// URL 编码辅助函数
fn url_escape(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
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
        let mgr = AuthManager::new();
        assert!(mgr.config_dir.ends_with("zust-grades"));
    }

    #[test]
    fn test_login_step() {
        let step = LoginStep::new(1, "测试");
        assert_eq!(step.step, 1);
        assert_eq!(step.message, "测试");
    }
}
