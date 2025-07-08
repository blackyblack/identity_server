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

pub async fn proxy_sign(
    private_key_hex: &str,
    user_signature: &str,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    sign_message(
        private_key_hex,
        &proxy_message_prefix(user_signature),
        nonce_manager,
    )
    .await
}

pub async fn proxy_verify(
    signature: String,
    signer: &UserAddress,
    nonce: Nonce,
    user_signature: &str,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    verify_message(
        signature,
        signer,
        nonce,
        &proxy_message_prefix(user_signature),
        nonce_manager,
    )
    .await
}

fn proxy_message_prefix(user_signature: &str) -> String {
    format!("proxy/{user_signature}")
}

#[cfg(test)]
mod tests {
    use crate::verify::{nonce::InMemoryNonceManager, random_keypair};

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let (server_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user_sig = "signature";
        let server_sig = proxy_sign(&server_key, user_sig, &nonce_manager)
            .await
            .expect("Should server sign");
        assert!(
            proxy_verify(
                server_sig.signature,
                &server_sig.signer,
                server_sig.nonce,
                user_sig,
                &nonce_manager
            )
            .await
            .is_ok()
        );
    }
}
