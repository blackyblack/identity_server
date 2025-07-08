use std::str::FromStr;

use ethers_core::types::{H160, Signature as EthSignature};
use ethers_signers::Signer;
use serde::{Deserialize, Serialize};

use crate::{
    identity::UserAddress,
    verify::{
        error::Error,
        nonce::{Nonce, NonceManager},
        private_key_to_wallet,
    },
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Signature {
    pub signer: UserAddress,
    pub signature: String,
    pub nonce: Nonce,
}

pub async fn generate(private_key_hex: &str, message: String) -> Result<String, Error> {
    let wallet = private_key_to_wallet(private_key_hex)?;
    let eth_signature = wallet.sign_message(message).await?;
    Ok(format!("0x{}", eth_signature))
}

pub async fn consume(
    signature: String,
    signer: &UserAddress,
    message: String,
    nonce: Nonce,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    let eth_signature = EthSignature::from_str(signature.as_str())?;
    let signer_address = H160::from_str(signer).map_err(|e| {
        Error::AddressParseError(format!("Failed to parse signer address: {:?}", e))
    })?;
    eth_signature
        .verify(message, signer_address)
        .map_err(Error::SignatureVerificationFailed)?;
    nonce_manager.use_nonce(signer, nonce).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::verify::{error::Error, nonce::InMemoryNonceManager, random_keypair};

    use super::*;

    #[async_std::test]
    async fn test_generate_and_verify_signature() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message".to_string();
        let signature = generate(&private_key, message.clone())
            .await
            .expect("Should generate signature");
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        assert!(
            consume(signature, &user, message, nonce, &nonce_manager)
                .await
                .is_ok()
        );
    }

    #[async_std::test]
    async fn test_invalid_signature() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message".to_string();
        let mut signature = generate(&private_key, message.clone())
            .await
            .expect("Should generate signature");
        // tamper with the signature
        signature.push_str("bad");
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        assert!(
            consume(signature, &user, message, nonce, &nonce_manager)
                .await
                .is_err()
        );
    }

    #[async_std::test]
    async fn test_invalid_nonce() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message".to_string();
        let signature = generate(&private_key, message.clone())
            .await
            .expect("Should generate signature");
        // change the nonce
        let bad_nonce = 67890;
        // should still verify since nonce is written in the signed message and
        // it was not tampered. Nonce is only used to prevent replay attacks and
        // should be consumed by the NonceManager.
        assert!(
            consume(signature, &user, message, bad_nonce, &nonce_manager)
                .await
                .is_ok()
        );
    }

    #[async_std::test]
    async fn test_wrong_signer() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let (_, user2) = random_keypair();
        let message = "message".to_string();
        let signature = generate(&private_key, message.clone())
            .await
            .expect("Should generate signature");
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        // replace user address with wallet2's address
        assert!(
            consume(signature, &user2, message, nonce, &nonce_manager)
                .await
                .is_err()
        );
    }

    #[async_std::test]
    async fn test_wrong_message() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message".to_string();
        let signature = generate(&private_key, message)
            .await
            .expect("Should generate signature");
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        let message_bad = "bad message".to_string();
        assert!(
            consume(signature, &user, message_bad, nonce, &nonce_manager)
                .await
                .is_err()
        );
    }

    #[async_std::test]
    async fn test_duplicate_nonce() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message".to_string();
        let signature = generate(&private_key, message.clone())
            .await
            .expect("Should generate signature");
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        assert!(
            consume(
                signature.clone(),
                &user,
                message.clone(),
                nonce,
                &nonce_manager
            )
            .await
            .is_ok()
        );
        // second verification with the same nonce should fail
        let err = consume(signature, &user, message, nonce, &nonce_manager)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::NonceError(_)));
    }
}
