use std::collections::HashSet;

use async_std::task;

use crate::identity::{IdentityService, UserAddress, next_timestamp};

impl IdentityService {
    pub async fn forget_with_timestamp(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        timestamp: u64,
    ) {
        task::block_on(self.vouches.remove_vouch(user.clone(), vouchee.clone()))
            .expect("storage error");
        self.punish_for_forgetting_with_timestamp(user, vouchee, timestamp)
            .await;
    }

    pub fn forgotten_users(&self, user: &UserAddress) -> HashSet<UserAddress> {
        task::block_on(self.penalties.forgotten_users(user)).expect("storage error")
    }

    // allows to clean up outdated penalties
    pub fn delete_forgotten(&self, user: UserAddress, forgotten: &UserAddress) {
        task::block_on(self.penalties.remove_forgotten(user, forgotten)).expect("storage error");
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
        punish::{penalty, punish},
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
        // this is implementation of forget() but with overridden timestamp
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
