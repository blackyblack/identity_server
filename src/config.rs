use std::{
    collections::{HashMap, HashSet},
    io,
};

use async_std::fs;
use serde::Deserialize;

use crate::identity::{IdtAmount, UserAddress};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AdminsSection {
    #[serde(default)]
    pub admins: HashSet<String>,
    #[serde(default)]
    pub moderators: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExternalServer {
    pub url: String,
    #[serde(default)]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExternalServersSection {
    #[serde(default)]
    pub allow_all: bool,
    #[serde(default)]
    pub servers: Vec<ExternalServer>,
}

impl Default for ExternalServersSection {
    fn default() -> Self {
        Self {
            allow_all: false,
            servers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub admins: AdminsSection,
    #[serde(default)]
    pub external_servers: ExternalServersSection,
}

pub const DEFAULT_CONFIG_PATH: &str = "config.json";
pub const DEFAULT_GENESIS_PATH: &str = "genesis.json";

pub async fn load_config(path: &str) -> Result<Config, io::Error> {
    let content = match fs::read_to_string(path).await {
        Ok(content) => content,
        Err(err) => {
            log::warn!("Failed to read {path}: {}", err);
            return Ok(Config::default());
        }
    };
    let config: Config = serde_json::from_str(&content)?;
    Ok(config)
}

pub async fn load_genesis(path: &str) -> Result<HashMap<UserAddress, IdtAmount>, io::Error> {
    let content = match fs::read_to_string(path).await {
        Ok(content) => content,
        Err(err) => {
            log::warn!("Failed to read {path}: {}", err);
            return Ok(HashMap::new());
        }
    };
    let genesis: HashMap<UserAddress, IdtAmount> = serde_json::from_str(&content)?;
    Ok(genesis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default() {
        let json = "{}";
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert!(cfg.admins.admins.is_empty());
        assert!(cfg.admins.moderators.is_empty());
        assert!(!cfg.external_servers.allow_all);
    }

    #[test]
    fn test_parse_servers() {
        let json = r#"{
            "admins": {"admins": ["a"], "moderators": []},
            "external_servers": {
                "allow_all": false,
                "servers": [
                    {"url": "http://a.com", "alias": "a"},
                    {"url": "http://b.com"}
                ]
            }
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.admins.admins.len(), 1);
        assert_eq!(cfg.external_servers.servers.len(), 2);
        assert_eq!(cfg.external_servers.servers[0].alias.as_deref(), Some("a"));
        assert_eq!(cfg.external_servers.servers[1].alias, None);
    }
}
