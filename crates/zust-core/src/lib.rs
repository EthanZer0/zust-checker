//! zust-core — ZUST 教务系统核心库
//!
//! 模块:
//!   error.rs   — 统一错误类型 (ZustError, Result)
//!   types.rs   — 领域类型 (UserInfo, GradeEntry, SniperTarget, ...)
//!   crypto.rs  — AES-CBC + RSA PKCS#1 v1.5 加密
//!   session.rs — reqwest HTTP 会话管理
//!   auth.rs    — CAS + Wisedu 双步认证
//!   client.rs  — JWClient API 封装
//!   sniper.rs  — EnrollSniper 抢课引擎

pub mod auth;
pub mod client;
pub mod crypto;
pub mod error;
pub mod session;
pub mod sniper;
pub mod types;

// 重导出常用类型
pub use auth::AuthManager;
pub use client::JWClient;
pub use error::{Result, ZustError};
pub use session::HttpSession;
pub use sniper::EnrollSniper;
pub use types::*;
