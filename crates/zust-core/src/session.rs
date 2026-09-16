//! HTTP 会话管理
//!
//! 对应 Python newjwxt/session.py
//! reqwest 内置 cookie jar 处理重定向链（CAS SSO 关键），
//! 同时手动从响应头收集 Set-Cookie 以便持久化保存和验证。

use crate::error::{Result, ZustError};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use url::Url;

/// User-Agent（与 Python 版本完全一致）
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36";

/// HTTP 会话
///
/// `cookie_store(true)` — reqwest 内置 jar 自动处理重定向链的 cookie 传递。
/// `manual_cookies` — 从 Set-Cookie 收集，用于持久化和登录验证。
/// 使用 Arc<RwLock<>> 以满足 Tauri 的 Send + Sync 要求。
#[derive(Clone)]
pub struct HttpSession {
    client: reqwest::Client,
    manual_cookies: Arc<RwLock<HashMap<String, String>>>,
}

impl HttpSession {
    /// 创建新的 HTTP 会话
    pub fn new() -> Result<Self> {
        let client = Self::build_client()?;
        Ok(Self {
            client,
            manual_cookies: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 从 cookie 字典恢复会话（预填 manual_cookies）
    pub fn from_cookies(cookies: &HashMap<String, String>) -> Result<Self> {
        let client = Self::build_client()?;
        Ok(Self {
            client,
            manual_cookies: Arc::new(RwLock::new(cookies.clone())),
        })
    }

    fn build_client() -> std::result::Result<reqwest::Client, ZustError> {
        log::info!("Building reqwest client (cookie_store=true, NO redirect follow, connect_timeout=10s, timeout=30s)...");
        match reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .user_agent(USER_AGENT)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none()) // 手动处理 CAS/Wisedu 重定向
            .gzip(true)
            .brotli(true)
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
        {
            Ok(client) => {
                log::info!("reqwest client built successfully");
                Ok(client)
            }
            Err(e) => {
                let msg = format!(
                    "Client build failed: {e} (is_builder={}, is_connect={}, is_timeout={})",
                    e.is_builder(),
                    e.is_connect(),
                    e.is_timeout()
                );
                log::error!("{msg}");
                Err(ZustError::Http(msg))
            }
        }
    }

    /// 从响应 headers 中提取 Set-Cookie 到 manual_cookies
    fn collect_set_cookies(&self, headers: &reqwest::header::HeaderMap) {
        let mut count = 0usize;
        let mut cookies = self.manual_cookies.write().unwrap();
        for cookie_str in headers.get_all(reqwest::header::SET_COOKIE) {
            if let Ok(cs) = cookie_str.to_str() {
                for part in cs.split(';') {
                    let part = part.trim();
                    if let Some(eq) = part.find('=') {
                        let key = part[..eq].trim().to_string();
                        let val = part[eq + 1..].trim().to_string();
                        if !key.is_empty()
                            && !key.eq_ignore_ascii_case("path")
                            && !key.eq_ignore_ascii_case("domain")
                            && !key.eq_ignore_ascii_case("expires")
                            && !key.eq_ignore_ascii_case("max-age")
                            && !key.eq_ignore_ascii_case("secure")
                            && !key.eq_ignore_ascii_case("httponly")
                            && !key.eq_ignore_ascii_case("samesite")
                        {
                            cookies.insert(key, val);
                            count += 1;
                            break;
                        }
                    }
                    break;
                }
            }
        }
        if count > 0 {
            log::info!("Collected {count} new manual cookie(s)");
        }
    }

    fn cookie_header(&self) -> Option<String> {
        let cookies = self.manual_cookies.read().unwrap();
        if cookies.is_empty() {
            None
        } else {
            Some(
                cookies
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        }
    }

    pub fn merge_cookies(&self, new_cookies: &HashMap<String, String>) {
        let mut cookies = self.manual_cookies.write().unwrap();
        for (k, v) in new_cookies {
            cookies.insert(k.clone(), v.clone());
        }
    }

    pub fn set_cookie(&self, key: String, value: String) {
        self.manual_cookies.write().unwrap().insert(key, value);
    }

    pub fn cookies_as_dict(&self) -> HashMap<String, String> {
        self.manual_cookies.read().unwrap().clone()
    }

    pub fn has_cookie(&self, name: &str) -> bool {
        self.manual_cookies.read().unwrap().contains_key(name)
    }

    // ── HTTP methods ──

    async fn extract(resp: reqwest::Response) -> Result<(String, String, u16)> {
        let final_url = resp.url().to_string();
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        Ok((body, final_url, status))
    }

    async fn extract_bytes(resp: reqwest::Response) -> Result<(Vec<u8>, String, u16)> {
        let final_url = resp.url().to_string();
        let status = resp.status().as_u16();
        let body = resp.bytes().await?.to_vec();
        Ok((body, final_url, status))
    }

    pub async fn get(&self, url: &str) -> Result<(String, String, u16)> {
        self.get_with_headers(url, &HashMap::new()).await
    }

    pub async fn get_with_headers(
        &self,
        url: &str,
        extra_headers: &HashMap<String, String>,
    ) -> Result<(String, String, u16)> {
        let mut req = self.client.get(url);
        if let Some(cv) = self.cookie_header() {
            req = req.header("Cookie", cv);
        }
        for (k, v) in extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await?;
        let headers = resp.headers().clone();
        let result = Self::extract(resp).await?;
        self.collect_set_cookies(&headers);
        Ok(result)
    }

    /// GET with raw bytes response（用于验证码等二进制数据）
    pub async fn get_bytes_with_headers(
        &self,
        url: &str,
        extra_headers: &HashMap<String, String>,
    ) -> Result<(Vec<u8>, String, u16)> {
        let mut req = self.client.get(url);
        if let Some(cv) = self.cookie_header() {
            req = req.header("Cookie", cv);
        }
        for (k, v) in extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await?;
        let headers = resp.headers().clone();
        let result = Self::extract_bytes(resp).await?;
        self.collect_set_cookies(&headers);
        Ok(result)
    }

    pub async fn post(&self, url: &str, data: &[(&str, &str)]) -> Result<(String, String, u16)> {
        self.post_with_headers(url, data, &HashMap::new()).await
    }

    pub async fn post_with_headers(
        &self,
        url: &str,
        data: &[(&str, &str)],
        extra_headers: &HashMap<String, String>,
    ) -> Result<(String, String, u16)> {
        let mut req = self
            .client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded");

        if let Some(cv) = self.cookie_header() {
            req = req.header("Cookie", cv);
        }
        req = req.form(data);
        for (k, v) in extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await?;
        let headers = resp.headers().clone();
        let result = Self::extract(resp).await?;
        self.collect_set_cookies(&headers);
        Ok(result)
    }

    pub async fn post_json(
        &self,
        url: &str,
        json: &serde_json::Value,
        extra_headers: &HashMap<String, String>,
    ) -> Result<(String, String, u16)> {
        let mut req = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(json);

        if let Some(cv) = self.cookie_header() {
            req = req.header("Cookie", cv);
        }
        for (k, v) in extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await?;
        let headers = resp.headers().clone();
        let result = Self::extract(resp).await?;
        self.collect_set_cookies(&headers);
        Ok(result)
    }

    pub fn client_ref(&self) -> &reqwest::Client {
        &self.client
    }

    /// Resolve a redirect Location against the URL that produced it.
    ///
    /// CAS deployments do not always return absolute redirect URLs. Browser
    /// clients resolve relative Locations automatically, so the native client
    /// must do the same before issuing the next request.
    fn resolve_redirect(base_url: &str, location: &str) -> Result<String> {
        if let Ok(absolute) = Url::parse(location) {
            return Ok(absolute.to_string());
        }

        let base = Url::parse(base_url)
            .map_err(|e| ZustError::Http(format!("Invalid redirect base URL: {e}")))?;
        base.join(location)
            .map(|url| url.to_string())
            .map_err(|e| ZustError::Http(format!("Invalid redirect Location: {e}")))
    }

    /// POST raw body with a custom Content-Type（用于特殊场景的原始请求体）
    pub async fn post_raw(
        &self,
        url: &str,
        body: &str,
        content_type: &str,
        extra_headers: &HashMap<String, String>,
    ) -> Result<(String, String, u16)> {
        let mut req = self
            .client
            .post(url)
            .header("Content-Type", content_type)
            .body(body.to_string());

        if let Some(cv) = self.cookie_header() {
            req = req.header("Cookie", cv);
        }
        for (k, v) in extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }
        let resp = req.send().await?;
        let headers = resp.headers().clone();
        let result = Self::extract(resp).await?;
        self.collect_set_cookies(&headers);
        Ok(result)
    }

    /// 手动跟随重定向的 GET（用于 CAS SSO 重定向链）
    ///
    /// 与 `post_follow_redirects` 对应，但用于 GET 请求。
    /// 手动处理 302 重定向链，每跳收集 Set-Cookie。
    pub async fn get_follow_redirects(
        &self,
        url: &str,
        extra_headers: &HashMap<String, String>,
        max_hops: usize,
    ) -> Result<(String, String, u16)> {
        let mut current_url = url.to_string();
        let mut current_status: u16 = 0;
        let mut current_body = String::new();

        for hop in 0..=max_hops {
            let mut req = self.client.get(&current_url);
            if let Some(cv) = self.cookie_header() {
                req = req.header("Cookie", cv);
            }
            for (k, v) in extra_headers {
                req = req.header(k.as_str(), v.as_str());
            }

            let resp = req.send().await?;
            let headers = resp.headers().clone();
            current_status = resp.status().as_u16();
            current_body = resp.text().await?;
            self.collect_set_cookies(&headers);

            log::info!(
                "GET redirect hop {}: status={}, url={}",
                hop,
                current_status,
                current_url
            );

            if current_status == 301 || current_status == 302 || current_status == 303 {
                if let Some(loc) = headers.get(reqwest::header::LOCATION) {
                    let location = loc.to_str().unwrap_or_default();
                    current_url = Self::resolve_redirect(&current_url, location)?;
                    continue;
                }
            }
            break;
        }

        Ok((current_body, current_url, current_status))
    }

    /// 手动跟随重定向的 POST（用于 CAS SSO）
    ///
    /// CAS 登录流程中有多个 302 重定向，reqwest 的自动重定向容易造成循环。
    /// 此方法手动处理：POST → 获得 302 → 提取 Location → GET → 循环直到 200。
    /// 中间所有 cookie 自动被 cookie_store 收集。
    pub async fn post_follow_redirects(
        &self,
        url: &str,
        data: &[(&str, &str)],
        extra_headers: &HashMap<String, String>,
        max_hops: usize,
    ) -> Result<(String, String, u16)> {
        // Step 1: POST
        let mut req = self
            .client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded");

        if let Some(cv) = self.cookie_header() {
            req = req.header("Cookie", cv);
        }
        req = req.form(data);
        for (k, v) in extra_headers {
            req = req.header(k.as_str(), v.as_str());
        }

        let resp = req.send().await?;
        let headers = resp.headers().clone();
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        self.collect_set_cookies(&headers);
        let mut current_url = if let Some(loc) = headers.get(reqwest::header::LOCATION) {
            let location = loc.to_str().unwrap_or_default();
            Self::resolve_redirect(url, location)?
        } else {
            url.to_string()
        };
        let mut current_body = body;
        let mut current_status = status;
        log::info!("POST to {url} → status={status}, location={current_url}");

        // Step 2: Follow GET redirects
        for hop in 0..max_hops {
            if current_status != 301 && current_status != 302 && current_status != 303 {
                break;
            }
            log::info!("  Redirect hop {}: GET {current_url}", hop + 1);

            let mut get_req = self.client.get(&current_url);
            if let Some(cv) = self.cookie_header() {
                get_req = get_req.header("Cookie", cv);
            }
            let get_resp = get_req.send().await?;
            let get_headers = get_resp.headers().clone();
            current_status = get_resp.status().as_u16();
            current_body = get_resp.text().await?;
            self.collect_set_cookies(&get_headers);

            if let Some(loc) = get_headers.get(reqwest::header::LOCATION) {
                let location = loc.to_str().unwrap_or_default();
                current_url = Self::resolve_redirect(&current_url, location)?;
            }
            log::info!("  → status={current_status}, location={current_url}");
        }

        Ok((current_body, current_url.clone(), current_status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = HttpSession::new();
        assert!(session.is_ok());
    }

    #[test]
    fn test_session_from_cookies() {
        let mut cookies = HashMap::new();
        cookies.insert("test".to_string(), "value".to_string());
        let session = HttpSession::from_cookies(&cookies);
        assert!(session.is_ok());
        assert_eq!(
            session.unwrap().manual_cookies.read().unwrap().get("test"),
            Some(&"value".to_string())
        );
    }

    #[test]
    fn test_cookie_header() {
        let session = HttpSession {
            client: HttpSession::new().unwrap().client,
            manual_cookies: Arc::new(RwLock::new({
                let mut m = HashMap::new();
                m.insert("a".into(), "1".into());
                m.insert("b".into(), "2".into());
                m
            })),
        };
        let header = session.cookie_header().unwrap();
        assert!(header.contains("a=1"));
        assert!(header.contains("b=2"));
        assert!(header.contains("; "));
    }

    #[test]
    fn test_has_cookie() {
        let mut cookies = HashMap::new();
        cookies.insert("MOD_AUTH_CAS".to_string(), "xxx".to_string());
        let session = HttpSession::from_cookies(&cookies).unwrap();
        assert!(session.has_cookie("MOD_AUTH_CAS"));
        assert!(!session.has_cookie("nonexistent"));
    }

    #[test]
    fn test_resolve_relative_redirect() {
        let resolved = HttpSession::resolve_redirect(
            "https://authserver.zust.edu.cn/authserver/login?service=x",
            "/authserver/reAuthCheck/reAuthLoginView.do?isMultifactor=true",
        )
        .unwrap();
        assert_eq!(
            resolved,
            "https://authserver.zust.edu.cn/authserver/reAuthCheck/reAuthLoginView.do?isMultifactor=true"
        );
    }
}
