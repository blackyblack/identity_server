#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
