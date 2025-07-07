use std::sync::Arc;

use crate::{
    admins::{AdminStorage, InMemoryAdminStorage},
    config::ExternalServersSection,
    identity::IdentityService,
    verify::nonce::{InMemoryNonceManager, NonceManager},
};

pub mod admins;
pub mod forget;
pub mod idt;
pub mod proof;
pub mod punish;
pub mod vouch;

#[derive(Clone)]
pub struct State {
    pub identity_service: IdentityService,
    pub admin_storage: Arc<dyn AdminStorage>,
    pub nonce_manager: Arc<dyn NonceManager>,
    pub external_servers: ExternalServersSection,
}

impl Default for State {
    fn default() -> Self {
        Self {
            identity_service: IdentityService::default(),
            admin_storage: Arc::new(InMemoryAdminStorage::default()),
            nonce_manager: Arc::new(InMemoryNonceManager::default()),
            external_servers: ExternalServersSection::default(),
        }
    }
}
