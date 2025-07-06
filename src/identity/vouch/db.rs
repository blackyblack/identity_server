use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{Acquire, AnyPool, Row, any::AnyPoolOptions};

use crate::identity::{UserAddress, error::Error, vouch::storage::VouchStorage};

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
            "CREATE TABLE IF NOT EXISTS vouches (voucher TEXT NOT NULL, vouchee TEXT NOT NULL, timestamp INTEGER NOT NULL, PRIMARY KEY(voucher, vouchee))"
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
    async fn vouch(&self, from: UserAddress, to: UserAddress, timestamp: u64) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT timestamp FROM vouches WHERE voucher = ? AND vouchee = ?")
            .bind(&from)
            .bind(&to)
            .fetch_optional(tx.acquire().await?)
            .await?;
        // do not use upsert to support SQLite
        if row.is_some() {
            sqlx::query("UPDATE vouches SET timestamp = ? WHERE voucher = ? AND vouchee = ?")
                .bind(timestamp as i64)
                .bind(&from)
                .bind(&to)
                .execute(tx.acquire().await?)
                .await?;
        } else {
            sqlx::query("INSERT INTO vouches (voucher, vouchee, timestamp) VALUES (?, ?, ?)")
                .bind(&from)
                .bind(&to)
                .bind(timestamp as i64)
                .execute(tx.acquire().await?)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn vouchers_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        let rows = sqlx::query("SELECT voucher, timestamp FROM vouches WHERE vouchee = ?")
            .bind(user)
            .fetch_all(&self.pool)
            .await?;
        let vouchers = rows
            .into_iter()
            .map(|r| (r.get::<String, _>(0), r.get::<i64, _>(1) as u64))
            .collect();
        Ok(vouchers)
    }

    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<HashMap<UserAddress, u64>, Error> {
        let rows = sqlx::query("SELECT vouchee, timestamp FROM vouches WHERE voucher = ?")
            .bind(user)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get::<String, _>(0), r.get::<i64, _>(1) as u64))
            .collect())
    }

    async fn remove_vouch(&self, voucher: UserAddress, vouchee: UserAddress) -> Result<(), Error> {
        sqlx::query("DELETE FROM vouches WHERE voucher = ? AND vouchee = ?")
            .bind(voucher)
            .bind(vouchee)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_database_vouch_storage() {
        let storage = DatabaseVouchStorage::new("sqlite::memory:").await.unwrap();
        let user_a = "user_a".to_string();
        let user_b = "user_b".to_string();

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
