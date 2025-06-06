pub mod idt;
pub mod proof;
pub mod vouch;
pub mod error;

#[cfg(test)]
pub mod tests {
    use crate::state::ProofId;

    pub const MODERATOR: &str = "moderator";
    pub const USER_A: &str = "userA";
    pub const PROOF_ID: ProofId = 1;
}
