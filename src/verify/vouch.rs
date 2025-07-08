use crate::{
    identity::UserAddress,
    verify::{
        error::Error,
        nonce::{Nonce, NonceManager},
        sign_message,
        signature::Signature,
        verify_message,
    },
};

pub async fn vouch_sign(
    private_key_hex: &str,
    vouchee: UserAddress,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    sign_message(
        private_key_hex,
        &vouch_message_prefix(vouchee),
        nonce_manager,
    )
    .await
}

pub async fn vouch_verify(
    signature: String,
    signer: &UserAddress,
    nonce: Nonce,
    vouchee: UserAddress,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    verify_message(
        signature,
        signer,
        nonce,
        &vouch_message_prefix(vouchee),
        nonce_manager,
    )
    .await
}

fn vouch_message_prefix(user: UserAddress) -> String {
    format!("vouch/{user}")
}

#[cfg(test)]
mod tests {
    use crate::verify::{nonce::InMemoryNonceManager, random_keypair};

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let vouchee: String = "vouchee".to_string();
        let signature = vouch_sign(&private_key, vouchee.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(
            vouch_verify(
                signature.signature,
                &signature.signer,
                signature.nonce,
                vouchee,
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
        let vouchee = "vouchee".to_string();
        let signature = vouch_sign(&private_key, vouchee.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_user = "bad user".to_string();
        assert!(
            vouch_verify(
                signature.signature,
                &signature.signer,
                signature.nonce,
                bad_user,
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
        let vouchee = "vouchee".to_string();
        let signature = vouch_sign(&private_key, vouchee.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_nonce = 6060;
        assert!(
            vouch_verify(
                signature.signature,
                &signature.signer,
                bad_nonce,
                vouchee,
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
        let vouchee = "vouchee".to_string();
        let signature = vouch_sign(&private_key, vouchee.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(
            vouch_verify(
                signature.signature.clone(),
                &signature.signer,
                signature.nonce,
                vouchee.clone(),
                &nonce_manager
            )
            .await
            .is_ok()
        );
        // duplicate verification with the same nonce should fail
        let err = vouch_verify(
            signature.signature,
            &signature.signer,
            signature.nonce,
            vouchee,
            &nonce_manager,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, Error::NonceError(_)));
    }
}
