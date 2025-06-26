use std::collections::HashMap;
use std::sync::Mutex;

use crate::identity::UserAddress;

// Manages signature nonces to prevent replay attacks
pub trait NonceManager: Send + Sync {
    fn use_nonce(&self, user: &UserAddress, nonce: u64) -> bool;
    fn next_nonce(&self, user: &UserAddress) -> u64;
}

#[derive(Default)]
pub struct InMemoryNonceManager {
    used_nonce: Mutex<HashMap<UserAddress, u64>>,
    next_nonces: Mutex<HashMap<UserAddress, u64>>,
}

impl NonceManager for InMemoryNonceManager {
    fn use_nonce(&self, user: &UserAddress, nonce: u64) -> bool {
        let mut used_nonce_lock = self.used_nonce.lock().expect("Should acquire lock");
        let last_nonce = used_nonce_lock.entry(user.clone()).or_default();

        // if nonce is already used
        if *last_nonce >= nonce {
            return false;
        }

        // Otherwise, mark as used
        *last_nonce = nonce;
        true
    }

    fn next_nonce(&self, user: &UserAddress) -> u64 {
        let mut next_nonces_lock = self.next_nonces.lock().expect("Should acquire lock");
        let next_nonce = next_nonces_lock.entry(user.clone()).or_insert(0);
        *next_nonce += 1;
        *next_nonce
    }
}
