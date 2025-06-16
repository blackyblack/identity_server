use std::collections::HashMap;

use crate::{
    identity::{
        decay::{balance_after_decay, moderator_penalty_decay, system_penalty_decay},
        next_timestamp,
        proof::MAX_IDT_BY_PROOF,
        tree_walk::{ChildrenSelector, Visitor, walk_tree},
    },
    state::{IdtAmount, ProofId, State, UserAddress},
};

// allows to ban for twice the entire balance, i.e. permanent ban. However due to penalty decay
// IDT balance can eventually become positive.
// It only limits vouchee penalty because we do not want to limit amount of penalties and their value
// for a single user but we do not want to propagate it across the network indefinitely.
pub const MAX_VOUCHEE_PENALTY: IdtAmount = MAX_IDT_BY_PROOF * 2;
// vouchee's penalty is multipled to this coefficient before adding to voucher penalty
// stored as (nominator, denominator) to avoid floating point operations
pub const PENALTY_VOUCHEE_WEIGHT_RATIO: (u64, u64) = (1, 10);

struct PenaltyTree(State);

impl ChildrenSelector for PenaltyTree {
    async fn children(&self, root: &UserAddress) -> Vec<UserAddress> {
        self.0.vouchees(root)
    }
}

fn penalty_from_vouchees(
    state: &State,
    user: &UserAddress,
    visited: &im::HashSet<UserAddress>,
    penalties: &HashMap<UserAddress, IdtAmount>,
) -> IdtAmount {
    let mut penalty: IdtAmount = 0;
    for v in &state.vouchees(user) {
        if visited.contains(v) {
            continue;
        }
        // child penalty could be missing due to cyclic dependency.
        let vouchee_penalty = match penalties.get(v) {
            None => continue,
            Some(x) => x,
        };
        let vouchee_penalty_limited = if *vouchee_penalty > MAX_VOUCHEE_PENALTY {
            MAX_VOUCHEE_PENALTY
        } else {
            *vouchee_penalty
        };
        penalty += vouchee_penalty_limited;
    }
    penalty
        .saturating_mul(PENALTY_VOUCHEE_WEIGHT_RATIO.0.into())
        .saturating_div(PENALTY_VOUCHEE_WEIGHT_RATIO.1.into())
}

impl Visitor for PenaltyTree {
    async fn exit_node(
        &self,
        node: &UserAddress,
        visited_branch: &im::HashSet<UserAddress>,
        balances: &HashMap<UserAddress, IdtAmount>,
    ) -> IdtAmount {
        let proven_penalty = {
            let proven_penalty = match self.0.moderator_penalty(node) {
                None => 0,
                Some(e) => e.idt_balance,
            };
            let proven_penalty_decay = moderator_penalty_decay(&self.0, node);
            balance_after_decay(proven_penalty, proven_penalty_decay)
        };
        let system_penalty = {
            let system_penalty = match self.0.system_penalty(node) {
                None => 0,
                Some(e) => e.idt_balance,
            };
            let system_penalty_decay = system_penalty_decay(&self.0, node);
            balance_after_decay(system_penalty, system_penalty_decay)
        };
        let vouchees_penalty = penalty_from_vouchees(&self.0, node, visited_branch, balances);
        proven_penalty + system_penalty + vouchees_penalty
    }
}

pub fn punish(
    state: &mut State,
    user: UserAddress,
    moderator: UserAddress,
    penalty: IdtAmount,
    proof_id: ProofId,
) {
    // no balance check on penalty, up to the moderator to decide
    state.punish(user, moderator, penalty, proof_id, next_timestamp());
}

pub async fn penalty(state: &State, user: &UserAddress) -> IdtAmount {
    let tree = PenaltyTree(state.clone());
    walk_tree(&tree, user).await
}

#[cfg(test)]
mod tests {
    use crate::identity::{
        idt::idt_balance,
        proof::idt_by_proof,
        tests::{MODERATOR, PROOF_ID, USER_A},
        vouch::vouch,
    };

    use super::*;

    #[test]
    fn test_basic() {
        let mut state = State::default();
        assert!(state.moderator_penalty(&USER_A.to_string()).is_none());
        punish(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        assert!(state.moderator_penalty(&USER_A.to_string()).is_some());
        assert_eq!(
            state
                .moderator_penalty(&USER_A.to_string())
                .unwrap()
                .idt_balance,
            100
        );
        assert_eq!(
            state
                .moderator_penalty(&USER_A.to_string())
                .unwrap()
                .moderator,
            MODERATOR
        );
        assert_eq!(
            state
                .moderator_penalty(&USER_A.to_string())
                .unwrap()
                .proof_id,
            PROOF_ID
        );
        assert!(
            state
                .moderator_penalty(&USER_A.to_string())
                .unwrap()
                .timestamp
                > 0
        );
    }

    #[async_std::test]
    async fn test_penalty() {
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        punish(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            50,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 50);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 50);
    }

    #[async_std::test]
    async fn test_max_penalty() {
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        punish(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            200,
            PROOF_ID,
        );
        // cannot go lower than 0
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 0);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 200);
    }

    #[async_std::test]
    async fn test_penalty_for_voucher() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            200,
            PROOF_ID,
        );
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 210);
        punish(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            50,
            PROOF_ID,
        );
        // 100 - 0.1 * 50
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 95);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 5);
        // penalty affects vouchee twice:
        // first, from the direct punishment
        // second, from the voucher reduced balance from the vouchee penalty
        // 200 - 50 + 0.1 * 95
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 159);
        assert_eq!(penalty(&state, &user_b.to_string()).await, 50);
    }

    #[async_std::test]
    async fn test_max_penalty_from_vouchees() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            200,
            PROOF_ID,
        );
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 210);
        punish(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            50000,
            PROOF_ID,
        );
        assert_eq!(penalty(&state, &user_b.to_string()).await, 50000);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 5000);
        // balance is zero due to very high penalty
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 0);
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 0);
        punish(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            100000,
            PROOF_ID,
        );
        assert_eq!(penalty(&state, &user_b.to_string()).await, 100000);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 10000);
        punish(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            150000,
            PROOF_ID,
        );
        // penalty for a user is unlimited, but
        // penalty from vouchees is limited to 2 * MAX_IDT_BY_PROOF
        assert_eq!(penalty(&state, &user_b.to_string()).await, 150000);
        assert_eq!(penalty(&state, &USER_A.to_string()).await, 10000);
    }
}
