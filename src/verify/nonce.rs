use std::collections::HashMap;

use async_std::sync::Mutex;
use async_trait::async_trait;

use crate::identity::UserAddress;
use crate::verify::error::Error;

// Manages signature nonces to prevent replay attacks
#[async_trait]
pub trait NonceManager: Send + Sync {
    async fn use_nonce(&self, user: &UserAddress, nonce: u64) -> Result<(), Error>;
    async fn next_nonce(&self, user: &UserAddress) -> Result<u64, Error>;
}

#[derive(Default)]
pub struct InMemoryNonceManager {
    used_nonce: Mutex<HashMap<UserAddress, u64>>,
}

#[async_trait]
impl NonceManager for InMemoryNonceManager {
    async fn use_nonce(&self, user: &UserAddress, nonce: u64) -> Result<(), Error> {
        let mut used_nonce_lock = self.used_nonce.lock().await;
        let last_nonce = used_nonce_lock.entry(user.clone()).or_default();

        // if nonce is already used
        if *last_nonce >= nonce {
            return Err(Error::NonceUsedError(nonce));
        }

        // Otherwise, mark as used
        *last_nonce = nonce;
        Ok(())
    }

    async fn next_nonce(&self, user: &UserAddress) -> Result<u64, Error> {
        let used_nonce_lock = self.used_nonce.lock().await;
        used_nonce_lock
            .get(user)
            .copied()
            .unwrap_or_default()
            .checked_add(1)
            .ok_or(Error::NonceOverflowError)
    }
}
