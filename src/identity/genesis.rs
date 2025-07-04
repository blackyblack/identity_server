use std::collections::HashMap;

use crate::identity::{IdentityService, IdtAmount, UserAddress, error::Error};

impl IdentityService {
    pub async fn set_genesis(&self, users: HashMap<UserAddress, IdtAmount>) -> Result<(), Error> {
        self.proofs.set_genesis(users).await
    }

    pub async fn genesis_balance(&self, user: &UserAddress) -> Result<Option<IdtAmount>, Error> {
        self.proofs.genesis_balance(user).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{
        idt::balance,
        proof::prove,
        tests::{MODERATOR, PROOF_ID},
    };
    use std::collections::HashMap;

    #[async_std::test]
    async fn test_genesis_balance() {
        let service = IdentityService::default();
        let genesis_user = "genesis_user".to_string();
        let genesis_balance = 500;

        // set up a genesis balance for the user
        let mut balances = HashMap::new();
        balances.insert(genesis_user.clone(), genesis_balance);
        service.set_genesis(balances).await.unwrap();

        // check that the balance is recognized through the `balance` function
        assert_eq!(
            balance(&service, &genesis_user).await.unwrap(),
            genesis_balance
        );

        // now add a moderator proof and check that it overrides the genesis balance
        let moderator_balance = 1000;
        prove(
            &service,
            genesis_user.clone(),
            MODERATOR.to_string(),
            moderator_balance,
            PROOF_ID,
        )
        .await
        .unwrap();

        // check that the balance is updated
        assert_eq!(
            balance(&service, &genesis_user).await.unwrap(),
            moderator_balance
        );

        // check that the proof is updated
        let proof = service.proof(&genesis_user).await.unwrap();
        assert!(proof.is_some());
        let proof = proof.unwrap();
        assert_eq!(proof.amount, moderator_balance);
        assert_eq!(proof.moderator, MODERATOR);
        assert_eq!(proof.proof_id, PROOF_ID);
    }
}
