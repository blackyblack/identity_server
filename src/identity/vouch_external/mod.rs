use crate::identity::{IdentityService, UserAddress, error::Error, next_timestamp};
use std::collections::HashMap;

pub mod db;
pub mod storage;

impl IdentityService {
    pub async fn vouch_external_with_timestamp(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        self.external_vouches
            .vouch(server, from, to, timestamp)
            .await
    }

    pub async fn vouches_by_server_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, HashMap<UserAddress, u64>>, Error> {
        self.external_vouches
            .vouchers_by_server_with_time(user)
            .await
    }
}

pub async fn vouch_external(
    service: &IdentityService,
    server: UserAddress,
    from: UserAddress,
    to: UserAddress,
) -> Result<(), Error> {
    service
        .vouch_external_with_timestamp(server, from, to, next_timestamp())
        .await
}

pub async fn vouches_by_server_with_time(
    service: &IdentityService,
    user: &UserAddress,
) -> Result<HashMap<UserAddress, HashMap<UserAddress, u64>>, Error> {
    service.vouches_by_server_with_time(user).await
}

pub async fn vouches_by_server(
    service: &IdentityService,
    user: &UserAddress,
) -> Result<HashMap<UserAddress, Vec<UserAddress>>, Error> {
    Ok(vouches_by_server_with_time(service, user)
        .await?
        .into_iter()
        .map(|(srv, v)| (srv, v.into_keys().collect()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::tests::USER_A;

    #[async_std::test]
    async fn test_basic() {
        let service = IdentityService::default();
        let user_b = "userB";
        let server = "server";
        assert!(
            vouches_by_server(&service, &user_b.to_string())
                .await
                .unwrap()
                .is_empty()
        );
        vouch_external(
            &service,
            server.to_string(),
            USER_A.to_string(),
            user_b.to_string(),
        )
        .await
        .unwrap();
        let map = vouches_by_server(&service, &user_b.to_string())
            .await
            .unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.get(server).unwrap().contains(&USER_A.to_string()));
    }
}
