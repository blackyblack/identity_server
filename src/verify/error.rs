use ethers_core::types::SignatureError;
use ethers_signers::WalletError;
use hex::FromHexError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Hex decode error: {0}")]
    HexDecodeError(#[from] FromHexError),
    #[error("Signature generation error: {0}")]
    SignatureGenerationError(#[from] WalletError),
    #[error("Wallet creation error: {0}")]
    WalletCreationError(String),
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(#[from] SignatureError),
    #[error("Address parsing failed: {0}")]
    AddressParseError(String),
    #[error("Nonce error: {0}")]
    NonceError(#[from] crate::verify::nonce::error::Error),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
