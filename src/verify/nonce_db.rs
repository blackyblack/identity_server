use async_trait::async_trait;
use sqlx::{AnyPool, Row};

use crate::identity::UserAddress;
use crate::verify::error::Error;
use crate::verify::nonce::NonceManager;

pub struct DatabaseNonceManager {
    pool: AnyPool,
}

impl DatabaseNonceManager {
    pub async fn new(url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS nonces (user TEXT PRIMARY KEY, used_nonce INTEGER NOT NULL, next_nonce INTEGER NOT NULL)"
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl NonceManager for DatabaseNonceManager {
    async fn use_nonce(&self, user: &UserAddress, nonce: u64) -> Result<(), Error> {
        let row = sqlx::query("SELECT used_nonce FROM nonces WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        let used = row
            .as_ref()
            .map(|r| r.get::<i64, _>(0) as u64)
            .unwrap_or(0);
        if used >= nonce {
            return Err(Error::NonceUsedError(nonce));
        }
        if row.is_none() {
            sqlx::query(
                "INSERT INTO nonces (user, used_nonce, next_nonce) VALUES (?, ?, 0)",
            )
            .bind(user)
            .bind(nonce as i64)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query("UPDATE nonces SET used_nonce = ? WHERE user = ?")
                .bind(nonce as i64)
                .bind(user)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    async fn next_nonce(&self, user: &UserAddress) -> Result<u64, Error> {
        let row = sqlx::query("SELECT next_nonce FROM nonces WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        let next = match row {
            Some(ref r) => {
                let mut n = r.get::<i64, _>(0) as u64;
                n += 1;
                sqlx::query("UPDATE nonces SET next_nonce = ? WHERE user = ?")
                    .bind(n as i64)
                    .bind(user)
                    .execute(&self.pool)
                    .await?;
                n
            }
            None => {
                let n = 1u64;
                sqlx::query(
                    "INSERT INTO nonces (user, used_nonce, next_nonce) VALUES (?, 0, ?)",
                )
                .bind(user)
                .bind(n as i64)
                .execute(&self.pool)
                .await?;
                n
            }
        };
        Ok(next)
    }

    async fn nonce(&self, user: &UserAddress) -> Result<u64, Error> {
        let row = sqlx::query("SELECT next_nonce FROM nonces WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<i64, _>(0) as u64).unwrap_or(0))
    }
}
