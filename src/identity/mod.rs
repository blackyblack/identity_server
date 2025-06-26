use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use crate::verify::nonce::{InMemoryNonceManager, NonceManager};

mod decay;
pub mod error;
pub mod forget;
pub mod idt;
pub mod proof;
pub mod punish;
mod tree_walk;
pub mod vouch;

pub type UserAddress = String;
pub type ProofId = u128;
pub type IdtAmount = u128;

#[derive(Default)]
struct VouchStorage {
    // key - vouchee, vouch object
    // value - (voucher, unix timestamp) map
    vouchers: HashMap<UserAddress, HashMap<UserAddress, u64>>,
    // key - voucher, vouch subject
    // value - (vouchee, unix timestamp) map
    vouchees: HashMap<UserAddress, HashMap<UserAddress, u64>>,
}

#[derive(Clone)]
pub struct ModeratorProof {
    pub moderator: UserAddress,
    pub amount: IdtAmount,
    pub proof_id: ProofId,
    pub timestamp: u64,
}

// key - proven user
// only single proof for a user is allowed. If proof should be updated,
// moderator should prove again and update proof manually.
type ProofStorage = HashMap<UserAddress, ModeratorProof>;

// system can generate own penalties, so proof_id and moderator are not required
#[derive(Clone, Default)]
pub struct SystemPenalty {
    pub amount: IdtAmount,
    pub timestamp: u64,
}

// key - punished user
#[derive(Default)]
struct PenaltyStorage {
    // only single ban for a user is allowed. If penalty should be increased,
    // moderator should ban again and increase penalty manually.
    moderator_penalty: HashMap<UserAddress, ModeratorProof>,
    // inside map key - forgotten user
    forget_penalties: HashMap<UserAddress, HashMap<UserAddress, SystemPenalty>>,
}

#[derive(Clone)]
pub struct IdentityService {
    vouches: Arc<RwLock<VouchStorage>>,
    proofs: Arc<RwLock<ProofStorage>>,
    penalties: Arc<RwLock<PenaltyStorage>>,
    nonce_manager: Arc<dyn NonceManager>,
}

impl IdentityService {
    pub fn new(nonce_manager: Arc<dyn NonceManager>) -> Self {
        Self {
            vouches: Arc::new(RwLock::new(VouchStorage::default())),
            proofs: Arc::new(RwLock::new(ProofStorage::default())),
            penalties: Arc::new(RwLock::new(PenaltyStorage::default())),
            nonce_manager,
        }
    }

    pub fn new_inmem() -> Self {
        Self::new(Arc::new(InMemoryNonceManager::default()))
    }

    pub fn nonce_manager(&self) -> Arc<dyn NonceManager> {
        self.nonce_manager.clone()
    }
}

impl Default for IdentityService {
    fn default() -> Self {
        Self::new_inmem()
    }
}

pub fn next_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Should be after the UNIX_EPOCH timestamp")
        .as_secs()
}

#[cfg(test)]
pub mod tests {
    use crate::identity::ProofId;

    pub const MODERATOR: &str = "moderator";
    pub const USER_A: &str = "userA";
    pub const PROOF_ID: ProofId = 1;
}
