use crate::identity::{
    IdentityService, IdtAmount, ModeratorProof, ProofId, UserAddress, error::Error, next_timestamp,
};

pub mod db;
pub mod storage;

pub const MAX_IDT_BY_PROOF: IdtAmount = 50000;

impl IdentityService {
    pub async fn prove_with_timestamp(
        &self,
        user: UserAddress,
        moderator: UserAddress,
        balance: IdtAmount,
        proof_id: ProofId,
        timestamp: u64,
    ) -> Result<(), Error> {
        if balance > MAX_IDT_BY_PROOF {
            return Err(Error::MaxBalanceExceeded);
        }
        let event = ModeratorProof {
            moderator,
            amount: balance,
            proof_id,
            timestamp,
        };
        self.proofs.set_proof(user, event).await
    }

    // TODO: avoid Option
    pub async fn proof(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error> {
        self.proofs.proof(user).await
    }
}

pub async fn prove(
    service: &IdentityService,
    user: UserAddress,
    moderator: UserAddress,
    balance: IdtAmount,
    proof_id: ProofId,
) -> Result<(), Error> {
    service
        .prove_with_timestamp(user, moderator, balance, proof_id, next_timestamp())
        .await
}

#[cfg(test)]
mod tests {
    use crate::identity::tests::{MODERATOR, PROOF_ID, USER_A};

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let service = IdentityService::default();
        assert!(service.proof(&USER_A.to_string()).await.unwrap().is_none());
        assert!(
            prove(
                &service,
                USER_A.to_string(),
                MODERATOR.to_string(),
                100,
                PROOF_ID
            )
            .await
            .is_ok()
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .amount,
            100
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .moderator,
            MODERATOR
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .proof_id,
            PROOF_ID
        );
        assert!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .timestamp
                > 0
        );

        assert!(
            prove(&service, USER_A.to_string(), MODERATOR.to_string(), 200, 2)
                .await
                .is_ok()
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .amount,
            200
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .moderator,
            MODERATOR
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .proof_id,
            2
        );
        // we do not compare with previous timestamp because it is measured in seconds and
        // test runs way much faster
        assert!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .timestamp
                > 0
        );
    }

    #[async_std::test]
    async fn test_max_balance() {
        let service = IdentityService::default();
        assert!(service.proof(&USER_A.to_string()).await.unwrap().is_none());
        assert!(
            prove(
                &service,
                USER_A.to_string(),
                MODERATOR.to_string(),
                40000,
                PROOF_ID
            )
            .await
            .is_ok()
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .amount,
            40000
        );
        assert!(
            prove(
                &service,
                USER_A.to_string(),
                MODERATOR.to_string(),
                50001,
                PROOF_ID
            )
            .await
            .is_err()
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .amount,
            40000
        );
        assert!(
            prove(
                &service,
                USER_A.to_string(),
                MODERATOR.to_string(),
                60000,
                PROOF_ID
            )
            .await
            .is_err()
        );
        assert_eq!(
            service
                .proof(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .amount,
            40000
        );
    }
}
