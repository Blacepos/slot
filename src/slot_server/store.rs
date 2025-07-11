use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Instant};

use slot_client::protocol::ValidName;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: ValidName,
    pub http_addr: SocketAddr,
    pub slot_addr: SocketAddr,
    pub time_last_heard: Instant,
}

pub struct ModuleStore {
    modules: Arc<RwLock<Vec<ModuleInfo>>>,
}

impl Clone for ModuleStore {
    fn clone(&self) -> Self {
        Self {
            modules: self.modules.clone(),
        }
    }
}

impl ModuleStore {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(RwLock::new(Vec::with_capacity(8))),
        }
    }

    pub async fn store_module(
        &self,
        name: &ValidName,
        http_addr: &SocketAddr,
        slot_addr: &SocketAddr,
    ) {
        self.modules.write().await.push(ModuleInfo {
            name: name.clone(),
            http_addr: *http_addr,
            slot_addr: *slot_addr,
            time_last_heard: Instant::now(),
            // time_last_pinged: Instant::now(),
        });
    }

    pub async fn find_module_by_name(&self, name: &str) -> Option<ModuleInfo> {
        let validated = ValidName::from_str(name).ok()?;
        self.modules
            .read()
            .await
            .iter()
            .find(|e| e.name == validated)
            .cloned()
    }

    pub async fn update_last_heard(&self, addr: &SocketAddr) {
        if let Some(module_info) = self
            .modules
            .write()
            .await
            .iter_mut()
            .find(|e| &e.slot_addr == addr)
        {
            log::debug!(
                "Received heartbeat response from module \"{}\"",
                module_info.name
            );
            module_info.time_last_heard = Instant::now();
        }
    }

    pub async fn get_vec(
        &self,
    ) -> tokio::sync::RwLockWriteGuard<'_, Vec<ModuleInfo>> {
        self.modules.write().await
    }
}
