use std::str::FromStr;

use ethers_core::types::{H160, Signature as EthSignature};
use ethers_signers::Signer;
use serde::{Deserialize, Serialize};

use crate::{
    identity::UserAddress,
    verify::{
        Nonce, error::Error, nonce::NonceManager, private_key_to_address, private_key_to_wallet,
    },
};

#[derive(Deserialize, Serialize, Clone)]
pub struct Signature {
    pub signer: UserAddress,
    pub signature: String,
    pub nonce: Nonce,
}

impl Signature {
    pub async fn generate(
        private_key_hex: &str,
        message: &str,
        nonce: Nonce,
    ) -> Result<Self, Error> {
        let signer = private_key_to_address(private_key_hex)?;
        let wallet = private_key_to_wallet(private_key_hex)?;
        let eth_signature = wallet.sign_message(message).await?;
        Ok(Self {
            signer,
            signature: format!("0x{}", eth_signature),
            nonce,
        })
    }

    pub async fn verify(
        &self,
        message: &str,
        nonce_manager: &dyn NonceManager,
    ) -> Result<(), Error> {
        let eth_signature = EthSignature::from_str(&self.signature)?;
        let signer_address = H160::from_str(&self.signer).map_err(|e| {
            Error::AddressParseError(format!("Failed to parse signer address: {:?}", e))
        })?;
        eth_signature
            .verify(message, signer_address)
            .map_err(Error::SignatureVerificationFailed)?;
        nonce_manager.use_nonce(&self.signer, self.nonce).await
    }
}

#[cfg(test)]
mod tests {
    use crate::verify::{error::Error, nonce::InMemoryNonceManager, random_keypair};

    use super::*;

    #[async_std::test]
    async fn test_generate_and_verify_signature() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message";
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        let signature = Signature::generate(&private_key, message, nonce)
            .await
            .expect("Should generate signature");
        assert!(signature.verify(message, &nonce_manager).await.is_ok());
    }

    #[async_std::test]
    async fn test_invalid_signature() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message";
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        let mut signature = Signature::generate(&private_key, message, nonce)
            .await
            .expect("Should generate signature");
        // tamper with the signature
        signature.signature.push_str("bad");
        assert!(signature.verify(message, &nonce_manager).await.is_err());
    }

    #[async_std::test]
    async fn test_invalid_nonce() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message";
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        let mut signature = Signature::generate(&private_key, message, nonce)
            .await
            .expect("Should generate signature");
        // change the nonce
        signature.nonce = 67890;
        // should still verify since nonce is written in the signed message and
        // it was not tampered. Nonce is only used to prevent replay attacks and
        // should be consumed by the NonceManager.
        assert!(signature.verify(message, &nonce_manager).await.is_ok());
    }

    #[async_std::test]
    async fn test_wrong_signer() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let (_, user2) = random_keypair();
        let message = "message";
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        let mut signature = Signature::generate(&private_key, message, nonce)
            .await
            .expect("Should generate signature");
        // replace user address with wallet2's address
        signature.signer = user2;
        assert!(signature.verify(message, &nonce_manager).await.is_err());
    }

    #[async_std::test]
    async fn test_wrong_message() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message";
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        let signature = Signature::generate(&private_key, message, nonce)
            .await
            .expect("Should generate signature");
        let message_bad = "bad message";
        assert!(signature.verify(message_bad, &nonce_manager).await.is_err());
    }

    #[async_std::test]
    async fn test_duplicate_nonce() {
        let nonce_manager = InMemoryNonceManager::default();
        let (private_key, user) = random_keypair();
        let message = "message";
        let nonce = nonce_manager.next_nonce(&user).await.unwrap();
        let signature = Signature::generate(&private_key, message, nonce)
            .await
            .expect("Should generate signature");
        assert!(signature.verify(message, &nonce_manager).await.is_ok());
        // second verification with the same nonce should fail
        let err = signature.verify(message, &nonce_manager).await.unwrap_err();
        assert!(matches!(err, Error::NonceUsedError(_)));
    }
}
