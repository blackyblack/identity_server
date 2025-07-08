use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{Acquire, AnyPool, Row, any::AnyPoolOptions};

use crate::identity::{
    IdtAmount, ModeratorProof, ProofId, UserAddress, error::Error, proof::storage::ProofStorage,
};

pub struct DatabaseProofStorage {
    pool: AnyPool,
}

impl DatabaseProofStorage {
    pub async fn new(url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS proofs (user TEXT PRIMARY KEY, moderator TEXT NOT NULL, amount INTEGER NOT NULL, proof_id INTEGER NOT NULL, timestamp INTEGER NOT NULL)"
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS genesis (user TEXT PRIMARY KEY, balance INTEGER NOT NULL)",
        )
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl ProofStorage for DatabaseProofStorage {
    async fn set_genesis(&self, users: HashMap<UserAddress, IdtAmount>) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM genesis")
            .execute(tx.acquire().await?)
            .await?;
        for (user, bal) in users {
            sqlx::query("INSERT INTO genesis (user, balance) VALUES (?, ?)")
                .bind(user)
                .bind(bal as i64)
                .execute(tx.acquire().await?)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn genesis_balance(&self, user: &UserAddress) -> Result<Option<IdtAmount>, Error> {
        let row = sqlx::query("SELECT balance FROM genesis WHERE user = ?")
            .bind(user)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get::<i64, _>(0) as IdtAmount))
    }

    async fn set_proof(&self, user: UserAddress, proof: ModeratorProof) -> Result<(), Error> {
        sqlx::query("REPLACE INTO proofs (user, moderator, amount, proof_id, timestamp) VALUES (?, ?, ?, ?, ?)")
            .bind(&user)
            .bind(&proof.moderator)
            .bind(proof.amount as i64)
            .bind(proof.proof_id as i64)
            .bind(proof.timestamp as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn proof(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error> {
        let row =
            sqlx::query("SELECT moderator, amount, proof_id, timestamp FROM proofs WHERE user = ?")
                .bind(user)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| ModeratorProof {
            moderator: r.get::<String, _>(0),
            amount: r.get::<i64, _>(1) as IdtAmount,
            proof_id: r.get::<i64, _>(2) as ProofId,
            timestamp: r.get::<i64, _>(3) as u64,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = DatabaseProofStorage::new("sqlite::memory:").await.unwrap();
        let user = "user".to_string();
        let moderator = "moderator".to_string();

        let mut genesis = HashMap::<UserAddress, IdtAmount>::new();
        genesis.insert(user.clone(), 100);
        storage.set_genesis(genesis).await.unwrap();
        assert_eq!(storage.genesis_balance(&user).await.unwrap().unwrap(), 100);
        assert!(
            storage
                .genesis_balance(&"none".to_string())
                .await
                .unwrap()
                .is_none()
        );

        let proof1 = ModeratorProof {
            moderator: moderator.clone(),
            amount: 10,
            proof_id: 1,
            timestamp: 1,
        };
        storage
            .set_proof(user.clone(), proof1.clone())
            .await
            .unwrap();
        let res = storage.proof(&user).await.unwrap().unwrap();
        assert_eq!(res.moderator, proof1.moderator);
        assert_eq!(res.amount, proof1.amount);
        assert_eq!(res.proof_id, proof1.proof_id);
        assert_eq!(res.timestamp, proof1.timestamp);

        let proof2 = ModeratorProof {
            moderator: "mod2".to_string(),
            amount: 20,
            proof_id: 2,
            timestamp: 2,
        };
        storage
            .set_proof(user.clone(), proof2.clone())
            .await
            .unwrap();
        let res = storage.proof(&user).await.unwrap().unwrap();
        assert_eq!(res.moderator, proof2.moderator);
        assert_eq!(res.amount, proof2.amount);
        assert_eq!(res.proof_id, proof2.proof_id);
        assert_eq!(res.timestamp, proof2.timestamp);

        assert!(storage.proof(&"none".to_string()).await.unwrap().is_none());
    }
}
