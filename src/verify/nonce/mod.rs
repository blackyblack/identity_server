use std::collections::HashMap;

use async_std::sync::Mutex;
use async_trait::async_trait;

use crate::identity::UserAddress;
use crate::verify::nonce::error::Error;

pub mod db;
pub mod error;

pub type Nonce = u64;

// Manages signature nonces to prevent replay attacks
#[async_trait]
pub trait NonceManager: Send + Sync {
    async fn use_nonce(&self, user: &UserAddress, nonce: Nonce) -> Result<(), Error>;
    async fn next_nonce(&self, user: &UserAddress) -> Result<Nonce, Error>;
}

#[derive(Default)]
pub struct InMemoryNonceManager {
    used_nonce: Mutex<HashMap<UserAddress, Nonce>>,
}

#[async_trait]
impl NonceManager for InMemoryNonceManager {
    async fn use_nonce(&self, user: &UserAddress, nonce: Nonce) -> Result<(), Error> {
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

    async fn next_nonce(&self, user: &UserAddress) -> Result<Nonce, Error> {
        let used_nonce_lock = self.used_nonce.lock().await;
        used_nonce_lock
            .get(user)
            .copied()
            .unwrap_or_default()
            .checked_add(1)
            .ok_or(Error::NonceOverflowError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::random_keypair;

    #[async_std::test]
    async fn test_basic() {
        let (_priv, user) = random_keypair();
        let manager = InMemoryNonceManager::default();

        // next nonce should be 1
        assert_eq!(manager.next_nonce(&user).await.unwrap(), 1);
        // next nonce should still be 1 until we use it
        assert_eq!(manager.next_nonce(&user).await.unwrap(), 1);

        manager.use_nonce(&user, 1).await.unwrap();
        // next nonce should now be 2
        assert_eq!(manager.next_nonce(&user).await.unwrap(), 2);

        // using same nonce again should fail
        assert!(manager.use_nonce(&user, 1).await.is_err());
        // next nonce does not increment if use_nonce fails
        assert_eq!(manager.next_nonce(&user).await.unwrap(), 2);
    }
}
