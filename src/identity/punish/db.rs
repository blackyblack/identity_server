use async_trait::async_trait;
use sqlx::{Acquire, AnyPool, Row, any::AnyPoolOptions};

use crate::identity::{
    IdtAmount, ModeratorProof, ProofId, SystemPenalty, UserAddress, error::Error,
    punish::storage::PenaltyStorage,
};

pub struct DatabasePenaltyStorage {
    pool: AnyPool,
}

impl DatabasePenaltyStorage {
    pub async fn new(url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS moderator_penalties (user TEXT PRIMARY KEY, moderator TEXT NOT NULL, amount INTEGER NOT NULL, proof_id INTEGER NOT NULL, timestamp INTEGER NOT NULL)"
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS forget_penalties (user TEXT NOT NULL, forgotten TEXT NOT NULL, amount INTEGER NOT NULL, timestamp INTEGER NOT NULL, PRIMARY KEY(user, forgotten))"
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS forget_penalties_idx ON forget_penalties(user)")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl PenaltyStorage for DatabasePenaltyStorage {
    async fn set_moderator_penalty(
        &self,
        user: UserAddress,
        proof: ModeratorProof,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        // this checks for user record
        let row = sqlx::query("SELECT user FROM moderator_penalties WHERE user = ?")
            .bind(&user)
            .fetch_optional(tx.acquire().await?)
            .await?;
        if row.is_some() {
            sqlx::query("UPDATE moderator_penalties SET moderator = ?, amount = ?, proof_id = ?, timestamp = ? WHERE user = ?")
                .bind(&proof.moderator)
                .bind(proof.amount as i64)
                .bind(proof.proof_id as i64)
                .bind(proof.timestamp as i64)
                .bind(&user)
                .execute(tx.acquire().await?)
                .await?;
        } else {
            sqlx::query("INSERT INTO moderator_penalties (user, moderator, amount, proof_id, timestamp) VALUES (?, ?, ?, ?, ?)")
                .bind(&user)
                .bind(&proof.moderator)
                .bind(proof.amount as i64)
                .bind(proof.proof_id as i64)
                .bind(proof.timestamp as i64)
                .execute(tx.acquire().await?)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn set_forgotten_penalty(
        &self,
        user: UserAddress,
        vouchee: UserAddress,
        penalty: SystemPenalty,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        let row =
            sqlx::query("SELECT amount FROM forget_penalties WHERE user = ? AND forgotten = ?")
                .bind(&user)
                .bind(&vouchee)
                .fetch_optional(tx.acquire().await?)
                .await?;
        if row.is_some() {
            sqlx::query("UPDATE forget_penalties SET amount = ?, timestamp = ? WHERE user = ? AND forgotten = ?")
                .bind(penalty.amount as i64)
                .bind(penalty.timestamp as i64)
                .bind(&user)
                .bind(&vouchee)
                .execute(tx.acquire().await?)
                .await?;
        } else {
            sqlx::query("INSERT INTO forget_penalties (user, forgotten, amount, timestamp) VALUES (?, ?, ?, ?)")
                .bind(&user)
                .bind(&vouchee)
                .bind(penalty.amount as i64)
                .bind(penalty.timestamp as i64)
                .execute(tx.acquire().await?)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn remove_forgotten(
        &self,
        user: UserAddress,
        forgotten: &UserAddress,
    ) -> Result<(), Error> {
        sqlx::query("DELETE FROM forget_penalties WHERE user = ? AND forgotten = ?")
            .bind(&user)
            .bind(forgotten)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn moderator_penalty(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error> {
        let row = sqlx::query(
            "SELECT moderator, amount, proof_id, timestamp FROM moderator_penalties WHERE user = ?",
        )
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

    async fn forgotten_penalty(
        &self,
        user: &UserAddress,
        forgotten: &UserAddress,
    ) -> Result<Option<SystemPenalty>, Error> {
        let row = sqlx::query(
            "SELECT amount, timestamp FROM forget_penalties WHERE user = ? AND forgotten = ?",
        )
        .bind(user)
        .bind(forgotten)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| SystemPenalty {
            amount: r.get::<i64, _>(0) as IdtAmount,
            timestamp: r.get::<i64, _>(1) as u64,
        }))
    }

    async fn forgotten_users(
        &self,
        user: &UserAddress,
    ) -> Result<std::collections::HashSet<UserAddress>, Error> {
        let rows = sqlx::query("SELECT forgotten FROM forget_penalties WHERE user = ?")
            .bind(user)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get::<String, _>(0)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = DatabasePenaltyStorage::new("sqlite::memory:")
            .await
            .unwrap();
        let user = "user".to_string();
        let vouchee = "vouchee".to_string();

        let proof1 = ModeratorProof {
            moderator: "mod".to_string(),
            amount: 1,
            proof_id: 1,
            timestamp: 2,
        };
        storage
            .set_moderator_penalty(user.clone(), proof1.clone())
            .await
            .unwrap();
        let res = storage.moderator_penalty(&user).await.unwrap().unwrap();
        assert_eq!(res.moderator, proof1.moderator);
        assert_eq!(res.amount, proof1.amount);
        assert_eq!(res.proof_id, proof1.proof_id);
        assert_eq!(res.timestamp, proof1.timestamp);

        let proof2 = ModeratorProof {
            moderator: "mod2".to_string(),
            amount: 3,
            proof_id: 2,
            timestamp: 4,
        };
        storage
            .set_moderator_penalty(user.clone(), proof2.clone())
            .await
            .unwrap();
        let res = storage.moderator_penalty(&user).await.unwrap().unwrap();
        assert_eq!(res.moderator, proof2.moderator);
        assert_eq!(res.amount, proof2.amount);
        assert_eq!(res.proof_id, proof2.proof_id);
        assert_eq!(res.timestamp, proof2.timestamp);

        assert!(
            storage
                .moderator_penalty(&"none".to_string())
                .await
                .unwrap()
                .is_none()
        );

        let penalty1 = SystemPenalty {
            amount: 5,
            timestamp: 6,
        };
        storage
            .set_forgotten_penalty(user.clone(), vouchee.clone(), penalty1.clone())
            .await
            .unwrap();
        let res = storage
            .forgotten_penalty(&user, &vouchee)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(res.amount, penalty1.amount);
        assert_eq!(res.timestamp, penalty1.timestamp);
        assert!(
            storage
                .forgotten_users(&user)
                .await
                .unwrap()
                .contains(&vouchee)
        );

        let penalty2 = SystemPenalty {
            amount: 7,
            timestamp: 8,
        };
        storage
            .set_forgotten_penalty(user.clone(), vouchee.clone(), penalty2.clone())
            .await
            .unwrap();
        let res = storage
            .forgotten_penalty(&user, &vouchee)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(res.amount, penalty2.amount);
        assert_eq!(res.timestamp, penalty2.timestamp);

        storage
            .remove_forgotten(user.clone(), &vouchee)
            .await
            .unwrap();
        assert!(
            storage
                .forgotten_penalty(&user, &vouchee)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            !storage
                .forgotten_users(&user)
                .await
                .unwrap()
                .contains(&vouchee)
        );
    }
}
