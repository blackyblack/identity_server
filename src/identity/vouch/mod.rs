use std::collections::HashMap;

use crate::identity::{IdentityService, UserAddress, error::Error, next_timestamp};

pub mod db;
pub mod storage;

impl IdentityService {
    pub async fn vouch_with_timestamp(
        &self,
        from: UserAddress,
        to: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        self.vouches.vouch(from, to, timestamp).await
    }

    pub async fn vouchers_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        self.vouches.vouchers_with_time(user).await
    }

    pub async fn vouchees_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        self.vouches.vouchees_with_time(user).await
    }
}

pub async fn vouch(
    service: &IdentityService,
    from: UserAddress,
    to: UserAddress,
) -> Result<(), Error> {
    service
        .vouch_with_timestamp(from, to, next_timestamp())
        .await
}

pub async fn vouchers(
    service: &IdentityService,
    user: &UserAddress,
) -> Result<Vec<UserAddress>, Error> {
    Ok(service
        .vouchers_with_time(user)
        .await?
        .into_keys()
        .collect())
}

pub async fn vouchees(
    service: &IdentityService,
    user: &UserAddress,
) -> Result<Vec<UserAddress>, Error> {
    Ok(service
        .vouchees_with_time(user)
        .await?
        .into_keys()
        .collect())
}

pub async fn voucher_timestamp(
    service: &IdentityService,
    user: &UserAddress,
    voucher: &UserAddress,
) -> Result<Option<u64>, Error> {
    Ok(service
        .vouchers_with_time(user)
        .await?
        .get(voucher)
        .copied())
}

pub async fn vouchee_timestamp(
    service: &IdentityService,
    user: &UserAddress,
    vouchee: &UserAddress,
) -> Result<Option<u64>, Error> {
    Ok(service
        .vouchees_with_time(user)
        .await?
        .get(vouchee)
        .copied())
}

#[cfg(test)]
mod tests {
    use crate::identity::tests::USER_A;

    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let service = IdentityService::default();
        let user_b = "userB";
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        assert!(vouchees(&service, &user_b.to_string()).await.unwrap().len() == 0);
        assert!(vouchers(&service, &user_b.to_string()).await.unwrap().len() == 1);
    }

    #[async_std::test]
    async fn test_vouch_self() {
        let service = IdentityService::default();
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        // user can vouch for himself
        vouch(&service, USER_A.to_string(), USER_A.to_string())
            .await
            .unwrap();
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 1);
    }

    #[async_std::test]
    async fn test_vouch_twice() {
        let service = IdentityService::default();
        let user_b = "userB";
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        assert!(vouchees(&service, &user_b.to_string()).await.unwrap().len() == 0);
        assert!(vouchers(&service, &user_b.to_string()).await.unwrap().len() == 1);
        // duplicate vouch does not change anything
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        assert!(vouchees(&service, &user_b.to_string()).await.unwrap().len() == 0);
        assert!(vouchers(&service, &user_b.to_string()).await.unwrap().len() == 1);
    }

    #[async_std::test]
    async fn test_vouch_mutual() {
        let service = IdentityService::default();
        let user_b = "userB";
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        vouch(&service, USER_A.to_string(), user_b.to_string())
            .await
            .unwrap();
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 0);
        assert!(vouchees(&service, &user_b.to_string()).await.unwrap().len() == 0);
        assert!(vouchers(&service, &user_b.to_string()).await.unwrap().len() == 1);
        vouch(&service, user_b.to_string(), USER_A.to_string())
            .await
            .unwrap();
        assert!(vouchees(&service, &USER_A.to_string()).await.unwrap().len() == 1);
        assert!(vouchers(&service, &USER_A.to_string()).await.unwrap().len() == 1);
        assert!(vouchees(&service, &user_b.to_string()).await.unwrap().len() == 1);
        assert!(vouchers(&service, &user_b.to_string()).await.unwrap().len() == 1);
    }
}
