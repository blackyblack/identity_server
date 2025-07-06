use std::collections::{HashMap, HashSet};

use async_std::sync::RwLock;
use async_trait::async_trait;

use crate::identity::{ModeratorProof, SystemPenalty, UserAddress, error::Error};

#[async_trait]
pub trait PenaltyStorage: Send + Sync {
    async fn set_moderator_penalty(
        &self,
        user: UserAddress,
        proof: ModeratorProof,
    ) -> Result<(), Error>;
    async fn set_forgotten_penalty(
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
    async fn set_moderator_penalty(
        &self,
        user: UserAddress,
        proof: ModeratorProof,
    ) -> Result<(), Error> {
        self.moderator_penalty.write().await.insert(user, proof);
        Ok(())
    }

    async fn set_forgotten_penalty(
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
