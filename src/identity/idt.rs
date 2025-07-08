use std::collections::HashMap;

use crate::{
    identity::{
        IdentityService, IdtAmount, UserAddress,
        decay::{balance_after_decay, proof_decay, vouch_decay},
        error::Error,
        punish::penalty,
        tree_walk::{ChildrenSelector, Visitor, walk_tree},
        vouch::vouchers,
    },
    numbers::Rational,
};

pub const TOP_VOUCHERS_SIZE: u16 = 5;
// voucher's balance is multiplied to this coefficient before adding to vouchee balance,
// stored as (numerator, denominator)
pub const VOUCHER_WEIGHT_RATIO: (u32, u32) = (1, 10);

struct VouchTree<'a> {
    service: &'a IdentityService,
}

impl ChildrenSelector for VouchTree<'_> {
    async fn children(&self, root: &UserAddress) -> Result<Vec<UserAddress>, Error> {
        vouchers(self.service, root).await
    }
}

async fn top_vouchers(
    service: &IdentityService,
    user: &UserAddress,
    visited: &im::HashSet<UserAddress>,
    balances: &HashMap<UserAddress, IdtAmount>,
) -> Result<Vec<(UserAddress, IdtAmount)>, Error> {
    let mut top_balances: Vec<(UserAddress, IdtAmount)> = vec![];
    for v in &vouchers(service, user).await? {
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
    top_balances.sort_by(|&(_, balance_a), &(_, balance_b)| balance_b.cmp(&balance_a));
    top_balances.truncate(TOP_VOUCHERS_SIZE.into());
    Ok(top_balances)
}

impl Visitor for VouchTree<'_> {
    async fn exit_node(
        &self,
        node: &UserAddress,
        visited_branch: &im::HashSet<UserAddress>,
        balances: &HashMap<UserAddress, IdtAmount>,
    ) -> Result<IdtAmount, Error> {
        let voucher_scale = Rational::new(VOUCHER_WEIGHT_RATIO.0, VOUCHER_WEIGHT_RATIO.1)
            .expect("VOUCHER_WEIGHT_RATIO should not be NaN");
        let proven_balance = {
            match self.service.proof(node).await? {
                // fallback to genesis balance if proof is not found
                // genesis balance does not decay but only lasts till the first proof
                None => self
                    .service
                    .genesis_balance(node)
                    .await?
                    .unwrap_or_default(),
                Some(e) => {
                    let proven_balance_decay = proof_decay(self.service, node).await?;
                    balance_after_decay(e.amount, proven_balance_decay)
                }
            }
        };

        let top_vouchers = top_vouchers(self.service, node, visited_branch, balances).await?;
        let mut balance_from_vouchers = 0;
        for (user, balance) in &top_vouchers {
            let voucher_balance_decay = vouch_decay(self.service, node, user).await?;
            let voucher_balance = voucher_scale.mul(*balance);
            balance_from_vouchers += balance_after_decay(voucher_balance, voucher_balance_decay);
        }
        let penalty = penalty(self.service, node).await?;
        let positive_balance = proven_balance + balance_from_vouchers;
        Ok(positive_balance.saturating_sub(penalty))
    }
}

pub async fn balance(service: &IdentityService, user: &UserAddress) -> Result<IdtAmount, Error> {
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
    use std::collections::HashMap;

    #[async_std::test]
    async fn test_basic() {
        let user_b = "userB";
        let service = IdentityService::default();
        prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 100);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 0);
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        // IDT of A does not change after vouching
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 100);
        // IDT of B increased
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 10);
    }

    #[async_std::test]
    async fn test_cyclic() {
        let user_b = "userB";
        let service = IdentityService::default();
        prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        )
        .await
        .unwrap();
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 100);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 10);
        vouch(&service, user_b.to_string(), USER_A.to_string())
            .await
            .unwrap();
        // cyclic vouch does not change user A balance
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 100);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 10);
    }

    #[async_std::test]
    async fn test_mutual() {
        let user_b = "userB";
        let service = IdentityService::default();
        prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            200,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 100);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 200);
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 100);
        // 200 + 0.1 * 100
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 210);
        vouch(&service, user_b.to_string(), USER_A.to_string())
            .await
            .unwrap();
        // 100 + 0.1 * 200
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 120);
        // not increased due to cyclic dependency
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 210);
    }

    #[async_std::test]
    async fn test_branches() {
        let user_b = "userB";
        let user_c = "userC";
        let user_d = "userD";
        let service = IdentityService::default();
        prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            20000,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_c.to_string(),
            MODERATOR.to_string(),
            30000,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 20000);
        assert_eq!(balance(&service, &user_c.to_string()).await.unwrap(), 30000);
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        vouch(&service, USER_A.to_string(), user_c.to_string())
            .await
            .unwrap();
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 21000);
        assert_eq!(balance(&service, &user_c.to_string()).await.unwrap(), 31000);
        vouch(&service, user_b.to_string(), user_d.to_string())
            .await
            .unwrap();
        assert_eq!(balance(&service, &user_d.to_string()).await.unwrap(), 2100);
        vouch(&service, user_c.to_string(), user_d.to_string())
            .await
            .unwrap();
        assert_eq!(balance(&service, &user_d.to_string()).await.unwrap(), 5200);
        vouch(&service, user_b.to_string(), user_c.to_string())
            .await
            .unwrap();
        assert_eq!(balance(&service, &user_c.to_string()).await.unwrap(), 33100);
        assert_eq!(balance(&service, &user_d.to_string()).await.unwrap(), 5410);
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
        prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            1000,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            2000,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_c.to_string(),
            MODERATOR.to_string(),
            3000,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_d.to_string(),
            MODERATOR.to_string(),
            4000,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_e.to_string(),
            MODERATOR.to_string(),
            5000,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_f.to_string(),
            MODERATOR.to_string(),
            6000,
            PROOF_ID,
        )
        .await
        .unwrap();
        prove(
            &service,
            user_g.to_string(),
            MODERATOR.to_string(),
            7000,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 1000);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 2000);
        assert_eq!(balance(&service, &user_c.to_string()).await.unwrap(), 3000);
        assert_eq!(balance(&service, &user_d.to_string()).await.unwrap(), 4000);
        assert_eq!(balance(&service, &user_e.to_string()).await.unwrap(), 5000);
        assert_eq!(balance(&service, &user_f.to_string()).await.unwrap(), 6000);
        assert_eq!(balance(&service, &user_g.to_string()).await.unwrap(), 7000);
        vouch(&service, user_b.to_string(), USER_A.to_string())
            .await
            .unwrap();
        // 1000 + 0.1 * 2000
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 1200);
        vouch(&service, user_c.to_string(), USER_A.to_string())
            .await
            .unwrap();
        // 1200 + 0.1 * 3000
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 1500);
        vouch(&service, user_d.to_string(), USER_A.to_string())
            .await
            .unwrap();
        // 1500 + 0.1 * 4000
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 1900);
        vouch(&service, user_e.to_string(), USER_A.to_string())
            .await
            .unwrap();
        // 1900 + 0.1 * 5000
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 2400);
        vouch(&service, user_f.to_string(), USER_A.to_string())
            .await
            .unwrap();
        // 2400 + 0.1 * 6000
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 3000);
        vouch(&service, user_g.to_string(), USER_A.to_string())
            .await
            .unwrap();
        // 3000 + 0.1 * 7000 - 0.1 * 2000
        // only 5 top vouchers are considered
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 3500);
    }

    #[async_std::test]
    async fn test_voucher_sort_order() {
        let user_a = USER_A.to_string();
        let voucher_b = "userB".to_string();
        let voucher_c = "userC".to_string();
        let voucher_d = "userD".to_string();
        let service = IdentityService::default();
        vouch(&service, voucher_b.clone(), user_a.clone())
            .await
            .unwrap();
        vouch(&service, voucher_c.clone(), user_a.clone())
            .await
            .unwrap();
        vouch(&service, voucher_d.clone(), user_a.clone())
            .await
            .unwrap();

        let mut balances = HashMap::new();
        balances.insert(voucher_b.clone(), 5);
        balances.insert(voucher_c.clone(), 10);
        balances.insert(voucher_d.clone(), 8);

        let top = top_vouchers(&service, &user_a, &im::HashSet::new(), &balances)
            .await
            .unwrap();
        assert_eq!(top.len(), 3);
        assert_eq!(top[0], (voucher_c, 10));
        assert_eq!(top[1], (voucher_d, 8));
        assert_eq!(top[2], (voucher_b, 5));
    }

    #[async_std::test]
    async fn test_decay() {
        let ts = next_timestamp();
        let user_b = "userB";
        let service = IdentityService::default();
        service
            .prove_with_timestamp(
                USER_A.to_string(),
                MODERATOR.to_string(),
                1000,
                PROOF_ID,
                ts,
            )
            .await
            .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 1000);

        service
            .prove_with_timestamp(
                USER_A.to_string(),
                MODERATOR.to_string(),
                1000,
                PROOF_ID,
                ts - 86400,
            )
            .await
            .unwrap();
        // decay after 1 day
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 999);

        service
            .prove_with_timestamp(
                USER_A.to_string(),
                MODERATOR.to_string(),
                1,
                PROOF_ID,
                ts - 86400 * 10,
            )
            .await
            .unwrap();
        // cannot go lower than 0
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 0);

        service
            .prove_with_timestamp(
                user_b.to_string(),
                MODERATOR.to_string(),
                1000,
                PROOF_ID,
                ts,
            )
            .await
            .unwrap();

        vouch(&service, user_b.to_string(), USER_A.to_string())
            .await
            .unwrap();

        // only proven balance is affected so far
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 100);

        service
            .vouch_with_timestamp(user_b.to_string(), USER_A.to_string(), ts - 86400)
            .await
            .unwrap();
        // vouch balance also decays at 1 IDT per day rate
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 99);

        service
            .prove_with_timestamp(
                user_b.to_string(),
                MODERATOR.to_string(),
                1000,
                PROOF_ID,
                ts - 86400,
            )
            .await
            .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 98);
    }
}
