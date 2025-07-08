use std::collections::{HashMap, HashSet};

use crate::{
    identity::{
        IdentityService, IdtAmount, ModeratorProof, ProofId, SystemPenalty, UserAddress,
        decay::{balance_after_decay, moderator_penalty_decay, system_penalty_decay},
        error::Error,
        next_timestamp,
        proof::MAX_IDT_BY_PROOF,
        tree_walk::{ChildrenSelector, Visitor, walk_tree},
        vouch::vouchees,
    },
    numbers::Rational,
};

pub mod db;
pub mod storage;

// allows to ban for twice the entire balance, i.e. permanent ban. However due to penalty decay
// IDT balance can eventually become positive.
// It only limits vouchee penalty because we do not want to limit amount of penalties and their value
// for a single user but we do not want to propagate it across the network indefinitely.
pub const MAX_VOUCHEE_PENALTY: IdtAmount = MAX_IDT_BY_PROOF * 2;
// vouchee's penalty is multiplied to this coefficient before adding to voucher penalty
// stored as (numerator, denominator)
pub const PENALTY_VOUCHEE_WEIGHT_RATIO: (u32, u32) = (1, 10);
pub const FORGET_PENALTY: IdtAmount = 500;

struct PenaltyTree<'a> {
    service: &'a IdentityService,
}

impl ChildrenSelector for PenaltyTree<'_> {
    async fn children(&self, root: &UserAddress) -> Result<Vec<UserAddress>, Error> {
        vouchees(self.service, root).await
    }
}

async fn penalty_from_vouchees(
    service: &IdentityService,
    user: &UserAddress,
    visited: &im::HashSet<UserAddress>,
    penalties: &HashMap<UserAddress, IdtAmount>,
) -> Result<IdtAmount, Error> {
    let mut penalty: IdtAmount = 0;
    for v in &vouchees(service, user).await? {
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
    let penalty_scale = Rational::new(
        PENALTY_VOUCHEE_WEIGHT_RATIO.0,
        PENALTY_VOUCHEE_WEIGHT_RATIO.1,
    )
    .expect("PENALTY_VOUCHEE_WEIGHT_RATIO denominator must not be zero");
    Ok(penalty_scale.mul(penalty))
}

async fn vouchee_penalty(
    service: &IdentityService,
    user: &UserAddress,
    vouchee: &UserAddress,
) -> Result<IdtAmount, Error> {
    let vouchee_penalty_maybe = service.forgotten_penalty(user, vouchee).await?;
    let vouchee_penalty = match vouchee_penalty_maybe {
        None => return Ok(0),
        Some(p) => p,
    };
    let decay = system_penalty_decay(&vouchee_penalty);
    let result_penalty = balance_after_decay(vouchee_penalty.amount, decay);
    // cleanup outdated penalties
    if result_penalty == 0 {
        service.delete_forgotten(user.clone(), vouchee).await?;
        return Ok(0);
    }
    Ok(result_penalty)
}

async fn forgotten_penalties_sum(
    service: &IdentityService,
    user: &UserAddress,
    vouchees: &HashSet<UserAddress>,
) -> Result<IdtAmount, Error> {
    let mut penalty = 0;
    for vouchee in vouchees {
        let vouchee_penalty = vouchee_penalty(service, user, vouchee).await?;
        penalty += vouchee_penalty;
    }
    Ok(penalty)
}

impl Visitor for PenaltyTree<'_> {
    async fn exit_node(
        &self,
        node: &UserAddress,
        visited_branch: &im::HashSet<UserAddress>,
        balances: &HashMap<UserAddress, IdtAmount>,
    ) -> Result<IdtAmount, Error> {
        let proven_penalty = {
            let proven_penalty = match self.service.moderator_penalty(node).await? {
                None => 0,
                Some(e) => e.amount,
            };
            let proven_penalty_decay = moderator_penalty_decay(self.service, node).await?;
            balance_after_decay(proven_penalty, proven_penalty_decay)
        };
        let vouchees = self.service.forgotten_users(node).await?;
        let system_penalty = forgotten_penalties_sum(self.service, node, &vouchees).await?;
        let vouchees_penalty =
            penalty_from_vouchees(self.service, node, visited_branch, balances).await?;
        Ok(proven_penalty + system_penalty + vouchees_penalty)
    }
}

impl IdentityService {
    pub async fn punish_with_timestamp(
        &self,
        user: UserAddress,
        moderator: UserAddress,
        balance: IdtAmount,
        proof_id: ProofId,
        timestamp: u64,
    ) -> Result<(), Error> {
        let event = ModeratorProof {
            moderator,
            amount: balance,
            proof_id,
            timestamp,
        };
        self.penalties.set_moderator_penalty(user, event).await
    }

    pub async fn punish_for_forgetting_with_timestamp(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        let vouchee_penalty = {
            let penalty_scale = Rational::new(
                PENALTY_VOUCHEE_WEIGHT_RATIO.0,
                PENALTY_VOUCHEE_WEIGHT_RATIO.1,
            )
            .expect("PENALTY_VOUCHEE_WEIGHT_RATIO denominator must not be zero");
            let vouchee_penalty = penalty(self, &vouchee).await?;
            penalty_scale.mul(vouchee_penalty)
        };
        let event = SystemPenalty {
            amount: FORGET_PENALTY + vouchee_penalty,
            timestamp,
        };
        self.penalties
            .set_forgotten_penalty(user, vouchee, event)
            .await
    }

    pub async fn moderator_penalty(
        &self,
        user: &UserAddress,
    ) -> Result<Option<ModeratorProof>, Error> {
        self.penalties.moderator_penalty(user).await
    }

    pub async fn forgotten_penalty(
        &self,
        user: &UserAddress,
        forgotten: &UserAddress,
    ) -> Result<Option<SystemPenalty>, Error> {
        self.penalties.forgotten_penalty(user, forgotten).await
    }
}

pub async fn punish(
    service: &IdentityService,
    user: UserAddress,
    moderator: UserAddress,
    balance: IdtAmount,
    proof_id: ProofId,
) -> Result<(), Error> {
    service
        .punish_with_timestamp(user, moderator, balance, proof_id, next_timestamp())
        .await
}

pub async fn punish_for_forgetting(
    service: &IdentityService,
    user: UserAddress,
    vouchee: UserAddress,
) -> Result<(), Error> {
    service
        .punish_for_forgetting_with_timestamp(user, vouchee, next_timestamp())
        .await
}

pub async fn penalty(service: &IdentityService, user: &UserAddress) -> Result<IdtAmount, Error> {
    let tree = PenaltyTree { service };
    walk_tree(&tree, user).await
}

#[cfg(test)]
mod tests {
    use crate::identity::{
        IdentityService,
        idt::balance,
        next_timestamp,
        proof::prove,
        punish::{penalty, punish},
        tests::{MODERATOR, PROOF_ID, USER_A},
        vouch::vouch,
    };

    #[async_std::test]
    async fn test_basic() {
        let service = IdentityService::default();
        assert!(
            service
                .moderator_penalty(&USER_A.to_string())
                .await
                .unwrap()
                .is_none()
        );
        punish(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            100,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert!(
            service
                .moderator_penalty(&USER_A.to_string())
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            service
                .moderator_penalty(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .amount,
            100
        );
        assert_eq!(
            service
                .moderator_penalty(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .moderator,
            MODERATOR
        );
        assert_eq!(
            service
                .moderator_penalty(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .proof_id,
            PROOF_ID
        );
        assert!(
            service
                .moderator_penalty(&USER_A.to_string())
                .await
                .unwrap()
                .unwrap()
                .timestamp
                > 0
        );
    }

    #[async_std::test]
    async fn test_penalty() {
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
        punish(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            50,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 50);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 50);
    }

    #[async_std::test]
    async fn test_max_penalty() {
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
        punish(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            200,
            PROOF_ID,
        )
        .await
        .unwrap();
        // cannot go lower than 0
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 200);
    }

    #[async_std::test]
    async fn test_penalty_for_voucher() {
        let service = IdentityService::default();
        let user_b = "userB";
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
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 100);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 210);
        punish(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            50,
            PROOF_ID,
        )
        .await
        .unwrap();
        // 100 - 0.1 * 50
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 95);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 5);
        // penalty affects vouchee twice:
        // first, from the direct punishment
        // second, from the voucher reduced balance from the vouchee penalty
        // 200 - 50 + 0.1 * 95
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 159);
        assert_eq!(penalty(&service, &user_b.to_string()).await.unwrap(), 50);
    }

    #[async_std::test]
    async fn test_max_penalty_from_vouchees() {
        let service = IdentityService::default();
        let user_b = "userB";
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
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 210);
        punish(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            50000,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert_eq!(penalty(&service, &user_b.to_string()).await.unwrap(), 50000);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 5000);
        // balance is zero due to very high penalty
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 0);
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 0);
        punish(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            100000,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert_eq!(
            penalty(&service, &user_b.to_string()).await.unwrap(),
            100000
        );
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 10000);
        punish(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            150000,
            PROOF_ID,
        )
        .await
        .unwrap();
        // penalty for a user is unlimited, but
        // penalty from vouchees is limited to 2 * MAX_IDT_BY_PROOF
        assert_eq!(
            penalty(&service, &user_b.to_string()).await.unwrap(),
            150000
        );
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 10000);
    }

    #[async_std::test]
    async fn test_cleanup_forgotten_penalty() {
        let service = IdentityService::default();
        let user_b = "userB";
        let ts = next_timestamp();
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 0);
        service
            .punish_for_forgetting_with_timestamp(USER_A.to_string(), user_b.to_string(), ts)
            .await
            .unwrap();
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 500);
        service
            .punish_for_forgetting_with_timestamp(
                USER_A.to_string(),
                user_b.to_string(),
                ts - 86400,
            )
            .await
            .unwrap();
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 499);
        assert!(
            service
                .forgotten_penalty(&USER_A.to_string(), &user_b.to_string())
                .await
                .unwrap()
                .is_some()
        );
        service
            .punish_for_forgetting_with_timestamp(
                USER_A.to_string(),
                user_b.to_string(),
                ts - 86400 * 500,
            )
            .await
            .unwrap();
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 0);
        // forgotten penalty is cleaned up
        assert!(
            service
                .forgotten_penalty(&USER_A.to_string(), &user_b.to_string())
                .await
                .unwrap()
                .is_none()
        );
    }
}
