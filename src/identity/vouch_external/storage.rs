use std::collections::HashMap;

use async_std::sync::RwLock;
use async_trait::async_trait;

use crate::identity::{UserAddress, error::Error};

// key - voucher
pub type VoucherWithTime = HashMap<UserAddress, u64>;
// key - server
pub type ServerWithVoucher = HashMap<UserAddress, VoucherWithTime>;
// key - vouchee
pub type VoucheeWithServer = HashMap<UserAddress, ServerWithVoucher>;

#[async_trait]
pub trait ExternalVouchStorage: Send + Sync {
    async fn vouch(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error>;

    async fn vouchers_with_time(&self, user: &UserAddress) -> Result<ServerWithVoucher, Error>;

    async fn remove_vouch(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
    ) -> Result<(), Error>;
}

#[derive(Default)]
pub struct InMemoryExternalVouchStorage {
    data: RwLock<VoucheeWithServer>,
}

#[async_trait]
impl ExternalVouchStorage for InMemoryExternalVouchStorage {
    async fn vouch(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        let mut lock = self.data.write().await;
        lock.entry(to)
            .or_default()
            .entry(server)
            .or_default()
            .insert(from, timestamp);
        Ok(())
    }

    async fn vouchers_with_time(&self, user: &UserAddress) -> Result<ServerWithVoucher, Error> {
        Ok(self
            .data
            .read()
            .await
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn remove_vouch(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
    ) -> Result<(), Error> {
        let mut lock = self.data.write().await;
        let server_map = match lock.get_mut(&to) {
            Some(map) => map,
            None => return Ok(()),
        };
        let vouchers = match server_map.get_mut(&server) {
            Some(vouchers) => vouchers,
            None => return Ok(()),
        };
        vouchers.remove(&from);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = InMemoryExternalVouchStorage::default();
        storage
            .vouch("server".into(), "from".into(), "to".into(), 1)
            .await
            .unwrap();
        let map = storage.vouchers_with_time(&"to".to_string()).await.unwrap();
        assert_eq!(map.get("server").unwrap().get("from").copied().unwrap(), 1);
    }

    #[async_std::test]
    async fn test_remove_vouch() {
        let storage = InMemoryExternalVouchStorage::default();
        storage
            .vouch("server".into(), "from".into(), "to".into(), 1)
            .await
            .unwrap();

        let map = storage.vouchers_with_time(&"to".into()).await.unwrap();
        assert_eq!(map.get("server").unwrap().get("from").copied().unwrap(), 1);

        storage
            .remove_vouch("server".into(), "from".into(), "to".into())
            .await
            .unwrap();
        // verify it no longer exists
        let map = storage.vouchers_with_time(&"to".into()).await.unwrap();
        assert!(map.get("server").unwrap().get("from").is_none());
    }
}
