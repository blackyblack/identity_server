use async_trait::async_trait;
use async_std::sync::RwLock;
use std::collections::HashMap;

use crate::identity::{UserAddress, error::Error};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalVouchRecord {
    pub voucher: UserAddress,
    pub vouchee: UserAddress,
    pub server: UserAddress,
    pub timestamp: u64,
}

#[async_trait]
pub trait ExternalVouchStorage: Send + Sync {
    async fn add_vouch(
        &self,
        voucher: UserAddress,
        vouchee: UserAddress,
        server: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error>;

    async fn all_vouches(&self) -> Result<Vec<ExternalVouchRecord>, Error>;
}

#[derive(Default)]
pub struct InMemoryExternalVouchStorage {
    data: RwLock<HashMap<(UserAddress, UserAddress, UserAddress), ExternalVouchRecord>>, 
}

#[async_trait]
impl ExternalVouchStorage for InMemoryExternalVouchStorage {
    async fn add_vouch(
        &self,
        voucher: UserAddress,
        vouchee: UserAddress,
        server: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        self
            .data
            .write()
            .await
            .insert(
                (voucher.clone(), vouchee.clone(), server.clone()),
                ExternalVouchRecord {
                    voucher,
                    vouchee,
                    server,
                    timestamp,
                },
            );
        Ok(())
    }

    async fn all_vouches(&self) -> Result<Vec<ExternalVouchRecord>, Error> {
        Ok(self
            .data
            .read()
            .await
            .values()
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = InMemoryExternalVouchStorage::default();
        storage
            .add_vouch(
                "a".into(),
                "b".into(),
                "s".into(),
                1,
            )
            .await
            .unwrap();
        let records = storage.all_vouches().await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].voucher, "a");
        assert_eq!(records[0].vouchee, "b");
        assert_eq!(records[0].server, "s");
        assert_eq!(records[0].timestamp, 1);
    }

    #[async_std::test]
    async fn test_replace() {
        let storage = InMemoryExternalVouchStorage::default();
        storage
            .add_vouch("a".into(), "b".into(), "s".into(), 1)
            .await
            .unwrap();
        storage
            .add_vouch("a".into(), "b".into(), "s".into(), 2)
            .await
            .unwrap();
        let records = storage.all_vouches().await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].timestamp, 2);
    }
}
