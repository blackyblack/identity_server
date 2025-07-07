use crate::verify::{
    error::Error,
    nonce::{Nonce, NonceManager},
    private_key_to_address,
    signature::Signature,
};

pub async fn server_sign(
    private_key_hex: &str,
    user_signature: String,
    nonce_manager: &dyn NonceManager,
) -> Result<Signature, Error> {
    let server = private_key_to_address(private_key_hex)?;
    let nonce = nonce_manager.next_nonce(&server).await?;
    let message = server_signature_message(&user_signature, nonce);
    Signature::generate(private_key_hex, &message, nonce).await
}

pub async fn server_verify(
    signature: &Signature,
    user_signature: String,
    nonce_manager: &dyn NonceManager,
) -> Result<(), Error> {
    let message = server_signature_message(&user_signature, signature.nonce);
    signature.verify(&message, nonce_manager).await
}

fn server_signature_message(signature: &str, nonce: Nonce) -> String {
    format!("server/{signature}/{nonce}")
}

#[cfg(test)]
mod tests {
    use crate::verify::{nonce::InMemoryNonceManager, random_keypair};

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let (server_key, _) = random_keypair();
        let nonce_manager = InMemoryNonceManager::default();
        let user_sig = "signature".to_string();
        let server_sig = server_sign(&server_key, user_sig.clone(), &nonce_manager)
            .await
            .expect("Should server sign");
        assert!(
            server_verify(&server_sig, user_sig, &nonce_manager)
                .await
                .is_ok()
        );
    }
}
