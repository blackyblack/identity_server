use std::time::SystemTime;

mod decay;
pub mod error;
pub mod idt;
pub mod proof;
pub mod punish;
mod tree_walk;
pub mod vouch;

pub fn next_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Should be after the UNIX_EPOCH timestamp")
        .as_secs()
}

#[cfg(test)]
pub mod tests {
    use crate::state::ProofId;

    pub const MODERATOR: &str = "moderator";
    pub const USER_A: &str = "userA";
    pub const PROOF_ID: ProofId = 1;
}
