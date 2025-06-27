use crate::{
    identity::UserAddress,
    verify::{error::Error, nonce::NonceManager, private_key_to_address, signature::Signature},
};

pub async fn forget_sign(
    private_key_hex: &str,
    vouchee: UserAddress,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    let user = private_key_to_address(private_key_hex)?;
    let nonce = nonce_manager.next_nonce(&user);
    let message = forget_signature_message(vouchee, nonce);
    Signature::generate(private_key_hex, &message, nonce).await
}

pub fn forget_verify(
    signature: &Signature,
    vouchee: UserAddress,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    let message = forget_signature_message(vouchee, signature.nonce);
    signature.verify(&message, nonce_manager)
}

fn forget_signature_message(user: UserAddress, nonce: u64) -> String {
    format!("forget/{user}/{nonce}")
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
        let signature = forget_sign(&private_key, user.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(forget_verify(&signature, user, &nonce_manager).is_ok());
    }

    #[async_std::test]
    async fn test_bad_user() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let signature = forget_sign(&private_key, user.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_user = "bad user".to_string();
        assert!(forget_verify(&signature, bad_user, &nonce_manager).is_err());
    }

    #[async_std::test]
    async fn test_bad_nonce() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let signature = forget_sign(&private_key, user.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        let bad_nonce = 6060;
        let signature = Signature {
            nonce: bad_nonce,
            ..signature
        };
        let err = forget_verify(&signature, user, &nonce_manager).unwrap_err();
        assert!(matches!(err, Error::SignatureVerificationFailed(_)));
    }

    #[async_std::test]
    async fn test_consumed_nonce() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user = "user".to_string();
        let signature = forget_sign(&private_key, user.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(forget_verify(&signature, user.clone(), &nonce_manager).is_ok());
        // duplicate verification with the same nonce should fail
        let err = forget_verify(&signature, user, &nonce_manager).unwrap_err();
        assert!(matches!(err, Error::NonceUsedError(_)));
    }
}
