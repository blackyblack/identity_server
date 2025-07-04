use serde::Deserialize;

use crate::identity::{
    error::Error,
    next_timestamp,
    IdtAmount,
    ModeratorProof,
    UserAddress,
    IdentityService,
};

#[derive(Deserialize)]
pub struct GenesisBalance {
    pub user: UserAddress,
    pub balance: IdtAmount,
}

impl IdentityService {
    pub async fn create_initial_balance_with_timestamp(
        &self,
        user: UserAddress,
        balance: IdtAmount,
        timestamp: u64,
    ) -> Result<(), Error> {
        let proof = ModeratorProof {
            moderator: String::new(),
            amount: balance,
            proof_id: 0,
            timestamp,
        };
        self.proofs.set_proof(user, proof).await
    }

    pub async fn create_initial_balance(
        &self,
        user: UserAddress,
        balance: IdtAmount,
    ) -> Result<(), Error> {
        self
            .create_initial_balance_with_timestamp(user, balance, next_timestamp())
            .await
    }

    pub async fn apply_genesis_balances(
        &self,
        balances: Vec<GenesisBalance>,
    ) -> Result<(), Error> {
        for entry in balances {
            self.create_initial_balance(entry.user, entry.balance).await?;
        }
        Ok(())
    }
}
