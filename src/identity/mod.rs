use std::{sync::Arc, time::SystemTime};

use crate::identity::storage::{
    InMemoryPenaltyStorage, InMemoryProofStorage, InMemoryVouchStorage, PenaltyStorage,
    ProofStorage, VouchStorage,
};

mod decay;
pub mod error;
pub mod forget;
pub mod genesis;
pub mod idt;
pub mod proof;
pub mod punish;
pub mod storage;
pub mod storage_db;
mod tree_walk;
pub mod vouch;

pub type UserAddress = String;
pub type ProofId = u64;
pub type IdtAmount = u64;

#[derive(Clone)]
pub struct ModeratorProof {
    pub moderator: UserAddress,
    pub amount: IdtAmount,
    pub proof_id: ProofId,
    pub timestamp: u64,
}

// system can generate own penalties, so proof_id and moderator are not required
#[derive(Clone, Default)]
pub struct SystemPenalty {
    pub amount: IdtAmount,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct IdentityService {
    vouches: Arc<dyn VouchStorage>,
    proofs: Arc<dyn ProofStorage>,
    penalties: Arc<dyn PenaltyStorage>,
}

impl Default for IdentityService {
    fn default() -> Self {
        Self {
            vouches: Arc::new(InMemoryVouchStorage::default()),
            proofs: Arc::new(InMemoryProofStorage::default()),
            penalties: Arc::new(InMemoryPenaltyStorage::default()),
        }
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
