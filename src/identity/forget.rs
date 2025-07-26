use std::collections::HashSet;

use crate::identity::{IdentityService, User, UserAddress, error::Error, next_timestamp};

impl IdentityService {
    pub async fn forget_with_timestamp(
        &self,
        user: User,
        vouchee: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        self.vouches
            .remove_vouch(user.clone(), vouchee.clone())
            .await?;
        self.punish_for_forgetting_with_timestamp(user.address().clone(), vouchee, timestamp)
            .await
    }

    pub async fn forgotten_users(&self, user: &UserAddress) -> Result<HashSet<UserAddress>, Error> {
        self.penalties.forgotten_users(user).await
    }

    // allows to clean up outdated penalties
    pub async fn delete_forgotten(
        &self,
        user: UserAddress,
        forgotten: &UserAddress,
    ) -> Result<(), Error> {
        self.penalties.remove_forgotten(user, forgotten).await
    }
}

pub async fn forget(
    service: &IdentityService,
    user: User,
    vouchee: UserAddress,
) -> Result<(), Error> {
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
        prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        )
        .await
        .unwrap();
        vouch(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_b.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 1000);
        forget(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_b.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 9500);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 500);
    }

    #[async_std::test]
    async fn test_keep_penalty() {
        let user_b = "userB";
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
        vouch(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_b.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 1000);
        punish(
            &service,
            user_b.to_string(),
            MODERATOR.to_string(),
            500,
            PROOF_ID,
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 9950);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 495);
        forget(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_b.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 9450);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 550);
    }

    #[async_std::test]
    async fn test_multiple_penalties() {
        let user_b = "userB";
        let user_c = "userC";
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
        vouch(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_b.to_string(),
        )
        .await
        .unwrap();
        vouch(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_c.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 1000);
        assert_eq!(balance(&service, &user_c.to_string()).await.unwrap(), 1000);
        forget(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_b.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 9500);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 500);
        forget(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_c.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 9000);
        assert_eq!(balance(&service, &user_c.to_string()).await.unwrap(), 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 1000);
    }

    #[async_std::test]
    async fn test_multiple_penalties_decay() {
        let user_b = "userB";
        let user_c = "userC";
        let service = IdentityService::default();
        let ts = next_timestamp();
        prove(
            &service,
            USER_A.to_string(),
            MODERATOR.to_string(),
            10000,
            PROOF_ID,
        )
        .await
        .unwrap();
        vouch(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_b.to_string(),
        )
        .await
        .unwrap();
        vouch(
            &service,
            User::LocalUser {
                user: USER_A.to_string(),
            },
            user_c.to_string(),
        )
        .await
        .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 10000);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 1000);
        assert_eq!(balance(&service, &user_c.to_string()).await.unwrap(), 1000);
        // this is implementation of forget() but with overridden timestamp
        service
            .forget_with_timestamp(
                User::LocalUser {
                    user: USER_A.to_string(),
                },
                user_b.to_string(),
                ts - 86400 * 2,
            )
            .await
            .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 9502);
        assert_eq!(balance(&service, &user_b.to_string()).await.unwrap(), 0);
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 498);
        service
            .forget_with_timestamp(
                User::LocalUser {
                    user: USER_A.to_string(),
                },
                user_c.to_string(),
                ts - 86400,
            )
            .await
            .unwrap();
        assert_eq!(balance(&service, &USER_A.to_string()).await.unwrap(), 9003);
        assert_eq!(balance(&service, &user_c.to_string()).await.unwrap(), 0);
        // penalties from forget() decay simultaneously for all forgotten users
        assert_eq!(penalty(&service, &USER_A.to_string()).await.unwrap(), 997);
    }
}
