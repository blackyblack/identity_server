use crate::{
    identity::next_timestamp,
    state::{State, UserAddress},
};

pub fn vouch(state: &mut State, from: UserAddress, to: UserAddress) {
    state.vouch(from, to, next_timestamp());
}

#[cfg(test)]
mod tests {
    use crate::identity::tests::USER_A;

    use super::*;

    #[test]
    fn test_basic() {
        let mut state = State::default();
        let user_b = "userB";
        assert!(state.vouchees(&USER_A.to_string()).len() == 0);
        assert!(state.vouchers(&USER_A.to_string()).len() == 0);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert!(state.vouchees(&USER_A.to_string()).len() == 1);
        assert!(state.vouchers(&USER_A.to_string()).len() == 0);
        assert!(state.vouchees(&user_b.to_string()).len() == 0);
        assert!(state.vouchers(&user_b.to_string()).len() == 1);
    }

    #[test]
    fn test_vouch_self() {
        let mut state = State::default();
        assert!(state.vouchees(&USER_A.to_string()).len() == 0);
        assert!(state.vouchers(&USER_A.to_string()).len() == 0);
        // user can vouch for himself
        vouch(&mut state, USER_A.to_string(), USER_A.to_string());
        assert!(state.vouchees(&USER_A.to_string()).len() == 1);
        assert!(state.vouchers(&USER_A.to_string()).len() == 1);
    }

    #[test]
    fn test_vouch_twice() {
        let mut state = State::default();
        let user_b = "userB";
        assert!(state.vouchees(&USER_A.to_string()).len() == 0);
        assert!(state.vouchers(&USER_A.to_string()).len() == 0);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert!(state.vouchees(&USER_A.to_string()).len() == 1);
        assert!(state.vouchers(&USER_A.to_string()).len() == 0);
        assert!(state.vouchees(&user_b.to_string()).len() == 0);
        assert!(state.vouchers(&user_b.to_string()).len() == 1);
        // duplicate vouch does not change anything
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert!(state.vouchees(&USER_A.to_string()).len() == 1);
        assert!(state.vouchers(&USER_A.to_string()).len() == 0);
        assert!(state.vouchees(&user_b.to_string()).len() == 0);
        assert!(state.vouchers(&user_b.to_string()).len() == 1);
    }

    #[test]
    fn test_vouch_mutual() {
        let mut state = State::default();
        let user_b = "userB";
        assert!(state.vouchees(&USER_A.to_string()).len() == 0);
        assert!(state.vouchers(&USER_A.to_string()).len() == 0);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert!(state.vouchees(&USER_A.to_string()).len() == 1);
        assert!(state.vouchers(&USER_A.to_string()).len() == 0);
        assert!(state.vouchees(&user_b.to_string()).len() == 0);
        assert!(state.vouchers(&user_b.to_string()).len() == 1);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        assert!(state.vouchees(&USER_A.to_string()).len() == 1);
        assert!(state.vouchers(&USER_A.to_string()).len() == 1);
        assert!(state.vouchees(&user_b.to_string()).len() == 1);
        assert!(state.vouchers(&user_b.to_string()).len() == 1);
    }
}
