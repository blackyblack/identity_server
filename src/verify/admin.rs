use crate::{
    identity::UserAddress,
    verify::{
        error::Error,
        nonce::{Nonce, NonceManager},
        private_key_to_address,
        signature::Signature,
    },
};

pub async fn admin_verify(
    signature: &Signature,
    recipient: UserAddress,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    let message = admin_signature_message(recipient, signature.nonce);
    signature.verify(&message, nonce_manager).await
}

pub async fn admin_sign(
    private_key_hex: &str,
    recipient: UserAddress,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    let sender = private_key_to_address(private_key_hex)?;
    let nonce = nonce_manager.next_nonce(&sender).await?;
    let message = admin_signature_message(recipient, nonce);
    Signature::generate(private_key_hex, &message, nonce).await
}

fn admin_signature_message(user: UserAddress, nonce: Nonce) -> String {
    format!("admin/{user}/{nonce}")
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
        let signature = admin_sign(&private_key, user.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(admin_verify(&signature, user, &nonce_manager).await.is_ok());
    }

    #[async_std::test]
    async fn test_bad_user() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let signature = admin_sign(&private_key, user.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(
            admin_verify(&signature, "bad user".to_string(), &nonce_manager)
                .await
                .is_err()
        );
    }

    #[async_std::test]
    async fn test_bad_nonce() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let signature = admin_sign(&private_key, user.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_nonce = 6060;
        let signature = Signature {
            nonce: bad_nonce,
            ..signature
        };
        assert!(
            admin_verify(&signature, user, &nonce_manager)
                .await
                .is_err()
        );
    }

    #[async_std::test]
    async fn test_consumed_nonce() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let signature = admin_sign(&private_key, user.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(
            admin_verify(&signature, user.clone(), &nonce_manager)
                .await
                .is_ok()
        );
        // duplicate verification with the same nonce should fail
        let err = admin_verify(&signature, user, &nonce_manager)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NonceError(_)));
    }
}
