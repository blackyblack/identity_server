use crate::verify::nonce::Nonce;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Nonce already used: {0}")]
    NonceUsedError(Nonce),
    #[error("Nonce limit reached")]
    NonceOverflowError,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
