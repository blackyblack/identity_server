#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Caller does not have admin privileges")]
    NoAdminPriviledge,
}
