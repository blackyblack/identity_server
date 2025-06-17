use std::collections::{HashMap, HashSet};

use crate::identity::{
    IdentityService, IdtAmount, SystemPenalty, UserAddress, next_timestamp,
    punish::{PENALTY_VOUCHEE_WEIGHT_RATIO, penalty},
};

pub const FORGET_PENALTY: IdtAmount = 500;

impl IdentityService {
    pub async fn forget_with_timestamp(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        timestamp: u64,
    ) {
        let vouchee_penalty = penalty(self, &vouchee)
            .await
            .saturating_mul(PENALTY_VOUCHEE_WEIGHT_RATIO.0.into())
            .saturating_div(PENALTY_VOUCHEE_WEIGHT_RATIO.1.into());
        let event = SystemPenalty {
            idt_balance: FORGET_PENALTY + vouchee_penalty,
            timestamp,
        };
        self.vouches
            .write()
            .expect("Poisoned RwLock detected")
            .vouchers
            .entry(vouchee.clone())
            .and_modify(|v| {
                v.remove(&user);
            });
        self.vouches
            .write()
            .expect("Poisoned RwLock detected")
            .vouchees
            .entry(user.clone())
            .and_modify(|v| {
                v.remove(&vouchee);
            });
        self.penalties
            .write()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .entry(user)
            .and_modify(|v| {
                v.insert(vouchee.clone(), event.clone());
            })
            .or_insert_with(move || HashMap::from([(vouchee, event)]));
    }

    pub fn forgotten_users(&self, user: &UserAddress) -> HashSet<UserAddress> {
        self.penalties
            .read()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .get(user)
            .cloned()
            .unwrap_or_default()
            .into_keys()
            .collect()
    }

    // removes first penalty of the user, if available
    // allows to clean up outdated penalties
    pub fn delete_forgotten(&self, user: UserAddress, forgotten: &UserAddress) {
        self.penalties
            .write()
            .expect("Poisoned RwLock detected")
            .forget_penalties
            .entry(user)
            .and_modify(|v| {
                v.remove(forgotten);
            });
    }
}

pub async fn forget(service: &IdentityService, user: UserAddress, vouchee: UserAddress) {
    service
        .forget_with_timestamp(user, vouchee, next_timestamp())
        .await
}

#[cfg(test)]
mod tests {
    use crate::identity::{
        idt::balance,
        proof::prove,
        punish::punish,
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
            10000,
            PROOF_ID,
        );
        vouch(&service, USER_A.to_string(), user_b.to_string());
        assert_eq!(balance(&service, &USER_A.to_string()).await, 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await, 1000);
        forget(&service, USER_A.to_string(), user_b.to_string()).await;
        assert_eq!(balance(&service, &USER_A.to_string()).await, 9500);
        assert_eq!(balance(&service, &user_b.to_string()).await, 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await, 500);
    }

    #[async_std::test]
    async fn test_keep_penalty() {
        let user_b = "userB";
        let service = IdentityService::default();
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        vouch(&service, USER_A.to_string(), user_b.to_string());
        assert_eq!(balance(&service, &USER_A.to_string()).await, 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await, 1000);
        punish(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            500,
            PROOF_ID,
        );
        assert_eq!(balance(&service, &USER_A.to_string()).await, 9950);
        assert_eq!(balance(&service, &user_b.to_string()).await, 495);
        forget(&service, USER_A.to_string(), user_b.to_string()).await;
        assert_eq!(balance(&service, &USER_A.to_string()).await, 9450);
        assert_eq!(balance(&service, &user_b.to_string()).await, 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await, 550);
    }

    #[async_std::test]
    async fn test_multiple_penalties() {
        let user_b = "userB";
        let user_c = "userC";
        let service = IdentityService::default();
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        vouch(&service, USER_A.to_string(), user_b.to_string());
        vouch(&service, USER_A.to_string(), user_c.to_string());
        assert_eq!(balance(&service, &USER_A.to_string()).await, 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await, 1000);
        assert_eq!(balance(&service, &user_c.to_string()).await, 1000);
        forget(&service, USER_A.to_string(), user_b.to_string()).await;
        assert_eq!(balance(&service, &USER_A.to_string()).await, 9500);
        assert_eq!(balance(&service, &user_b.to_string()).await, 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await, 500);
        forget(&service, USER_A.to_string(), user_c.to_string()).await;
        assert_eq!(balance(&service, &USER_A.to_string()).await, 9000);
        assert_eq!(balance(&service, &user_c.to_string()).await, 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await, 1000);
    }

    #[async_std::test]
    async fn test_multiple_penalties_decay() {
        let user_b = "userB";
        let user_c = "userC";
        let service = IdentityService::default();
        let ts = next_timestamp();
        let _ = prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        );
        vouch(&service, USER_A.to_string(), user_b.to_string());
        vouch(&service, USER_A.to_string(), user_c.to_string());
        assert_eq!(balance(&service, &USER_A.to_string()).await, 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await, 1000);
        assert_eq!(balance(&service, &user_c.to_string()).await, 1000);
        // this is implementation of forget() but with overriden timestamp
        service
            .forget_with_timestamp(USER_A.to_string(), user_b.to_string(), ts - 86400 * 2)
            .await;
        assert_eq!(balance(&service, &USER_A.to_string()).await, 9502);
        assert_eq!(balance(&service, &user_b.to_string()).await, 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await, 498);
        service
            .forget_with_timestamp(USER_A.to_string(), user_c.to_string(), ts - 86400)
            .await;
        assert_eq!(balance(&service, &USER_A.to_string()).await, 9003);
        assert_eq!(balance(&service, &user_c.to_string()).await, 0);
        // penalties from forget() decay simultaneously for all forgotten users
        assert_eq!(penalty(&service, &USER_A.to_string()).await, 997);
    }
}
