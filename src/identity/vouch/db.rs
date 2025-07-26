use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{AnyPool, Row, any::AnyPoolOptions};

use crate::identity::{User, UserAddress, error::Error, vouch::storage::VouchStorage};

pub struct DatabaseVouchStorage {
    pool: AnyPool,
}

impl DatabaseVouchStorage {
    pub async fn new(url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS vouches (voucher TEXT NOT NULL, vouchee TEXT NOT NULL, server TEXT, timestamp INTEGER NOT NULL, PRIMARY KEY(voucher, vouchee, server))"
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS voucher_idx ON vouches(voucher)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS vouchee_idx ON vouches(vouchee)")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl VouchStorage for DatabaseVouchStorage {
    async fn vouch(&self, from: User, to: UserAddress, timestamp: u64) -> Result<(), Error> {
        let server = from.server().cloned();
        let addr = from.address().clone();
        let mut q = sqlx::query(
            "REPLACE INTO vouches (voucher, vouchee, server, timestamp) VALUES (?, ?, ?, ?)",
        );
        q = q
            .bind(&addr)
            .bind(&to)
            .bind(server.as_deref())
            .bind(timestamp as i64);
        q.execute(&self.pool).await?;
        Ok(())
    }

    async fn vouchers_with_time(
        &self,
        user: &UserAddress,
        server: Option<&UserAddress>,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        let rows = if let Some(server) = server {
            sqlx::query("SELECT voucher, timestamp FROM vouches WHERE vouchee = ? AND server = ?")
                .bind(user)
                .bind(server)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(
                "SELECT voucher, timestamp FROM vouches WHERE vouchee = ? AND server IS NULL",
            )
            .bind(user)
            .fetch_all(&self.pool)
            .await?
        };
        let vouchers = rows
            .into_iter()
            .map(|r| (r.get::<String, _>(0), r.get::<i64, _>(1) as u64))
            .collect();
        Ok(vouchers)
    }

    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
        server: Option<&UserAddress>,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        let rows = if let Some(server) = server {
            sqlx::query("SELECT vouchee, timestamp FROM vouches WHERE voucher = ? AND server = ?")
                .bind(user)
                .bind(server)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query(
                "SELECT vouchee, timestamp FROM vouches WHERE voucher = ? AND server IS NULL",
            )
            .bind(user)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows
            .into_iter()
            .map(|r| (r.get::<String, _>(0), r.get::<i64, _>(1) as u64))
            .collect())
    }

    async fn remove_vouch(&self, voucher: User, vouchee: UserAddress) -> Result<(), Error> {
        let server = voucher.server().cloned();
        let addr = voucher.address().clone();
        let mut q = if server.is_some() {
            sqlx::query("DELETE FROM vouches WHERE voucher = ? AND vouchee = ? AND server = ?")
        } else {
            sqlx::query("DELETE FROM vouches WHERE voucher = ? AND vouchee = ? AND server IS NULL")
        };
        q = q.bind(addr).bind(vouchee);
        if let Some(srv) = server {
            q = q.bind(srv);
        }
        q.execute(&self.pool).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = DatabaseVouchStorage::new("sqlite::memory:").await.unwrap();
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
