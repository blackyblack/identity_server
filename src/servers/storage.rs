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
