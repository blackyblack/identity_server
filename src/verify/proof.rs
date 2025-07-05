use crate::{
    identity::{IdtAmount, ProofId, UserAddress},
    verify::{
        Nonce, error::Error, nonce::NonceManager, private_key_to_address, signature::Signature,
    },
};

pub async fn proof_sign(
    private_key_hex: &str,
    user: UserAddress,
    amount: IdtAmount,
    proof_id: ProofId,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    let moderator = private_key_to_address(private_key_hex)?;
    let nonce = nonce_manager.next_nonce(&moderator).await?;
    let message = proof_signature_message(user, nonce, amount, proof_id);
    Signature::generate(private_key_hex, &message, nonce).await
}

pub async fn proof_verify(
    signature: &Signature,
    user: UserAddress,
    amount: IdtAmount,
    proof_id: ProofId,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    let message = proof_signature_message(user.clone(), signature.nonce, amount, proof_id);
    signature.verify(&message, nonce_manager).await
}

fn proof_signature_message(
    user: UserAddress,
    nonce: Nonce,
    amount: IdtAmount,
    proof_id: ProofId,
) -> String {
    format!("proof/{user}/{nonce}/{amount}/{proof_id}")
}

#[cfg(test)]
mod tests {
    use crate::verify::{nonce::InMemoryNonceManager, random_keypair};

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let amount = 100;
        let proof_id = 123;
        let signature = proof_sign(&private_key, user.clone(), amount, proof_id, &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(
            proof_verify(&signature, user, amount, proof_id, &nonce_manager)
                .await
                .is_ok()
        );
    }

    #[async_std::test]
    async fn test_bad_user() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let amount = 100;
        let proof_id = 123;
        let signature = proof_sign(&private_key, user.clone(), amount, proof_id, &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_user = "bad user".to_string();
        assert!(
            proof_verify(&signature, bad_user, amount, proof_id, &nonce_manager)
                .await
                .is_err()
        );
    }

    #[async_std::test]
    async fn test_bad_amount() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let amount = 100;
        let proof_id = 123;
        let signature = proof_sign(&private_key, user.clone(), amount, proof_id, &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_amount = 200;
        assert!(
            proof_verify(&signature, user, bad_amount, proof_id, &nonce_manager)
                .await
                .is_err()
        );
    }

    #[async_std::test]
    async fn test_bad_nonce() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let amount = 100;
        let proof_id = 123;
        let signature = proof_sign(&private_key, user.clone(), amount, proof_id, &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_nonce = 6060;
        let signature = Signature {
            nonce: bad_nonce,
            ..signature
        };
        assert!(
            proof_verify(&signature, user, amount, proof_id, &nonce_manager)
                .await
                .is_err()
        );
    }

    #[async_std::test]
    async fn test_consumed_nonce() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let amount = 100;
        let proof_id = 123;
        let signature = proof_sign(&private_key, user.clone(), amount, proof_id, &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(
            proof_verify(&signature, user.clone(), amount, proof_id, &nonce_manager)
                .await
                .is_ok()
        );
        // duplicate verification with the same nonce should fail
        let err = proof_verify(&signature, user, amount, proof_id, &nonce_manager)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NonceUsedError(_)));
    }

    #[async_std::test]
    async fn test_bad_proof_id() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let amount = 100;
        let proof_id = 123;
        let signature = proof_sign(&private_key, user.clone(), amount, proof_id, &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_proof_id = 6060;
        assert!(
            proof_verify(&signature, user, amount, bad_proof_id, &nonce_manager)
                .await
                .is_err()
        );
    }
}
