//! 应用状态管理

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use zust_core::auth::AuthManager;
use zust_core::client::JWClient;
use zust_core::sniper::EnrollSniper;

pub struct AppState {
    pub client: Arc<Mutex<Option<JWClient>>>,
    pub auth_manager: Mutex<AuthManager>,
    pub config_dir: PathBuf,
}

impl AppState {
    pub fn new() -> Self {
        let config_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            client: Arc::new(Mutex::new(None)),
            auth_manager: Mutex::new(AuthManager::new()),
            config_dir,
        }
    }
}

pub struct SniperState {
    pub sniper: Mutex<Option<Arc<EnrollSniper>>>,
    pub handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl SniperState {
    pub fn new() -> Self {
        Self {
            sniper: Mutex::new(None),
            handle: Mutex::new(None),
        }
    }
}
