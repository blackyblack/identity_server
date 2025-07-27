use std::collections::HashMap;

use async_std::sync::RwLock;
use async_trait::async_trait;

use crate::identity::{UserAddress, error::Error};

#[async_trait]
pub trait ExternalVouchStorage: Send + Sync {
    async fn vouch(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error>;

    async fn vouchers_by_server_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, HashMap<UserAddress, u64>>, Error>;
}

#[derive(Default)]
struct ExternalVouchData {
    vouchers: HashMap<UserAddress, HashMap<UserAddress, HashMap<UserAddress, u64>>>,
}

#[derive(Default)]
pub struct InMemoryExternalVouchStorage {
    data: RwLock<ExternalVouchData>,
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
        lock.vouchers
            .entry(to)
            .or_default()
            .entry(server)
            .or_default()
            .insert(from, timestamp);
        Ok(())
    }

    async fn vouchers_by_server_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, HashMap<UserAddress, u64>>, Error> {
        Ok(self
            .data
            .read()
            .await
            .vouchers
            .get(user)
            .cloned()
            .unwrap_or_default())
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
        let map = storage
            .vouchers_by_server_with_time(&"to".to_string())
            .await
            .unwrap();
        assert_eq!(map.get("server").unwrap().get("from").copied().unwrap(), 1);
    }
}
