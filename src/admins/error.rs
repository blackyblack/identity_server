#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Caller does not have admin privileges")]
    NoAdminPrivilege,
    #[error("Caller does not have moderator privileges")]
    NoModeratorPrivilege,
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
