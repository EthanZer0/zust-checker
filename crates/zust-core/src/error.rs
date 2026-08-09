//! 统一错误类型

use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
pub enum ZustError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Login failed: {0}")]
    LoginFailed(String),

    #[error("Session expired")]
    SessionExpired,

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("API mismatch: {0}")]
    ApiMismatch(String),

    #[error("Sniper not running")]
    SniperNotRunning,

    #[error("Base64 decode error: {0}")]
    Base64(String),
}

impl From<reqwest::Error> for ZustError {
    fn from(e: reqwest::Error) -> Self {
        use std::error::Error;
        // Include the full source chain for debugging
        let mut msg = e.to_string();
        let mut source: Option<&dyn Error> = e.source();
        while let Some(s) = source {
            msg.push_str(" | caused by: ");
            msg.push_str(&s.to_string());
            source = s.source();
        }
        // Also include URL if available
        if let Some(url) = e.url() {
            msg.push_str(&format!(" | url: {url}"));
        }
        ZustError::Http(msg)
    }
}

impl From<serde_json::Error> for ZustError {
    fn from(e: serde_json::Error) -> Self {
        ZustError::Json(e.to_string())
    }
}

impl From<std::io::Error> for ZustError {
    fn from(e: std::io::Error) -> Self {
        ZustError::Io(e.to_string())
    }
}

impl From<base64::DecodeError> for ZustError {
    fn from(e: base64::DecodeError) -> Self {
        ZustError::Base64(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ZustError>;
