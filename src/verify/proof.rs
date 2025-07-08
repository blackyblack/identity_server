use crate::{
    identity::{IdtAmount, ProofId, UserAddress},
    verify::{
        error::Error,
        nonce::{Nonce, NonceManager},
        sign_message,
        signature::Signature,
        verify_message,
    },
};

pub async fn proof_sign(
    private_key_hex: &str,
    user: UserAddress,
    amount: IdtAmount,
    proof_id: ProofId,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    sign_message(
        private_key_hex,
        &proof_message_prefix(user, amount, proof_id),
        nonce_manager,
    )
    .await
}

pub async fn proof_verify(
    signature: String,
    signer: &UserAddress,
    nonce: Nonce,
    user: UserAddress,
    amount: IdtAmount,
    proof_id: ProofId,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    verify_message(
        signature,
        signer,
        nonce,
        &proof_message_prefix(user, amount, proof_id),
        nonce_manager,
    )
    .await
}

fn proof_message_prefix(user: UserAddress, amount: IdtAmount, proof_id: ProofId) -> String {
    format!("proof/{user}/{amount}/{proof_id}")
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
            proof_verify(
                signature.signature,
                &signature.signer,
                signature.nonce,
                user,
                amount,
                proof_id,
                &nonce_manager
            )
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
            proof_verify(
                signature.signature,
                &signature.signer,
                signature.nonce,
                bad_user,
                amount,
                proof_id,
                &nonce_manager
            )
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
            proof_verify(
                signature.signature,
                &signature.signer,
                signature.nonce,
                user,
                bad_amount,
                proof_id,
                &nonce_manager
            )
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
        assert!(
            proof_verify(
                signature.signature,
                &signature.signer,
                bad_nonce,
                user,
                amount,
                proof_id,
                &nonce_manager
            )
            .await
            .is_err()
        );
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
            proof_verify(
                signature.signature,
                &signature.signer,
                signature.nonce,
                user,
                amount,
                bad_proof_id,
                &nonce_manager
            )
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
            proof_verify(
                signature.signature.clone(),
                &signature.signer,
                signature.nonce,
                user.clone(),
                amount,
                proof_id,
                &nonce_manager
            )
            .await
            .is_ok()
        );
        // duplicate verification with the same nonce should fail
        let err = proof_verify(
            signature.signature,
            &signature.signer,
            signature.nonce,
            user,
            amount,
            proof_id,
            &nonce_manager,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, Error::NonceError(_)));
    }
}
