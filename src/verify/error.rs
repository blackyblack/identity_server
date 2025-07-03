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
    #[error("Nonce already used: {0}")]
    NonceUsedError(u64),
    #[error("Nonce limit reached")]
    NonceOverflowError,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
