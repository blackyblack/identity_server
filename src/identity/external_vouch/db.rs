use async_trait::async_trait;
use sqlx::{AnyPool, Row, any::AnyPoolOptions};

use crate::identity::{UserAddress, error::Error};
use super::storage::{ExternalVouchStorage, ExternalVouchRecord};

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
            "CREATE TABLE IF NOT EXISTS external_vouches (voucher TEXT NOT NULL, vouchee TEXT NOT NULL, server TEXT NOT NULL, timestamp INTEGER NOT NULL, PRIMARY KEY(voucher, vouchee, server))",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl ExternalVouchStorage for DatabaseExternalVouchStorage {
    async fn add_vouch(
        &self,
        voucher: UserAddress,
        vouchee: UserAddress,
        server: UserAddress,
        timestamp: u64,
    ) -> Result<(), Error> {
        sqlx::query("REPLACE INTO external_vouches (voucher, vouchee, server, timestamp) VALUES (?, ?, ?, ?)")
            .bind(voucher)
            .bind(vouchee)
            .bind(server)
            .bind(timestamp as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn all_vouches(&self) -> Result<Vec<ExternalVouchRecord>, Error> {
        let rows = sqlx::query("SELECT voucher, vouchee, server, timestamp FROM external_vouches")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| ExternalVouchRecord {
                voucher: r.get::<String, _>(0),
                vouchee: r.get::<String, _>(1),
                server: r.get::<String, _>(2),
                timestamp: r.get::<i64, _>(3) as u64,
            })
            .collect())
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
            .add_vouch("a".into(), "b".into(), "s".into(), 1)
            .await
            .unwrap();
        let records = storage.all_vouches().await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].voucher, "a");
        assert_eq!(records[0].vouchee, "b");
        assert_eq!(records[0].server, "s");
        assert_eq!(records[0].timestamp, 1);
    }
}
