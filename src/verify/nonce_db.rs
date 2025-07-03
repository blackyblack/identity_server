use async_trait::async_trait;
use sqlx::any::AnyPoolOptions;
use sqlx::{Acquire, AnyPool, Row};

use crate::identity::UserAddress;
use crate::verify::error::Error;
use crate::verify::nonce::NonceManager;

pub struct DatabaseNonceManager {
    pool: AnyPool,
}

impl DatabaseNonceManager {
    pub async fn new(url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS nonces (user TEXT PRIMARY KEY, used_nonce INTEGER NOT NULL)"
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl NonceManager for DatabaseNonceManager {
    async fn use_nonce(&self, user: &UserAddress, nonce: u64) -> Result<(), Error> {
        // start a transaction for atomic operations
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT used_nonce FROM nonces WHERE user = ?")
            .bind(user)
            .fetch_optional(tx.acquire().await?)
            .await?;

        let mut used = 0;
        if let Some(ref r) = row {
            used = r.get::<i64, _>(0) as u64;
        }

        if used >= nonce {
            return Err(Error::NonceUsedError(nonce));
        }

        // do not use upsert to support SQLite
        if row.is_some() {
            sqlx::query("UPDATE nonces SET used_nonce = ? WHERE user = ?")
                .bind(nonce as i64)
                .bind(user)
                .execute(tx.acquire().await?)
                .await?;
        } else {
            sqlx::query("INSERT INTO nonces (user, used_nonce) VALUES(?, ?)")
                .bind(user)
                .bind(nonce as i64)
                .execute(tx.acquire().await?)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn next_nonce(&self, user: &UserAddress) -> Result<u64, Error> {
        self.nonce(user)
            .await?
            .checked_add(1)
            .ok_or(Error::NonceOverflowError)
    }

    async fn nonce(&self, user: &UserAddress) -> Result<u64, Error> {
        let row = sqlx::query("SELECT used_nonce FROM nonces WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        if let Some(ref r) = row {
            // does not support u64 directly, so we use i64 when reading from DB
            return Ok(r.get::<i64, _>(0) as u64);
        }
        Ok(0)
    }
}
