use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use super::{ModeratorProof, SystemPenalty, UserAddress};

pub trait VouchStorage: Send + Sync {
    fn vouch(&self, from: UserAddress, to: UserAddress, timestamp: u64);
    fn vouchers_with_time(&self, user: &UserAddress) -> HashMap<UserAddress, u64>;
    fn vouchees_with_time(&self, user: &UserAddress) -> HashMap<UserAddress, u64>;
    fn remove_vouch(&self, voucher: UserAddress, vouchee: UserAddress);
}

#[derive(Default)]
struct VouchData {
    vouchers: HashMap<UserAddress, HashMap<UserAddress, u64>>,
    vouchees: HashMap<UserAddress, HashMap<UserAddress, u64>>,
}

#[derive(Default)]
pub struct InMemoryVouchStorage {
    data: RwLock<VouchData>,
}

impl VouchStorage for InMemoryVouchStorage {
    fn vouch(&self, from: UserAddress, to: UserAddress, timestamp: u64) {
        let mut lock = self.data.write().expect("Poisoned RwLock detected");
        lock.vouchers
            .entry(to.clone())
            .and_modify(|v| {
                v.insert(from.clone(), timestamp);
            })
            .or_insert_with(|| {
                let mut m = HashMap::new();
                m.insert(from.clone(), timestamp);
                m
            });
        lock.vouchees
            .entry(from)
            .and_modify(|v| {
                v.insert(to.clone(), timestamp);
            })
            .or_insert_with(|| {
                let mut m = HashMap::new();
                m.insert(to, timestamp);
                m
            });
    }

    fn vouchers_with_time(&self, user: &UserAddress) -> HashMap<UserAddress, u64> {
        self.data
            .read()
            .expect("Poisoned RwLock detected")
            .vouchers
            .get(user)
            .cloned()
            .unwrap_or_default()
    }

    fn vouchees_with_time(&self, user: &UserAddress) -> HashMap<UserAddress, u64> {
        self.data
            .read()
            .expect("Poisoned RwLock detected")
            .vouchees
            .get(user)
            .cloned()
            .unwrap_or_default()
    }

    fn remove_vouch(&self, voucher: UserAddress, vouchee: UserAddress) {
        let mut lock = self.data.write().expect("Poisoned RwLock detected");
        lock.vouchers.entry(vouchee.clone()).and_modify(|v| {
            v.remove(&voucher);
        });
        lock.vouchees.entry(voucher).and_modify(|v| {
            v.remove(&vouchee);
        });
    }
}

pub trait ProofStorage: Send + Sync {
    fn set_proof(&self, user: UserAddress, proof: ModeratorProof);
    fn proof(&self, user: &UserAddress) -> Option<ModeratorProof>;
}

#[derive(Default)]
pub struct InMemoryProofStorage {
    data: RwLock<HashMap<UserAddress, ModeratorProof>>,
}

impl ProofStorage for InMemoryProofStorage {
    fn set_proof(&self, user: UserAddress, proof: ModeratorProof) {
        self.data
            .write()
            .expect("Poisoned RwLock detected")
            .insert(user, proof);
    }

    fn proof(&self, user: &UserAddress) -> Option<ModeratorProof> {
        self.data
            .read()
            .expect("Poisoned RwLock detected")
            .get(user)
            .cloned()
    }
}

pub trait PenaltyStorage: Send + Sync {
    fn insert_moderator_penalty(&self, user: UserAddress, proof: ModeratorProof);
    fn insert_forgotten_penalty(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        penalty: SystemPenalty,
    );
    fn remove_forgotten(&self, user: UserAddress, forgotten: &UserAddress);
    fn moderator_penalty(&self, user: &UserAddress) -> Option<ModeratorProof>;
    fn forgotten_penalty(
        &self,
        user: &UserAddress,
        forgotten: &UserAddress,
    ) -> Option<SystemPenalty>;
    fn forgotten_users(&self, user: &UserAddress) -> HashSet<UserAddress>;
}

#[derive(Default)]
struct PenaltyData {
    moderator_penalty: HashMap<UserAddress, ModeratorProof>,
    forget_penalties: HashMap<UserAddress, HashMap<UserAddress, SystemPenalty>>,
}

#[derive(Default)]
pub struct InMemoryPenaltyStorage {
    data: RwLock<PenaltyData>,
}

impl PenaltyStorage for InMemoryPenaltyStorage {
    fn insert_moderator_penalty(&self, user: UserAddress, proof: ModeratorProof) {
        self.data
            .write()
            .expect("Poisoned RwLock detected")
            .moderator_penalty
            .insert(user, proof);
    }

    fn insert_forgotten_penalty(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        penalty: SystemPenalty,
    ) {
        self.data
            .write()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .entry(user)
            .and_modify(|v| {
                v.insert(vouchee.clone(), penalty.clone());
            })
            .or_insert_with(move || HashMap::from([(vouchee, penalty)]));
    }

    fn remove_forgotten(&self, user: UserAddress, forgotten: &UserAddress) {
        self.data
            .write()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .entry(user)
            .and_modify(|v| {
                v.remove(forgotten);
            });
    }

    fn moderator_penalty(&self, user: &UserAddress) -> Option<ModeratorProof> {
        self.data
            .read()
            .expect("Poisoned RwLock detected")
            .moderator_penalty
            .get(user)
            .cloned()
    }

    fn forgotten_penalty(
        &self,
        user: &UserAddress,
        forgotten: &UserAddress,
    ) -> Option<SystemPenalty> {
        self.data
            .read()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .get(user)
            .and_then(|v| v.get(forgotten).cloned())
    }

    fn forgotten_users(&self, user: &UserAddress) -> HashSet<UserAddress> {
        self.data
            .read()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .get(user)
            .cloned()
            .unwrap_or_default()
            .into_keys()
            .collect()
    }
}
