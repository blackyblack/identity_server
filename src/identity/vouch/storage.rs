use std::collections::HashMap;

use async_std::sync::RwLock;
use async_trait::async_trait;

use crate::identity::{UserAddress, error::Error};

#[async_trait]
pub trait VouchStorage: Send + Sync {
    async fn vouch(&self, from: UserAddress, to: UserAddress, timestamp: u64) -> Result<(), Error>;
    async fn vouchers_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error>;
    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error>;
    async fn remove_vouch(&self, voucher: UserAddress, vouchee: UserAddress) -> Result<(), Error>;
}

#[derive(Default)]
struct VouchData {
    // key - vouchee, vouch object
    // value - (voucher, unix timestamp) map
    vouchers: HashMap<UserAddress, HashMap<UserAddress, u64>>,
    // key - voucher, vouch subject
    // value - (vouchee, unix timestamp) map
    vouchees: HashMap<UserAddress, HashMap<UserAddress, u64>>,
}

#[derive(Default)]
pub struct InMemoryVouchStorage {
    // separate struct for atomic access to vouch data without deadlocks
    data: RwLock<VouchData>,
}

#[async_trait]
impl VouchStorage for InMemoryVouchStorage {
    async fn vouch(&self, from: UserAddress, to: UserAddress, timestamp: u64) -> Result<(), Error> {
        let mut lock = self.data.write().await;
        lock.vouchers
            .entry(to.clone())
            .and_modify(|v| {
                v.insert(from.clone(), timestamp);
            })
            .or_insert_with(|| {
                let mut m = HashMap::new();
                m.insert(from.clone(), timestamp);
                m
            });
        lock.vouchees
            .entry(from)
            .and_modify(|v| {
                v.insert(to.clone(), timestamp);
            })
            .or_insert_with(|| {
                let mut m = HashMap::new();
                m.insert(to, timestamp);
                m
            });
        Ok(())
    }

    async fn vouchers_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        Ok(self
            .data
            .read()
            .await
            .vouchers
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        Ok(self
            .data
            .read()
            .await
            .vouchees
            .get(user)
            .cloned()
            .unwrap_or_default())
    }

    async fn remove_vouch(&self, voucher: UserAddress, vouchee: UserAddress) -> Result<(), Error> {
        let mut lock = self.data.write().await;
        lock.vouchers.entry(vouchee.clone()).and_modify(|v| {
            v.remove(&voucher);
        });
        lock.vouchees.entry(voucher).and_modify(|v| {
            v.remove(&vouchee);
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = InMemoryVouchStorage::default();
        let user_a = "user_a".to_string();
        let user_b = "user_b".to_string();

        assert!(
            storage
                .vouchers_with_time(&user_b)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            storage
                .vouchees_with_time(&user_a)
                .await
                .unwrap()
                .is_empty()
        );

        storage
            .vouch(user_a.clone(), user_b.clone(), 1)
            .await
            .unwrap();
        assert_eq!(
            storage
                .vouchers_with_time(&user_b)
                .await
                .unwrap()
                .get(&user_a)
                .copied()
                .unwrap(),
            1
        );
        assert_eq!(
            storage
                .vouchees_with_time(&user_a)
                .await
                .unwrap()
                .get(&user_b)
                .copied()
                .unwrap(),
            1
        );
        assert_eq!(
            storage
                .vouchers_with_time(&user_a)
                .await
                .unwrap()
                .get(&user_b),
            None
        );
        assert_eq!(
            storage
                .vouchees_with_time(&user_b)
                .await
                .unwrap()
                .get(&user_a),
            None
        );

        storage
            .vouch(user_a.clone(), user_b.clone(), 5)
            .await
            .unwrap();
        assert_eq!(
            storage
                .vouchers_with_time(&user_b)
                .await
                .unwrap()
                .get(&user_a)
                .copied()
                .unwrap(),
            5
        );

        storage
            .remove_vouch(user_a.clone(), user_b.clone())
            .await
            .unwrap();
        assert!(
            storage
                .vouchers_with_time(&user_b)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            storage
                .vouchees_with_time(&user_a)
                .await
                .unwrap()
                .is_empty()
        );

        // removing a non-existing vouch should not fail
        storage
            .remove_vouch(user_a.clone(), user_b.clone())
            .await
            .unwrap();
    }
}
