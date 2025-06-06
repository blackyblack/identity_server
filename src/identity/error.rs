#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Max balance from proof exceeded")]
    MaxBalanceExceeded,
}
