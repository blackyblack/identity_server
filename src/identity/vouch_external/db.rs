use async_trait::async_trait;
use sqlx::{AnyPool, Row, any::AnyPoolOptions};

use super::storage::ExternalVouchStorage;
use crate::identity::{UserAddress, error::Error, vouch_external::storage::ServerWithVoucher};
use std::collections::HashMap;

pub struct DatabaseExternalVouchStorage {
    pool: AnyPool,
}

impl DatabaseExternalVouchStorage {
    pub async fn new(url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS external_vouches (server TEXT NOT NULL, voucher TEXT NOT NULL, vouchee TEXT NOT NULL, timestamp INTEGER NOT NULL, PRIMARY KEY(server, voucher, vouchee))",
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS external_voucher_idx ON external_vouches(server, voucher)",
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS external_vouchee_idx ON external_vouches(vouchee)")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl ExternalVouchStorage for DatabaseExternalVouchStorage {
    async fn vouch(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        sqlx::query(
            "REPLACE INTO external_vouches (server, voucher, vouchee, timestamp) VALUES (?, ?, ?, ?)",
        )
        .bind(&server)
        .bind(&from)
        .bind(&to)
        .bind(timestamp as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn vouchers_with_time(&self, user: &UserAddress) -> Result<ServerWithVoucher, Error> {
        let rows = sqlx::query(
            "SELECT server, voucher, timestamp FROM external_vouches WHERE vouchee = ?",
        )
        .bind(user)
        .fetch_all(&self.pool)
        .await?;
        let mut map: HashMap<UserAddress, HashMap<UserAddress, u64>> = HashMap::new();
        for r in rows {
            let server: String = r.get(0);
            let voucher: String = r.get(1);
            let ts: i64 = r.get(2);
            map.entry(server).or_default().insert(voucher, ts as u64);
        }
        Ok(map)
    }

    async fn remove_vouch(
        &self,
        server: UserAddress,
        from: UserAddress,
        to: UserAddress,
    ) -> Result<(), Error> {
        sqlx::query(
            "DELETE FROM external_vouches WHERE server = ? AND voucher = ? AND vouchee = ?",
        )
        .bind(&server)
        .bind(&from)
        .bind(&to)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = DatabaseExternalVouchStorage::new("sqlite::memory:")
            .await
            .unwrap();
        storage
            .vouch("server".into(), "from".into(), "to".into(), 1)
            .await
            .unwrap();
        let map = storage.vouchers_with_time(&"to".to_string()).await.unwrap();
        assert_eq!(map.get("server").unwrap().get("from").copied().unwrap(), 1);
    }

    #[async_std::test]
    async fn test_remove_vouch() {
        let storage = DatabaseExternalVouchStorage::new("sqlite::memory:")
            .await
            .unwrap();
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
        assert!(map.get("server").is_none());
    }
}
