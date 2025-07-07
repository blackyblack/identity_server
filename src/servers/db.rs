use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{AnyPool, Row, any::AnyPoolOptions};

use crate::{
    identity::UserAddress,
    servers::{
        error::Error,
        storage::{ServerInfo, ServerStorage},
    },
};

pub struct DatabaseServerStorage {
    pool: AnyPool,
}

impl DatabaseServerStorage {
    pub async fn new(url: &str) -> Result<Self, Error> {
        sqlx::any::install_default_drivers();
        let pool = AnyPoolOptions::new()
            .max_connections(1)
            .connect(url)
            .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS servers (address TEXT PRIMARY KEY, url TEXT NOT NULL, scale REAL NOT NULL)")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl ServerStorage for DatabaseServerStorage {
    async fn add_server(&self, address: UserAddress, info: ServerInfo) -> Result<(), Error> {
        sqlx::query("INSERT OR REPLACE INTO servers (address, url, scale) VALUES (?, ?, ?)")
            .bind(address)
            .bind(info.url)
            .bind(info.scale)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_server(&self, address: UserAddress) -> Result<(), Error> {
        sqlx::query("DELETE FROM servers WHERE address = ?")
            .bind(address)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn servers(&self) -> Result<HashMap<UserAddress, ServerInfo>, Error> {
        let rows = sqlx::query("SELECT address, url, scale FROM servers")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get::<String, _>(0),
                    ServerInfo {
                        url: r.get::<String, _>(1),
                        scale: r.get::<f64, _>(2),
                    },
                )
            })
            .collect())
    }
}
