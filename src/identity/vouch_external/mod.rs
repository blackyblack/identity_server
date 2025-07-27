use crate::identity::{IdentityService, UserAddress, error::Error, next_timestamp};

pub mod db;
pub mod storage;

pub struct ExternalVouch {
    pub voucher: UserAddress,
    pub server: UserAddress,
    pub timestamp: u64,
}

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

    pub async fn vouchers_external(&self, user: &UserAddress) -> Result<Vec<ExternalVouch>, Error> {
        let vouchers = self
            .external_vouches
            .vouchers_with_time(user)
            .await?
            .into_iter()
            .flat_map(|(server, vouches)| {
                vouches
                    .into_iter()
                    .map(move |(voucher, timestamp)| ExternalVouch {
                        voucher,
                        server: server.clone(),
                        timestamp,
                    })
            })
            .collect();
        Ok(vouchers)
    }

    pub async fn remove_external_vouch(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
    ) -> Result<(), Error> {
        self.external_vouches.remove_vouch(server, from, to).await
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

pub async fn forget_external(
    service: &IdentityService,
    server: UserAddress,
    from: UserAddress,
    to: UserAddress,
) -> Result<(), Error> {
    service.remove_external_vouch(server, from, to).await
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
            service
                .vouchers_external(&user_b.to_string())
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
        let vouchers = service
            .vouchers_external(&user_b.to_string())
            .await
            .unwrap();
        assert_eq!(vouchers.len(), 1);
        assert_eq!(vouchers.get(0).unwrap().voucher, USER_A.to_string());
    }
}
