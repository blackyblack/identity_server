use crate::{
    identity::{
        next_timestamp,
        punish::{PENALTY_VOUCHEE_WEIGHT_RATIO, penalty},
    },
    state::{IdtAmount, State, UserAddress},
};

pub const FORGET_PENALTY: IdtAmount = 500;

pub async fn forget(state: &mut State, user: UserAddress, vouchee: UserAddress) {
    let vouchee_penalty = penalty(state, &vouchee)
        .await
        .saturating_mul(PENALTY_VOUCHEE_WEIGHT_RATIO.0.into())
        .saturating_div(PENALTY_VOUCHEE_WEIGHT_RATIO.1.into());
    state.forget(
        user.clone(),
        vouchee,
        FORGET_PENALTY + vouchee_penalty,
        next_timestamp(),
    );
}

#[cfg(test)]
mod tests {
    use crate::identity::{
        idt::idt_balance,
        proof::idt_by_proof,
        punish::punish,
        tests::{MODERATOR, PROOF_ID, USER_A},
        vouch::vouch,
    };

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 10000);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 1000);
        forget(&mut state, USER_A.to_string(), user_b.to_string()).await;
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 9500);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 0);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 500);
    }

    #[async_std::test]
    async fn test_keep_penalty() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 10000);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 1000);
        punish(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            500,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 9950);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 495);
        forget(&mut state, USER_A.to_string(), user_b.to_string()).await;
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 9450);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 0);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 550);
    }

    #[async_std::test]
    async fn test_multiple_penalties() {
        let user_b = "userB";
        let user_c = "userC";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        vouch(&mut state, USER_A.to_string(), user_c.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 10000);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 1000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 1000);
        forget(&mut state, USER_A.to_string(), user_b.to_string()).await;
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 9500);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 0);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 500);
        forget(&mut state, USER_A.to_string(), user_c.to_string()).await;
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 9000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 0);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 1000);
    }

    #[async_std::test]
    async fn test_multiple_penalties_decay() {
        let user_b = "userB";
        let user_c = "userC";
        let mut state = State::default();
        let ts = next_timestamp();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        vouch(&mut state, USER_A.to_string(), user_c.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 10000);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 1000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 1000);
        // this is implementation of forget() but with overriden timestamp
        state.forget(
            USER_A.to_string(),
            user_b.to_string(),
            FORGET_PENALTY,
            ts - 86400 * 2,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 9502);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 0);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 498);
        state.forget(
            USER_A.to_string(),
            user_c.to_string(),
            FORGET_PENALTY,
            ts - 86400,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 9003);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 0);
        // penalties from forget() decay simultaneously for all forgotten users
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 997);
    }
}
