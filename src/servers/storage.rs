use std::collections::HashMap;

use async_std::sync::RwLock;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::identity::UserAddress;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub url: String,
    pub scale: f64,
}

#[async_trait]
pub trait ServerStorage: Send + Sync {
    async fn add_server(
        &self,
        address: UserAddress,
        info: ServerInfo,
    ) -> Result<(), crate::servers::error::Error>;

    async fn remove_server(&self, address: UserAddress)
    -> Result<(), crate::servers::error::Error>;

    async fn servers(
        &self,
    ) -> Result<HashMap<UserAddress, ServerInfo>, crate::servers::error::Error>;
}

#[derive(Default)]
pub struct InMemoryServerStorage {
    servers: RwLock<HashMap<UserAddress, ServerInfo>>,
}

#[async_trait]
impl ServerStorage for InMemoryServerStorage {
    async fn add_server(
        &self,
        address: UserAddress,
        info: ServerInfo,
    ) -> Result<(), crate::servers::error::Error> {
        self.servers.write().await.insert(address, info);
        Ok(())
    }

    async fn remove_server(
        &self,
        address: UserAddress,
    ) -> Result<(), crate::servers::error::Error> {
        self.servers.write().await.remove(&address);
        Ok(())
    }

    async fn servers(
        &self,
    ) -> Result<HashMap<UserAddress, ServerInfo>, crate::servers::error::Error> {
        Ok(self.servers.read().await.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_basic() {
        let storage = InMemoryServerStorage::default();
        let server_address = "server1".to_string();
        let server_info = ServerInfo {
            url: "http://example.com".to_string(),
            scale: 1.0,
        };

        // initially, there should be no servers
        assert!(storage.servers().await.unwrap().is_empty());

        // add a server
        storage
            .add_server(server_address.clone(), server_info.clone())
            .await
            .unwrap();

        // check that the server was added correctly
        let servers = storage.servers().await.unwrap();
        assert_eq!(servers.len(), 1);
        assert!(servers.contains_key(&server_address));
        let retrieved_info = servers.get(&server_address).unwrap();
        assert_eq!(retrieved_info.url, server_info.url);
        assert_eq!(retrieved_info.scale, server_info.scale);

        // remove the server
        storage.remove_server(server_address.clone()).await.unwrap();

        // check that the server was removed
        assert!(storage.servers().await.unwrap().is_empty());
    }

    #[async_std::test]
    async fn test_multiple_servers() {
        let storage = InMemoryServerStorage::default();
        let server1 = "server1".to_string();
        let server2 = "server2".to_string();

        let info1 = ServerInfo {
            url: "http://example1.com".to_string(),
            scale: 1.0,
        };

        let info2 = ServerInfo {
            url: "http://example2.com".to_string(),
            scale: 2.0,
        };

        // add two servers
        storage
            .add_server(server1.clone(), info1.clone())
            .await
            .unwrap();
        storage
            .add_server(server2.clone(), info2.clone())
            .await
            .unwrap();

        // check both servers were added
        let servers = storage.servers().await.unwrap();
        assert_eq!(servers.len(), 2);

        let retrieved_info1 = servers.get(&server1).unwrap();
        assert_eq!(retrieved_info1.url, info1.url);
        assert_eq!(retrieved_info1.scale, info1.scale);

        let retrieved_info2 = servers.get(&server2).unwrap();
        assert_eq!(retrieved_info2.url, info2.url);
        assert_eq!(retrieved_info2.scale, info2.scale);

        // remove one server
        storage.remove_server(server1.clone()).await.unwrap();

        // check that only one server remains
        let servers = storage.servers().await.unwrap();
        assert_eq!(servers.len(), 1);
        assert!(servers.contains_key(&server2));
        assert!(!servers.contains_key(&server1));
    }

    #[async_std::test]
    async fn test_update_server() {
        let storage = InMemoryServerStorage::default();
        let server = "server1".to_string();

        let info1 = ServerInfo {
            url: "http://example1.com".to_string(),
            scale: 1.0,
        };

        let info2 = ServerInfo {
            url: "http://example2.com".to_string(),
            scale: 2.0,
        };

        // add a server
        storage
            .add_server(server.clone(), info1.clone())
            .await
            .unwrap();

        // update the server info
        storage
            .add_server(server.clone(), info2.clone())
            .await
            .unwrap();

        // check that the server was updated correctly
        let servers = storage.servers().await.unwrap();
        assert_eq!(servers.len(), 1);

        let retrieved_info = servers.get(&server).unwrap();
        assert_eq!(retrieved_info.url, info2.url);
        assert_eq!(retrieved_info.scale, info2.scale);
    }

    #[async_std::test]
    async fn test_remove_nonexistent() {
        let storage = InMemoryServerStorage::default();
        let server = "nonexistent".to_string();

        // removing a non-existent server should not cause an error
        let result = storage.remove_server(server.clone()).await;
        assert!(result.is_ok());

        // servers should still be empty
        assert!(storage.servers().await.unwrap().is_empty());
    }
}
