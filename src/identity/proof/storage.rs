use std::collections::HashMap;

use async_std::sync::RwLock;
use async_trait::async_trait;

use crate::identity::{IdtAmount, ModeratorProof, UserAddress, error::Error};

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[async_std::test]
    async fn test_inmemory_proof_storage() {
        let storage = InMemoryProofStorage::default();
        let user = "user".to_string();
        let moderator = "moderator".to_string();

        let mut genesis = HashMap::<UserAddress, IdtAmount>::new();
        genesis.insert(user.clone(), 100);
        storage.set_genesis(genesis).await.unwrap();
        assert_eq!(storage.genesis_balance(&user).await.unwrap().unwrap(), 100);
        assert!(
            storage
                .genesis_balance(&"none".to_string())
                .await
                .unwrap()
                .is_none()
        );

        let proof1 = ModeratorProof {
            moderator: moderator.clone(),
            amount: 10,
            proof_id: 1,
            timestamp: 1,
        };
        storage
            .set_proof(user.clone(), proof1.clone())
            .await
            .unwrap();
        let res = storage.proof(&user).await.unwrap().unwrap();
        assert_eq!(res.moderator, proof1.moderator);
        assert_eq!(res.amount, proof1.amount);
        assert_eq!(res.proof_id, proof1.proof_id);
        assert_eq!(res.timestamp, proof1.timestamp);

        let proof2 = ModeratorProof {
            moderator: "mod2".to_string(),
            amount: 20,
            proof_id: 2,
            timestamp: 2,
        };
        storage
            .set_proof(user.clone(), proof2.clone())
            .await
            .unwrap();
        let res = storage.proof(&user).await.unwrap().unwrap();
        assert_eq!(res.moderator, proof2.moderator);
        assert_eq!(res.amount, proof2.amount);
        assert_eq!(res.proof_id, proof2.proof_id);
        assert_eq!(res.timestamp, proof2.timestamp);

        assert!(storage.proof(&"none".to_string()).await.unwrap().is_none());
    }
}
