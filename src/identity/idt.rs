use std::collections::HashMap;

use crate::identity::{
    IdentityService, IdtAmount, UserAddress,
    decay::{balance_after_decay, proof_decay, vouch_decay},
    punish::penalty,
    tree_walk::{ChildrenSelector, Visitor, walk_tree},
    vouch::vouchers,
};

pub const TOP_VOUCHERS_SIZE: u16 = 5;
// voucher's balance is multipled to this coefficient before adding to vouchee balance,
// stored as (nominator, denominator) to avoid floating point operations
pub const VOUCHER_WEIGHT_RATIO: (u64, u64) = (1, 10);

struct VouchTree<'a> {
    service: &'a IdentityService,
}

impl ChildrenSelector for VouchTree<'_> {
    async fn children(&self, root: &UserAddress) -> Vec<UserAddress> {
        vouchers(self.service, root)
    }
}

fn top_vouchers(
    service: &IdentityService,
    user: &UserAddress,
    visited: &im::HashSet<UserAddress>,
    balances: &HashMap<UserAddress, IdtAmount>,
) -> Vec<(UserAddress, IdtAmount)> {
    let mut top_balances: Vec<(UserAddress, IdtAmount)> = vec![];
    for v in &vouchers(service, user) {
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

impl Visitor for VouchTree<'_> {
    async fn exit_node(
        &self,
        node: &UserAddress,
        visited_branch: &im::HashSet<UserAddress>,
        balances: &HashMap<UserAddress, IdtAmount>,
    ) -> IdtAmount {
        let proven_balance = {
            let proven_balance = match self.service.proof(node) {
                None => 0,
                Some(e) => e.amount,
            };
            let proven_balance_decay = proof_decay(self.service, node);
            balance_after_decay(proven_balance, proven_balance_decay)
        };

        let top_vouchers = top_vouchers(self.service, node, visited_branch, balances);
        let balance_from_vouchers = top_vouchers.into_iter().fold(0, |acc, (user, b)| {
            let voucher_balance_decay = vouch_decay(self.service, node, &user);
            let voucher_balance = b
                .saturating_mul(VOUCHER_WEIGHT_RATIO.0.into())
                .saturating_div(VOUCHER_WEIGHT_RATIO.1.into());
            acc + balance_after_decay(voucher_balance, voucher_balance_decay)
        });
        let penalty = penalty(self.service, node).await;
        let positive_balance = proven_balance + balance_from_vouchers;
        positive_balance.saturating_sub(penalty)
    }
}

pub async fn balance(service: &IdentityService, user: &UserAddress) -> IdtAmount {
    let tree = VouchTree { service };
    walk_tree(&tree, user).await
}

#[cfg(test)]
mod tests {
    use crate::identity::{
        next_timestamp,
        proof::prove,
        tests::{MODERATOR, PROOF_ID, USER_A},
        vouch::vouch,
    };

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let user_b = "userB";
        let service = IdentityService::default();
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        assert_eq!(balance(&service, &USER_A.to_string()).await, 100);
        assert_eq!(balance(&service, &user_b.to_string()).await, 0);
        vouch(&service, USER_A.to_string(), user_b.to_string());
        // IDT of A does not change after vouching
        assert_eq!(balance(&service, &USER_A.to_string()).await, 100);
        // IDT of B increased
        assert_eq!(balance(&service, &user_b.to_string()).await, 10);
    }

    #[async_std::test]
    async fn test_cyclic() {
        let user_b = "userB";
        let service = IdentityService::default();
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        vouch(&service, USER_A.to_string(), user_b.to_string());
        assert_eq!(balance(&service, &USER_A.to_string()).await, 100);
        assert_eq!(balance(&service, &user_b.to_string()).await, 10);
        vouch(&service, user_b.to_string(), USER_A.to_string());
        // cyclic vouch does not change user A balance
        assert_eq!(balance(&service, &USER_A.to_string()).await, 100);
        assert_eq!(balance(&service, &user_b.to_string()).await, 10);
    }

    #[async_std::test]
    async fn test_mutual() {
        let user_b = "userB";
        let service = IdentityService::default();
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            200,
            PROOF_ID,
        );
        assert_eq!(balance(&service, &USER_A.to_string()).await, 100);
        assert_eq!(balance(&service, &user_b.to_string()).await, 200);
        vouch(&service, USER_A.to_string(), user_b.to_string());
        assert_eq!(balance(&service, &USER_A.to_string()).await, 100);
        // 200 + 0.1 * 100
        assert_eq!(balance(&service, &user_b.to_string()).await, 210);
        vouch(&service, user_b.to_string(), USER_A.to_string());
        // 100 + 0.1 * 200
        assert_eq!(balance(&service, &USER_A.to_string()).await, 120);
        // not increased due to cyclic dependency
        assert_eq!(balance(&service, &user_b.to_string()).await, 210);
    }

    #[async_std::test]
    async fn test_branches() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let service = IdentityService::default();
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            20000,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_c.to_string(),
            MODERATOR.to_string(),
            30000,
            PROOF_ID,
        );
        assert_eq!(balance(&service, &USER_A.to_string()).await, 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await, 20000);
        assert_eq!(balance(&service, &user_c.to_string()).await, 30000);
        vouch(&service, USER_A.to_string(), user_b.to_string());
        vouch(&service, USER_A.to_string(), user_c.to_string());
        assert_eq!(balance(&service, &user_b.to_string()).await, 21000);
        assert_eq!(balance(&service, &user_c.to_string()).await, 31000);
        vouch(&service, user_b.to_string(), user_d.to_string());
        assert_eq!(balance(&service, &user_d.to_string()).await, 2100);
        vouch(&service, user_c.to_string(), user_d.to_string());
        assert_eq!(balance(&service, &user_d.to_string()).await, 5200);
        vouch(&service, user_b.to_string(), user_c.to_string());
        assert_eq!(balance(&service, &user_c.to_string()).await, 33100);
        assert_eq!(balance(&service, &user_d.to_string()).await, 5410);
    }

    #[async_std::test]
    async fn test_max_vouchers() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let user_e = "userE";
        let user_f = "userF";
        let user_g = "userG";
        let service = IdentityService::default();
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            2000,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_c.to_string(),
            MODERATOR.to_string(),
            3000,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_d.to_string(),
            MODERATOR.to_string(),
            4000,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_e.to_string(),
            MODERATOR.to_string(),
            5000,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_f.to_string(),
            MODERATOR.to_string(),
            6000,
            PROOF_ID,
        );
        let _ = prove(
            &service,
            user_g.to_string(),
            MODERATOR.to_string(),
            7000,
            PROOF_ID,
        );
        assert_eq!(balance(&service, &USER_A.to_string()).await, 1000);
        assert_eq!(balance(&service, &user_b.to_string()).await, 2000);
        assert_eq!(balance(&service, &user_c.to_string()).await, 3000);
        assert_eq!(balance(&service, &user_d.to_string()).await, 4000);
        assert_eq!(balance(&service, &user_e.to_string()).await, 5000);
        assert_eq!(balance(&service, &user_f.to_string()).await, 6000);
        assert_eq!(balance(&service, &user_g.to_string()).await, 7000);
        vouch(&service, user_b.to_string(), USER_A.to_string());
        // 1000 + 0.1 * 2000
        assert_eq!(balance(&service, &USER_A.to_string()).await, 1200);
        vouch(&service, user_c.to_string(), USER_A.to_string());
        // 1200 + 0.1 * 3000
        assert_eq!(balance(&service, &USER_A.to_string()).await, 1500);
        vouch(&service, user_d.to_string(), USER_A.to_string());
        // 1500 + 0.1 * 4000
        assert_eq!(balance(&service, &USER_A.to_string()).await, 1900);
        vouch(&service, user_e.to_string(), USER_A.to_string());
        // 1900 + 0.1 * 5000
        assert_eq!(balance(&service, &USER_A.to_string()).await, 2400);
        vouch(&service, user_f.to_string(), USER_A.to_string());
        // 2400 + 0.1 * 6000
        assert_eq!(balance(&service, &USER_A.to_string()).await, 3000);
        vouch(&service, user_g.to_string(), USER_A.to_string());
        // 3000 + 0.1 * 7000 - 0.1 * 2000
        // only 5 top vouchers are considered
        assert_eq!(balance(&service, &USER_A.to_string()).await, 3500);
    }

    #[async_std::test]
    async fn test_decay() {
        let ts = next_timestamp();
        let user_b = "userB";
        let service = IdentityService::default();
        let _ = service.prove_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
            ts,
        );
        assert_eq!(balance(&service, &USER_A.to_string()).await, 1000);

        let _ = service.prove_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
            ts - 86400,
        );
        // decay after 1 day
        assert_eq!(balance(&service, &USER_A.to_string()).await, 999);

        let _ = service.prove_with_timestamp(
            USER_A.to_string(),
            MODERATOR.to_string(),
            1,
            PROOF_ID,
            ts - 86400 * 10,
        );
        // cannot go lower than 0
        assert_eq!(balance(&service, &USER_A.to_string()).await, 0);

        let _ = service.prove_with_timestamp(
            user_b.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
            ts,
        );

        vouch(&service, user_b.to_string(), USER_A.to_string());

        // only proven balance is affected so far
        assert_eq!(balance(&service, &USER_A.to_string()).await, 100);

        service.vouch_with_timestamp(user_b.to_string(), USER_A.to_string(), ts - 86400);
        // vouch balance also decays at 1 IDT per day rate
        assert_eq!(balance(&service, &USER_A.to_string()).await, 99);

        let _ = service.prove_with_timestamp(
            user_b.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
            ts - 86400,
        );
        assert_eq!(balance(&service, &USER_A.to_string()).await, 98);
    }
}
