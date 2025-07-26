use std::collections::HashMap;

use async_std::sync::RwLock;
use async_trait::async_trait;

use crate::identity::{User, UserAddress, error::Error};

#[derive(Default)]
struct ServerVouchData {
    // key - vouchee, vouch object
    // value - (voucher, unix timestamp) map
    vouchers: HashMap<UserAddress, HashMap<UserAddress, u64>>,
    // key - voucher, vouch subject
    // value - (vouchee, unix timestamp) map
    vouchees: HashMap<UserAddress, HashMap<UserAddress, u64>>,
}

#[async_trait]
pub trait VouchStorage: Send + Sync {
    async fn vouch(&self, from: User, to: UserAddress, timestamp: u64) -> Result<(), Error>;

    async fn vouchers_with_time(
        &self,
        user: &UserAddress,
        server: Option<&UserAddress>,
    ) -> Result<HashMap<UserAddress, u64>, Error>;

    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
        server: Option<&UserAddress>,
    ) -> Result<HashMap<UserAddress, u64>, Error>;

    async fn remove_vouch(&self, voucher: User, vouchee: UserAddress) -> Result<(), Error>;
}

#[derive(Default)]
struct VouchData {
    servers: HashMap<Option<UserAddress>, ServerVouchData>,
}

#[derive(Default)]
pub struct InMemoryVouchStorage {
    // separate struct for atomic access to vouch data without deadlocks
    data: RwLock<VouchData>,
}

#[async_trait]
impl VouchStorage for InMemoryVouchStorage {
    async fn vouch(&self, from: User, to: UserAddress, timestamp: u64) -> Result<(), Error> {
        let mut lock = self.data.write().await;
        let server = from.server().cloned();
        let from_address = from.address().clone();
        let entry = lock.servers.entry(server).or_default();
        entry
            .vouchers
            .entry(to.clone())
            .and_modify(|v| {
                v.insert(from_address.clone(), timestamp);
            })
            .or_insert_with(|| {
                let mut m = HashMap::new();
                m.insert(from_address.clone(), timestamp);
                m
            });
        entry
            .vouchees
            .entry(from_address)
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
        server: Option<&UserAddress>,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        Ok(self
            .data
            .read()
            .await
            .servers
            .get(&server.cloned())
            .and_then(|d| d.vouchers.get(user).cloned())
            .unwrap_or_default())
    }

    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
        server: Option<&UserAddress>,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        Ok(self
            .data
            .read()
            .await
            .servers
            .get(&server.cloned())
            .and_then(|d| d.vouchees.get(user).cloned())
            .unwrap_or_default())
    }

    async fn remove_vouch(&self, voucher: User, vouchee: UserAddress) -> Result<(), Error> {
        let mut lock = self.data.write().await;
        let server = voucher.server().cloned();
        let voucher_addr = voucher.address().clone();
        if let Some(data) = lock.servers.get_mut(&server) {
            data.vouchers.entry(vouchee.clone()).and_modify(|v| {
                v.remove(&voucher_addr);
            });
            data.vouchees.entry(voucher_addr).and_modify(|v| {
                v.remove(&vouchee);
            });
        }
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
                .vouchers_with_time(&user_b, None)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            storage
                .vouchees_with_time(&user_a, None)
                .await
                .unwrap()
                .is_empty()
        );

        storage
            .vouch(
                User::LocalUser {
                    user: user_a.clone(),
                },
                user_b.clone(),
                1,
            )
            .await
            .unwrap();
        assert_eq!(
            storage
                .vouchers_with_time(&user_b, None)
                .await
                .unwrap()
                .get(&user_a)
                .copied()
                .unwrap(),
            1
        );
        assert_eq!(
            storage
                .vouchees_with_time(&user_a, None)
                .await
                .unwrap()
                .get(&user_b)
                .copied()
                .unwrap(),
            1
        );
        assert_eq!(
            storage
                .vouchers_with_time(&user_a, None)
                .await
                .unwrap()
                .get(&user_b),
            None
        );
        assert_eq!(
            storage
                .vouchees_with_time(&user_b, None)
                .await
                .unwrap()
                .get(&user_a),
            None
        );

        storage
            .vouch(
                User::LocalUser {
                    user: user_a.clone(),
                },
                user_b.clone(),
                5,
            )
            .await
            .unwrap();
        assert_eq!(
            storage
                .vouchers_with_time(&user_b, None)
                .await
                .unwrap()
                .get(&user_a)
                .copied()
                .unwrap(),
            5
        );

        storage
            .remove_vouch(
                User::LocalUser {
                    user: user_a.clone(),
                },
                user_b.clone(),
            )
            .await
            .unwrap();
        assert!(
            storage
                .vouchers_with_time(&user_b, None)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            storage
                .vouchees_with_time(&user_a, None)
                .await
                .unwrap()
                .is_empty()
        );

        // removing a non-existing vouch should not fail
        storage
            .remove_vouch(
                User::LocalUser {
                    user: user_a.clone(),
                },
                user_b.clone(),
            )
            .await
            .unwrap();
    }
}
