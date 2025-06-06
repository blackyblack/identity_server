use crate::{identity::error::Error, state::{IdtAmount, ProofId, State, UserAddress}};

pub const MAX_IDT_BY_PROOF: u64 = 50000;

pub fn idt_by_proof(state: &mut State, user: UserAddress, moderator: UserAddress, balance: IdtAmount, proof_id: ProofId) -> Result<(), Error> {
    if balance > MAX_IDT_BY_PROOF.into() {
        return Err(Error::MaxBalanceExceeded);
    }
    state.prove(user, moderator, balance, proof_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::identity::tests::{MODERATOR, PROOF_ID, USER_A};

    use super::*;

    #[test]
    fn test_basic() {
        let mut state = State::default();
        assert!(state.proof_event(&USER_A.to_string()).is_none());
        assert!(idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID).is_ok());
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().idt_balance, 100);
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().moderator, MODERATOR);
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().proof_id, PROOF_ID);
        assert!(state.proof_event(&USER_A.to_string()).unwrap().timestamp > 0);

        assert!(idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 200, 2).is_ok());
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().idt_balance, 200);
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().moderator, MODERATOR);
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().proof_id, 2);
        // we do not compare with previous timestamp because it is measured in seconds and
        // test runs way much faster
        assert!(state.proof_event(&USER_A.to_string()).unwrap().timestamp > 0);
    }

    #[test]
    fn test_max_balance() {
        let mut state = State::default();
        assert!(state.proof_event(&USER_A.to_string()).is_none());
        assert!(idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 40000, PROOF_ID).is_ok());
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().idt_balance, 40000);
        assert!(idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 50001, PROOF_ID).is_err());
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().idt_balance, 40000);
        assert!(idt_by_proof(&mut state, USER_A.to_string(), MODERATOR.to_string(), 60000, PROOF_ID).is_err());
        assert_eq!(state.proof_event(&USER_A.to_string()).unwrap().idt_balance, 40000);
    }
}
