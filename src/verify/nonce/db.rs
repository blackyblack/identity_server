use async_trait::async_trait;
use sqlx::any::AnyPoolOptions;
use sqlx::{Acquire, AnyPool, Row};

use crate::identity::UserAddress;
use crate::verify::nonce::error::Error;
use crate::verify::nonce::{Nonce, NonceManager};

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
    async fn use_nonce(&self, user: &UserAddress, nonce: Nonce) -> Result<(), Error> {
        // start a transaction for atomic operations
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT used_nonce FROM nonces WHERE user = ?")
            .bind(user)
            .fetch_optional(tx.acquire().await?)
            .await?;

        let mut used = 0;
        if let Some(ref r) = row {
            used = r.get::<i64, _>(0) as Nonce;
        }

        if used >= nonce {
            return Err(Error::NonceUsedError(nonce));
        }

        sqlx::query("REPLACE INTO nonces (user, used_nonce) VALUES(?, ?)")
            .bind(user)
            .bind(nonce as i64)
            .execute(tx.acquire().await?)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn next_nonce(&self, user: &UserAddress) -> Result<Nonce, Error> {
        let row = sqlx::query("SELECT used_nonce FROM nonces WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        if let Some(ref r) = row {
            // does not support u64 directly, so we use i64 when reading from DB
            return (r.get::<i64, _>(0) as Nonce)
                .checked_add(1)
                .ok_or(Error::NonceOverflowError);
        }
        // first nonce is 1
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::random_keypair;

    #[async_std::test]
    async fn test_basic() {
        let (_priv, user) = random_keypair();
        let manager = DatabaseNonceManager::new("sqlite::memory:").await.unwrap();

        // next nonce should be 1
        assert_eq!(manager.next_nonce(&user).await.unwrap(), 1);
        // next nonce should still be 1 until we use it
        assert_eq!(manager.next_nonce(&user).await.unwrap(), 1);

        manager.use_nonce(&user, 1).await.unwrap();
        // next nonce should now be 2
        assert_eq!(manager.next_nonce(&user).await.unwrap(), 2);

        // using same nonce again should fail
        assert!(manager.use_nonce(&user, 1).await.is_err());
        // next nonce does not increment if use_nonce fails
        assert_eq!(manager.next_nonce(&user).await.unwrap(), 2);
    }
}
