use std::sync::Arc;

use crate::{
    admins::{AdminStorage, InMemoryAdminStorage},
    identity::IdentityService,
    servers::storage::{InMemoryServerStorage, ServerStorage},
    verify::nonce::{InMemoryNonceManager, NonceManager},
};

pub mod admins;
pub mod forget;
pub mod idt;
pub mod proof;
pub mod punish;
pub mod servers;
pub mod vouch;

#[derive(Clone)]
pub struct State {
    pub identity_service: IdentityService,
    pub admin_storage: Arc<dyn AdminStorage>,
    pub nonce_manager: Arc<dyn NonceManager>,
    pub server_storage: Arc<dyn ServerStorage>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            identity_service: IdentityService::default(),
            admin_storage: Arc::new(InMemoryAdminStorage::default()),
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
            server_storage: Arc::new(InMemoryServerStorage::default()),
        }
    }
}
