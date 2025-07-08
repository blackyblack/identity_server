use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::{AnyPool, Row, any::AnyPoolOptions};

use crate::{
    identity::UserAddress,
    numbers::Rational,
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
        sqlx::query("CREATE TABLE IF NOT EXISTS servers (address TEXT PRIMARY KEY, url TEXT NOT NULL, scale_numerator INTEGER NOT NULL, scale_denominator INTEGER NOT NULL)")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl ServerStorage for DatabaseServerStorage {
    async fn add_server(&self, address: UserAddress, info: ServerInfo) -> Result<(), Error> {
        sqlx::query("REPLACE INTO servers (address, url, scale_numerator, scale_denominator) VALUES (?, ?, ?, ?)")
            .bind(address)
            .bind(info.url)
            .bind(info.scale.numerator() as i32)
            .bind(info.scale.denominator() as i32)
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
                let key = r.get::<String, _>(0);
                let url = r.get::<String, _>(1);
                let scale = Rational::new(r.get::<i32, _>(2) as u32, r.get::<i32, _>(3) as u32)
                    .expect("Scale factor denominator must not be zero");
                (key, ServerInfo { url, scale })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = DatabaseServerStorage::new("sqlite::memory:").await.unwrap();
        let server1 = "server1".to_string();
        let server2 = "server2".to_string();

        // verify initially no servers exist
        let servers = storage.servers().await.unwrap();
        assert!(servers.is_empty());

        // add a server
        let info1 = ServerInfo {
            url: "http://example1.com".to_string(),
            scale: Rational::default(),
        };
        storage
            .add_server(server1.clone(), info1.clone())
            .await
            .unwrap();

        // verify server was added
        let servers = storage.servers().await.unwrap();
        assert_eq!(servers.len(), 1);
        assert!(servers.contains_key(&server1));
        let retrieved_info = &servers[&server1];
        assert_eq!(retrieved_info.url, info1.url);
        assert_eq!(retrieved_info.scale, info1.scale);

        // add another server
        let info2 = ServerInfo {
            url: "http://example2.com".to_string(),
            scale: Rational::new(2, 1).unwrap(),
        };
        storage
            .add_server(server2.clone(), info2.clone())
            .await
            .unwrap();

        // verify both servers exist
        let servers = storage.servers().await.unwrap();
        assert_eq!(servers.len(), 2);
        assert!(servers.contains_key(&server1));
        assert!(servers.contains_key(&server2));

        // update a server
        let updated_info = ServerInfo {
            url: "http://updated.com".to_string(),
            scale: Rational::new(3, 1).unwrap(),
        };
        storage
            .add_server(server1.clone(), updated_info.clone())
            .await
            .unwrap();

        // verify update worked
        let servers = storage.servers().await.unwrap();
        assert_eq!(servers.len(), 2);
        let retrieved_info = &servers[&server1];
        assert_eq!(retrieved_info.url, updated_info.url);
        assert_eq!(retrieved_info.scale, updated_info.scale);

        // remove a server
        storage.remove_server(server1.clone()).await.unwrap();

        // verify server was removed
        let servers = storage.servers().await.unwrap();
        assert_eq!(servers.len(), 1);
        assert!(!servers.contains_key(&server1));
        assert!(servers.contains_key(&server2));

        // removing a non-existent server should not fail
        storage
            .remove_server("nonexistent".to_string())
            .await
            .unwrap();
    }
}
