use crate::identity::{IdentityService, UserAddress, error::Error, next_timestamp};

pub mod storage;
pub mod db;
use storage::ExternalVouchRecord;

impl IdentityService {
    pub async fn add_external_vouch_with_timestamp(
        &self,
        from: UserAddress,
        to: UserAddress,
        server: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        self
            .external_vouches
            .add_vouch(from, to, server, timestamp)
            .await
    }

    pub async fn external_vouches(&self) -> Result<Vec<ExternalVouchRecord>, Error> {
        self.external_vouches.all_vouches().await
    }
}

pub async fn add_external_vouch(
    service: &IdentityService,
    from: UserAddress,
    to: UserAddress,
    server: UserAddress,
) -> Result<(), Error> {
    service
        .add_external_vouch_with_timestamp(from, to, server, next_timestamp())
        .await
}
