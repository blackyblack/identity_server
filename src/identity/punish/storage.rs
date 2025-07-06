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

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = InMemoryPenaltyStorage::default();
        let user = "user".to_string();
        let vouchee = "vouchee".to_string();

        let proof1 = ModeratorProof {
            moderator: "mod".to_string(),
            amount: 1,
            proof_id: 1,
            timestamp: 2,
        };
        storage
            .set_moderator_penalty(user.clone(), proof1.clone())
            .await
            .unwrap();
        let res = storage.moderator_penalty(&user).await.unwrap().unwrap();
        assert_eq!(res.moderator, proof1.moderator);
        assert_eq!(res.amount, proof1.amount);
        assert_eq!(res.proof_id, proof1.proof_id);
        assert_eq!(res.timestamp, proof1.timestamp);

        let proof2 = ModeratorProof {
            moderator: "mod2".to_string(),
            amount: 3,
            proof_id: 2,
            timestamp: 4,
        };
        storage
            .set_moderator_penalty(user.clone(), proof2.clone())
            .await
            .unwrap();
        let res = storage.moderator_penalty(&user).await.unwrap().unwrap();
        assert_eq!(res.moderator, proof2.moderator);
        assert_eq!(res.amount, proof2.amount);
        assert_eq!(res.proof_id, proof2.proof_id);
        assert_eq!(res.timestamp, proof2.timestamp);

        assert!(
            storage
                .moderator_penalty(&"none".to_string())
                .await
                .unwrap()
                .is_none()
        );

        let penalty1 = SystemPenalty {
            amount: 5,
            timestamp: 6,
        };
        storage
            .set_forgotten_penalty(user.clone(), vouchee.clone(), penalty1.clone())
            .await
            .unwrap();
        let res = storage
            .forgotten_penalty(&user, &vouchee)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(res.amount, penalty1.amount);
        assert_eq!(res.timestamp, penalty1.timestamp);
        assert!(
            storage
                .forgotten_users(&user)
                .await
                .unwrap()
                .contains(&vouchee)
        );

        let penalty2 = SystemPenalty {
            amount: 7,
            timestamp: 8,
        };
        storage
            .set_forgotten_penalty(user.clone(), vouchee.clone(), penalty2.clone())
            .await
            .unwrap();
        let res = storage
            .forgotten_penalty(&user, &vouchee)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(res.amount, penalty2.amount);
        assert_eq!(res.timestamp, penalty2.timestamp);

        storage
            .remove_forgotten(user.clone(), &vouchee)
            .await
            .unwrap();
        assert!(
            storage
                .forgotten_penalty(&user, &vouchee)
                .await
                .unwrap()
                .is_none()
        );
    }
}
