use std::sync::Arc;

use serde_json::json;
use tide::{Response, http::mime};

use crate::{
    admins::{AdminStorage, InMemoryAdminStorage},
    identity::{IdentityService, UserAddress},
    servers::storage::{InMemoryServerStorage, ServerStorage},
    verify::{
        nonce::{InMemoryNonceManager, Nonce, NonceManager},
        verify_message,
    },
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

pub async fn verify_admin_action(
    state: &State,
    sender: &UserAddress,
    signature: String,
    nonce: Nonce,
    message_prefix: &str,
) -> Result<(), Response> {
    if state.admin_storage.check_admin(sender).await.is_err() {
        return Err(Response::builder(403)
            .body(json!({"error": "not admin"}))
            .content_type(mime::JSON)
            .build());
    }

    if verify_message(
        signature,
        sender,
        nonce,
        message_prefix,
        &*state.nonce_manager,
    )
    .await
    .is_err()
    {
        return Err(Response::builder(400)
            .body(json!({"error": "signature verification failed"}))
            .content_type(mime::JSON)
            .build());
    }

    Ok(())
}
