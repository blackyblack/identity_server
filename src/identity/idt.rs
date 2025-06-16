use std::collections::HashMap;

use crate::{
    identity::{
        decay::{balance_after_decay, proof_decay, vouch_decay},
        punish::penalty,
        tree_walk::{ChildrenSelector, Visitor, walk_tree},
    },
    state::{IdtAmount, State, UserAddress},
};

pub const TOP_VOUCHERS_SIZE: u16 = 5;
// voucher's balance is multipled to this coefficient before adding to vouchee balance,
// stored as (nominator, denominator) to avoid floating point operations
pub const VOUCHER_WEIGHT_RATIO: (u64, u64) = (1, 10);

struct VouchTree(State);

impl ChildrenSelector for VouchTree {
    async fn children(&self, root: &UserAddress) -> Vec<UserAddress> {
        self.0.vouchers(root)
    }
}

fn top_vouchers(
    state: &State,
    user: &UserAddress,
    visited: &im::HashSet<UserAddress>,
    balances: &HashMap<UserAddress, IdtAmount>,
) -> Vec<(UserAddress, IdtAmount)> {
    let mut top_balances: Vec<(UserAddress, IdtAmount)> = vec![];
    for v in &state.vouchers(user) {
        if visited.contains(v) {
            continue;
        }
        // child balance could be missing due to cyclic dependency.
        let voucher_balance = match balances.get(v) {
            None => continue,
            Some(x) => *x,
        };
        top_balances.push((v.clone(), voucher_balance));
    }
    top_balances.sort();
    top_balances.reverse();
    top_balances.truncate(TOP_VOUCHERS_SIZE.into());
    top_balances
}

impl Visitor for VouchTree {
    async fn exit_node(
        &self,
        node: &UserAddress,
        visited_branch: &im::HashSet<UserAddress>,
        balances: &HashMap<UserAddress, IdtAmount>,
    ) -> IdtAmount {
        let proven_balance = {
            let proven_balance = match self.0.proof(node) {
                None => 0,
                Some(e) => e.idt_balance,
            };
            let proven_balance_decay = proof_decay(&self.0, node);
            balance_after_decay(proven_balance, proven_balance_decay)
        };

        let top_vouchers = top_vouchers(&self.0, node, visited_branch, balances);
        let balance_from_vouchers = top_vouchers.into_iter().fold(0, |acc, (user, b)| {
            let voucher_balance_decay = vouch_decay(&self.0, node, &user);
            let voucher_balance = b
                .saturating_mul(VOUCHER_WEIGHT_RATIO.0.into())
                .saturating_div(VOUCHER_WEIGHT_RATIO.1.into());
            acc + balance_after_decay(voucher_balance, voucher_balance_decay)
        });
        let penalty = penalty(&self.0, node).await;
        let positive_balance = proven_balance + balance_from_vouchers;
        positive_balance.saturating_sub(penalty)
    }
}

pub async fn idt_balance(state: &State, user: &UserAddress) -> IdtAmount {
    let tree = VouchTree(state.clone());
    walk_tree(&tree, user).await
}

#[cfg(test)]
mod tests {
    use crate::identity::{
        next_timestamp,
        proof::idt_by_proof,
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
            100,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 0);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        // IDT of A does not change after vouching
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        // IDT of B increased
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 10);
    }

    #[async_std::test]
    async fn test_cyclic() {
        let user_b = "userB";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 10);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // cyclic vouch does not change user A balance
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 10);
    }

    #[async_std::test]
    async fn test_mutual() {
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
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 200);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);
        // 200 + 0.1 * 100
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 210);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // 100 + 0.1 * 200
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 120);
        // not increased due to cyclic dependency
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 210);
    }

    #[async_std::test]
    async fn test_branches() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            20000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_c.to_string(),
            MODERATOR.to_string(),
            30000,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 10000);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 20000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 30000);
        vouch(&mut state, USER_A.to_string(), user_b.to_string());
        vouch(&mut state, USER_A.to_string(), user_c.to_string());
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 21000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 31000);
        vouch(&mut state, user_b.to_string(), user_d.to_string());
        assert_eq!(idt_balance(&state, &user_d.to_string()).await, 2100);
        vouch(&mut state, user_c.to_string(), user_d.to_string());
        assert_eq!(idt_balance(&state, &user_d.to_string()).await, 5200);
        vouch(&mut state, user_b.to_string(), user_c.to_string());
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 33100);
        assert_eq!(idt_balance(&state, &user_d.to_string()).await, 5410);
    }

    #[async_std::test]
    async fn test_max_vouchers() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let user_e = "userE";
        let user_f = "userF";
        let user_g = "userG";
        let mut state = State::default();
        let _ = idt_by_proof(
            &mut state,
            USER_A.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_b.to_string(),
            MODERATOR.to_string(),
            2000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_c.to_string(),
            MODERATOR.to_string(),
            3000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_d.to_string(),
            MODERATOR.to_string(),
            4000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_e.to_string(),
            MODERATOR.to_string(),
            5000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_f.to_string(),
            MODERATOR.to_string(),
            6000,
            PROOF_ID,
        );
        let _ = idt_by_proof(
            &mut state,
            user_g.to_string(),
            MODERATOR.to_string(),
            7000,
            PROOF_ID,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1000);
        assert_eq!(idt_balance(&state, &user_b.to_string()).await, 2000);
        assert_eq!(idt_balance(&state, &user_c.to_string()).await, 3000);
        assert_eq!(idt_balance(&state, &user_d.to_string()).await, 4000);
        assert_eq!(idt_balance(&state, &user_e.to_string()).await, 5000);
        assert_eq!(idt_balance(&state, &user_f.to_string()).await, 6000);
        assert_eq!(idt_balance(&state, &user_g.to_string()).await, 7000);
        vouch(&mut state, user_b.to_string(), USER_A.to_string());
        // 1000 + 0.1 * 2000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1200);
        vouch(&mut state, user_c.to_string(), USER_A.to_string());
        // 1200 + 0.1 * 3000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1500);
        vouch(&mut state, user_d.to_string(), USER_A.to_string());
        // 1500 + 0.1 * 4000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1900);
        vouch(&mut state, user_e.to_string(), USER_A.to_string());
        // 1900 + 0.1 * 5000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 2400);
        vouch(&mut state, user_f.to_string(), USER_A.to_string());
        // 2400 + 0.1 * 6000
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 3000);
        vouch(&mut state, user_g.to_string(), USER_A.to_string());
        // 3000 + 0.1 * 7000 - 0.1 * 2000
        // only 5 top vouchers are considered
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 3500);
    }

    #[async_std::test]
    async fn test_decay() {
        let ts = next_timestamp();
        let user_b = "userB";
        let mut state = State::default();
        state.prove(
            USER_A.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
            ts,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 1000);

        state.prove(
            USER_A.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
            ts - 86400,
        );
        // decay after 1 day
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 999);

        state.prove(
            USER_A.to_string(),
            MODERATOR.to_string(),
            1,
            PROOF_ID,
            ts - 86400 * 10,
        );
        // cannot go lower than 0
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 0);

        state.prove(
            user_b.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
            ts,
        );

        vouch(&mut state, user_b.to_string(), USER_A.to_string());

        // only proven balance is affected so far
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 100);

        state.vouch(user_b.to_string(), USER_A.to_string(), ts - 86400);
        // vouch balance also decays at 1 IDT per day rate
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 99);

        state.prove(
            user_b.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
            ts - 86400,
        );
        assert_eq!(idt_balance(&state, &USER_A.to_string()).await, 98);
    }
}
