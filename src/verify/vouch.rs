use crate::{
    identity::UserAddress,
    verify::{
        error::Error,
        nonce::{Nonce, NonceManager},
        private_key_to_address,
        signature::Signature,
    },
};

pub async fn vouch_sign(
    private_key_hex: &str,
    vouchee: UserAddress,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    let user = private_key_to_address(private_key_hex)?;
    let nonce = nonce_manager.next_nonce(&user).await?;
    let message = vouch_signature_message(vouchee, nonce);
    Signature::generate(private_key_hex, &message, nonce).await
}

pub async fn vouch_verify(
    signature: &Signature,
    vouchee: UserAddress,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    let message = vouch_signature_message(vouchee.clone(), signature.nonce);
    signature.verify(&message, nonce_manager).await
}

pub async fn vouch_server_sign(
    private_key_hex: &str,
    user_signature: String,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    let server = private_key_to_address(private_key_hex)?;
    let nonce = nonce_manager.next_nonce(&server).await?;
    let message = vouch_server_signature_message(&user_signature, nonce);
    Signature::generate(private_key_hex, &message, nonce).await
}

pub async fn vouch_server_verify(
    signature: &Signature,
    user_signature: String,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    let message = vouch_server_signature_message(&user_signature, signature.nonce);
    signature.verify(&message, nonce_manager).await
}

fn vouch_server_signature_message(signature: &str, nonce: Nonce) -> String {
    format!("vouch_server/{signature}/{nonce}")
}

fn vouch_signature_message(user: UserAddress, nonce: Nonce) -> String {
    format!("vouch/{user}/{nonce}")
}

#[cfg(test)]
mod tests {
    use crate::verify::{nonce::InMemoryNonceManager, random_keypair};

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let (private_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let vouchee = "vouchee".to_string();
        let signature = vouch_sign(&private_key, vouchee.clone(), &nonce_manager)
            .await
            .expect("Should generate signature");
        assert!(
            vouch_verify(&signature, vouchee, &nonce_manager)
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
            vouch_verify(&signature, bad_user, &nonce_manager)
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
        let signature = Signature {
            nonce: bad_nonce,
            ..signature
        };
        assert!(
            vouch_verify(&signature, vouchee, &nonce_manager)
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
            vouch_verify(&signature, vouchee.clone(), &nonce_manager)
                .await
                .is_ok()
        );
        // duplicate verification with the same nonce should fail
        let err = vouch_verify(&signature, vouchee, &nonce_manager)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NonceError(_)));
    }

    #[async_std::test]
    async fn test_server_sign_verify() {
        let (user_key, _) = random_keypair();
        let (server_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let vouchee = "vouchee".to_string();
        let user_sig = vouch_sign(&user_key, vouchee.clone(), &nonce_manager)
            .await
            .expect("Should sign");
        let server_sig = vouch_server_sign(&server_key, user_sig.signature.clone(), &nonce_manager)
            .await
            .expect("Should server sign");
        assert!(
            vouch_server_verify(&server_sig, user_sig.signature, &nonce_manager)
                .await
                .is_ok()
        );
    }
}
