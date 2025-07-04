use async_trait::async_trait;
use sqlx::any::AnyPoolOptions;
use sqlx::{Acquire, AnyPool, Row};

use crate::identity::error::Error;
use crate::identity::storage::{PenaltyStorage, ProofStorage, VouchStorage};
use crate::identity::{IdtAmount, ModeratorProof, SystemPenalty, UserAddress};

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
    ) -> Result<std::collections::HashMap<UserAddress, u64>, Error> {
        let rows = sqlx::query("SELECT voucher, timestamp FROM vouches WHERE vouchee = ?")
            .bind(user)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get::<String, _>(0), r.get::<i64, _>(1) as u64))
            .collect())
    }

    async fn vouchees_with_time(
        &self,
        user: &UserAddress,
    ) -> Result<std::collections::HashMap<UserAddress, u64>, Error> {
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
    async fn set_genesis(
        &self,
        users: std::collections::HashMap<UserAddress, IdtAmount>,
    ) -> Result<(), Error> {
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
        Ok(row.map(|r| r.get::<i64, _>(0) as u128))
    }

    async fn set_proof(&self, user: UserAddress, proof: ModeratorProof) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT user FROM proofs WHERE user = ?")
            .bind(&user)
            .fetch_optional(tx.acquire().await?)
            .await?;
        if row.is_some() {
            sqlx::query("UPDATE proofs SET moderator = ?, amount = ?, proof_id = ?, timestamp = ? WHERE user = ?")
                .bind(&proof.moderator)
                .bind(proof.amount as i64)
                .bind(proof.proof_id as i64)
                .bind(proof.timestamp as i64)
                .bind(&user)
                .execute(tx.acquire().await?)
                .await?;
        } else {
            sqlx::query("INSERT INTO proofs (user, moderator, amount, proof_id, timestamp) VALUES (?, ?, ?, ?, ?)")
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

    async fn proof(&self, user: &UserAddress) -> Result<Option<ModeratorProof>, Error> {
        let row =
            sqlx::query("SELECT moderator, amount, proof_id, timestamp FROM proofs WHERE user = ?")
                .bind(user)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| ModeratorProof {
            moderator: r.get::<String, _>(0),
            amount: r.get::<i64, _>(1) as u128,
            proof_id: r.get::<i64, _>(2) as u128,
            timestamp: r.get::<i64, _>(3) as u64,
        }))
    }
}

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
        Ok(Self { pool })
    }
}

#[async_trait]
impl PenaltyStorage for DatabasePenaltyStorage {
    async fn insert_moderator_penalty(
        &self,
        user: UserAddress,
        proof: ModeratorProof,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;
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

    async fn insert_forgotten_penalty(
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
            amount: r.get::<i64, _>(1) as u128,
            proof_id: r.get::<i64, _>(2) as u128,
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
            amount: r.get::<i64, _>(0) as u128,
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
