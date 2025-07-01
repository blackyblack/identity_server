use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use async_trait::async_trait;

use super::{ModeratorProof, SystemPenalty, UserAddress, error::Error};

#[async_trait]
pub trait VouchStorage: Send + Sync {
    async fn vouch(&self, from: UserAddress, to: UserAddress, timestamp: u64) -> Result<(), Error>;
    async fn vouchers_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error>;
    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error>;
    async fn remove_vouch(&self, voucher: UserAddress, vouchee: UserAddress) -> Result<(), Error>;
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

#[async_trait]
impl VouchStorage for InMemoryVouchStorage {
    async fn vouch(&self, from: UserAddress, to: UserAddress, timestamp: u64) -> Result<(), Error> {
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
        Ok(())
    }

    async fn vouchers_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        Ok(self
            .data
            .read()
            .expect("Poisoned RwLock detected")
            .vouchers
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        Ok(self
            .data
            .read()
            .expect("Poisoned RwLock detected")
            .vouchees
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn remove_vouch(&self, voucher: UserAddress, vouchee: UserAddress) -> Result<(), Error> {
        let mut lock = self.data.write().expect("Poisoned RwLock detected");
        lock.vouchers.entry(vouchee.clone()).and_modify(|v| {
            v.remove(&voucher);
        });
        lock.vouchees.entry(voucher).and_modify(|v| {
            v.remove(&vouchee);
        });
        Ok(())
    }
}

#[async_trait]
pub trait ProofStorage: Send + Sync {
    async fn set_proof(&self, user: UserAddress, proof: ModeratorProof) -> Result<(), Error>;
    async fn proof(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error>;
}

#[derive(Default)]
pub struct InMemoryProofStorage {
    data: RwLock<HashMap<UserAddress, ModeratorProof>>,
}

#[async_trait]
impl ProofStorage for InMemoryProofStorage {
    async fn set_proof(&self, user: UserAddress, proof: ModeratorProof) -> Result<(), Error> {
        self.data
            .write()
            .expect("Poisoned RwLock detected")
            .insert(user, proof);
        Ok(())
    }

    async fn proof(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error> {
        Ok(self
            .data
            .read()
            .expect("Poisoned RwLock detected")
            .get(user)
            .cloned())
    }
}

#[async_trait]
pub trait PenaltyStorage: Send + Sync {
    async fn insert_moderator_penalty(
        &self,
        user: UserAddress,
        proof: ModeratorProof,
    ) -> Result<(), Error>;
    async fn insert_forgotten_penalty(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        penalty: SystemPenalty,
    ) -> Result<(), Error>;
    async fn remove_forgotten(
        &self,
        user: UserAddress,
        forgotten: &UserAddress,
    ) -> Result<(), Error>;
    async fn moderator_penalty(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error>;
    async fn forgotten_penalty(
        &self,
        user: &UserAddress,
        forgotten: &UserAddress,
    ) -> Result<Option<SystemPenalty>, Error>;
    async fn forgotten_users(&self, user: &UserAddress) -> Result<HashSet<UserAddress>, Error>;
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

#[async_trait]
impl PenaltyStorage for InMemoryPenaltyStorage {
    async fn insert_moderator_penalty(
        &self,
        user: UserAddress,
        proof: ModeratorProof,
    ) -> Result<(), Error> {
        self.data
            .write()
            .expect("Poisoned RwLock detected")
            .moderator_penalty
            .insert(user, proof);
        Ok(())
    }

    async fn insert_forgotten_penalty(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        penalty: SystemPenalty,
    ) -> Result<(), Error> {
        self.data
            .write()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .entry(user)
            .and_modify(|v| {
                v.insert(vouchee.clone(), penalty.clone());
            })
            .or_insert_with(move || HashMap::from([(vouchee, penalty)]));
        Ok(())
    }

    async fn remove_forgotten(
        &self,
        user: UserAddress,
        forgotten: &UserAddress,
    ) -> Result<(), Error> {
        self.data
            .write()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .entry(user)
            .and_modify(|v| {
                v.remove(forgotten);
            });
        Ok(())
    }

    async fn moderator_penalty(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error> {
        Ok(self
            .data
            .read()
            .expect("Poisoned RwLock detected")
            .moderator_penalty
            .get(user)
            .cloned())
    }

    async fn forgotten_penalty(
        &self,
        user: &UserAddress,
        forgotten: &UserAddress,
    ) -> Result<Option<SystemPenalty>, Error> {
        Ok(self
            .data
            .read()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .get(user)
            .and_then(|v| v.get(forgotten).cloned()))
    }

    async fn forgotten_users(&self, user: &UserAddress) -> Result<HashSet<UserAddress>, Error> {
        Ok(self
            .data
            .read()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .get(user)
            .cloned()
            .unwrap_or_default()
            .into_keys()
            .collect())
    }
}
