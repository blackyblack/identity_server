use std::collections::{HashMap, HashSet};

use async_std::sync::RwLock;
use async_trait::async_trait;

use crate::identity::IdtAmount;

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
    // key - vouchee, vouch object
    // value - (voucher, unix timestamp) map
    vouchers: HashMap<UserAddress, HashMap<UserAddress, u64>>,
    // key - voucher, vouch subject
    // value - (vouchee, unix timestamp) map
    vouchees: HashMap<UserAddress, HashMap<UserAddress, u64>>,
}

#[derive(Default)]
pub struct InMemoryVouchStorage {
    // separate struct for atomic access to vouch data without deadlocks
    data: RwLock<VouchData>,
}

#[async_trait]
impl VouchStorage for InMemoryVouchStorage {
    async fn vouch(&self, from: UserAddress, to: UserAddress, timestamp: u64) -> Result<(), Error> {
        let mut lock = self.data.write().await;
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
            .await
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
            .await
            .vouchees
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn remove_vouch(&self, voucher: UserAddress, vouchee: UserAddress) -> Result<(), Error> {
        let mut lock = self.data.write().await;
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
    async fn set_genesis(&self, users: HashMap<UserAddress, IdtAmount>) -> Result<(), Error>;
    async fn genesis_balance(&self, user: &UserAddress) -> Result<Option<IdtAmount>, Error>;
    async fn set_proof(&self, user: UserAddress, proof: ModeratorProof) -> Result<(), Error>;
    async fn proof(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error>;
}

#[derive(Default)]
pub struct InMemoryProofStorage {
    // key - proven user
    // only single proof for a user is allowed. If proof should be updated,
    // moderator should prove again and update proof manually.
    data: RwLock<HashMap<UserAddress, ModeratorProof>>,
    genesis: RwLock<HashMap<UserAddress, IdtAmount>>,
}

#[async_trait]
impl ProofStorage for InMemoryProofStorage {
    async fn set_genesis(&self, users: HashMap<UserAddress, IdtAmount>) -> Result<(), Error> {
        let mut genesis_lock = self.genesis.write().await;
        *genesis_lock = users;
        Ok(())
    }

    async fn genesis_balance(&self, user: &UserAddress) -> Result<Option<IdtAmount>, Error> {
        Ok(self.genesis.read().await.get(user).cloned())
    }

    async fn set_proof(&self, user: UserAddress, proof: ModeratorProof) -> Result<(), Error> {
        self.data.write().await.insert(user, proof);
        Ok(())
    }

    async fn proof(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error> {
        Ok(self.data.read().await.get(user).cloned())
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
pub struct InMemoryPenaltyStorage {
    // key - punished user
    // only single ban for a user is allowed. If penalty should be increased,
    // moderator should ban again and increase penalty manually.
    moderator_penalty: RwLock<HashMap<UserAddress, ModeratorProof>>,
    // outer map key - punished user
    // inner map key - forgotten user
    forget_penalties: RwLock<HashMap<UserAddress, HashMap<UserAddress, SystemPenalty>>>,
}

#[async_trait]
impl PenaltyStorage for InMemoryPenaltyStorage {
    async fn insert_moderator_penalty(
        &self,
        user: UserAddress,
        proof: ModeratorProof,
    ) -> Result<(), Error> {
        self.moderator_penalty.write().await.insert(user, proof);
        Ok(())
    }

    async fn insert_forgotten_penalty(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        penalty: SystemPenalty,
    ) -> Result<(), Error> {
        self.forget_penalties
            .write()
            .await
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
        self.forget_penalties
            .write()
            .await
            .entry(user)
            .and_modify(|v| {
                v.remove(forgotten);
            });
        Ok(())
    }

    async fn moderator_penalty(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error> {
        Ok(self.moderator_penalty.read().await.get(user).cloned())
    }

    async fn forgotten_penalty(
        &self,
        user: &UserAddress,
        forgotten: &UserAddress,
    ) -> Result<Option<SystemPenalty>, Error> {
        Ok(self
            .forget_penalties
            .read()
            .await
            .get(user)
            .and_then(|v| v.get(forgotten).cloned()))
    }

    async fn forgotten_users(&self, user: &UserAddress) -> Result<HashSet<UserAddress>, Error> {
        Ok(self
            .forget_penalties
            .read()
            .await
            .get(user)
            .cloned()
            .unwrap_or_default()
            .into_keys()
            .collect())
    }
}
