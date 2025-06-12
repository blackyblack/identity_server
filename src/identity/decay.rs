use crate::{identity::next_timestamp, state::{IdtAmount, State, UserAddress}};

fn flat_one_idt_decay(event_timestamp: u64) -> IdtAmount {
    let now = next_timestamp();
    // future timestamp, should not happen
    if now < event_timestamp {
        return 0;
    }
    // decay is 1 IDT per day
    ((now - event_timestamp) as u128) / 60 / 60 / 24
}

pub fn proof_decay(state: &State, user: &UserAddress) -> IdtAmount {
    let (timestamp, _balance) = match state.proof_event(user) {
        None => return 0,
        Some(e) => (e.timestamp, e.idt_balance),
    };
    flat_one_idt_decay(timestamp)
}

pub fn penalty_decay(state: &State, user: &UserAddress) -> IdtAmount {
    let (timestamp, _penalty) = match state.penalty_event(user) {
        None => return 0,
        Some(e) => (e.timestamp, e.idt_balance),
    };
    flat_one_idt_decay(timestamp)
}

pub fn vouch_decay(state: &State, user: &UserAddress, voucher: &UserAddress) -> IdtAmount {
    let timestamp = match state.voucher_timestamp(user, voucher) {
        None => return 0,
        Some(e) => e,
    };
    flat_one_idt_decay(timestamp)
}

#[cfg(test)]
mod tests {
    use crate::identity::tests::{MODERATOR, PROOF_ID, USER_A};

    use super::*;

    #[test]
    fn test_basic_proof_decay() {
        let mut state = State::default();
        let ts = next_timestamp();
        assert_eq!(proof_decay(&state, &USER_A.to_string()), 0);
        state.prove(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts);
        assert_eq!(proof_decay(&state, &USER_A.to_string()), 0);
        state.prove(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts - 86400);
        assert_eq!(proof_decay(&state, &USER_A.to_string()), 1);
        state.prove(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts - 100000);
        assert_eq!(proof_decay(&state, &USER_A.to_string()), 1);
        state.prove(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts - 86400 * 2);
        assert_eq!(proof_decay(&state, &USER_A.to_string()), 2);
    }

    #[test]
    fn test_basic_penalty_decay() {
        let mut state = State::default();
        let ts = next_timestamp();
        assert_eq!(penalty_decay(&state, &USER_A.to_string()), 0);
        state.punish(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts);
        assert_eq!(penalty_decay(&state, &USER_A.to_string()), 0);
        state.punish(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts - 86400);
        assert_eq!(penalty_decay(&state, &USER_A.to_string()), 1);
        state.punish(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts - 100000);
        assert_eq!(penalty_decay(&state, &USER_A.to_string()), 1);
        state.punish(USER_A.to_string(), MODERATOR.to_string(), 100, PROOF_ID, ts - 86400 * 2);
        assert_eq!(penalty_decay(&state, &USER_A.to_string()), 2);
    }

    #[test]
    fn test_basic_vouch_decay() {
        let user_b = "userB";
        let mut state = State::default();
        let ts = next_timestamp();
        assert_eq!(vouch_decay(&state, &USER_A.to_string(), &user_b.to_string()), 0);
        state.vouch(user_b.to_string(), USER_A.to_string(), ts);
        assert_eq!(vouch_decay(&state, &USER_A.to_string(), &user_b.to_string()), 0);
        state.vouch(user_b.to_string(), USER_A.to_string(), ts - 86400);
        assert_eq!(vouch_decay(&state, &USER_A.to_string(), &user_b.to_string()), 1);
        state.vouch(user_b.to_string(), USER_A.to_string(), ts - 100000);
        assert_eq!(vouch_decay(&state, &USER_A.to_string(), &user_b.to_string()), 1);
        state.vouch(user_b.to_string(), USER_A.to_string(), ts - 86400 * 2);
        assert_eq!(vouch_decay(&state, &USER_A.to_string(), &user_b.to_string()), 2);
    }
}
