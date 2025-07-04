#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Max balance from proof exceeded")]
    MaxBalanceExceeded,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
