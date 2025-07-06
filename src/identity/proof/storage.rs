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
