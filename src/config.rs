use std::{
    collections::{HashMap, HashSet},
    io,
};

use async_std::fs;
use serde::Deserialize;

use crate::identity::{IdtAmount, UserAddress};

pub const DEFAULT_CONFIG_PATH: &str = "config.json";
pub const DEFAULT_GENESIS_PATH: &str = "genesis.json";

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AdminsSection {
    #[serde(default)]
    pub admins: HashSet<String>,
    #[serde(default)]
    pub moderators: HashSet<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub admins: AdminsSection,
}

pub async fn load_config(path: &str) -> Result<Config, io::Error> {
    let content = match fs::read_to_string(path).await {
        Ok(content) => content,
        Err(err) => {
            log::warn!("Failed to read {}: {}", path, err);
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
            log::warn!("Failed to read {}: {}", path, err);
            return Ok(HashMap::new());
        }
    };
    let genesis: HashMap<UserAddress, IdtAmount> = serde_json::from_str(&content)?;
    Ok(genesis)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use async_std::{fs::File, io::WriteExt};
    use tempdir::TempDir;

    use super::*;

    async fn create_test_config(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("config.json");
        let mut file = File::create(&path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        path
    }

    async fn create_test_genesis_config(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("genesis.json");
        let mut file = File::create(&path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        path
    }

    #[test]
    fn test_parse_default() {
        let json = "{}";
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert!(cfg.admins.admins.is_empty());
        assert!(cfg.admins.moderators.is_empty());
        // no external server configuration
    }

    #[async_std::test]
    async fn test_load_config_nonexistent_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = temp_dir.path().join("nonexistent.json");
        let cfg = load_config(path.to_str().unwrap()).await.unwrap();
        assert!(cfg.admins.admins.is_empty());
        assert!(cfg.admins.moderators.is_empty());
    }

    #[async_std::test]
    async fn test_load_config_valid_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let content = r#"{
            "admins": {"admins": ["user"], "moderators": ["mod"]}
        }"#;
        let path = create_test_config(&temp_dir, content).await;
        let cfg = load_config(path.to_str().unwrap()).await.unwrap();
        assert_eq!(cfg.admins.admins.len(), 1);
        assert_eq!(cfg.admins.moderators.len(), 1);
    }

    #[async_std::test]
    async fn test_load_config_invalid_json() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_config(&temp_dir, "{").await;
        assert!(load_config(path.to_str().unwrap()).await.is_err());
    }

    #[async_std::test]
    async fn test_load_config_empty_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_config(&temp_dir, "").await;
        assert!(load_config(path.to_str().unwrap()).await.is_err());
    }

    #[async_std::test]
    async fn test_load_genesis_nonexistent_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = temp_dir.path().join("nonexistent.json");
        let balances = load_genesis(path.to_str().unwrap()).await.unwrap();
        assert!(balances.is_empty());
    }

    #[async_std::test]
    async fn test_load_genesis_valid() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_genesis_config(&temp_dir, "{\"alice\":1}").await;
        let balances = load_genesis(path.to_str().unwrap()).await.unwrap();
        assert_eq!(balances["alice"], 1);
    }

    #[async_std::test]
    async fn test_load_genesis_invalid() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_genesis_config(&temp_dir, "{").await;
        assert!(load_genesis(path.to_str().unwrap()).await.is_err());
    }

    #[async_std::test]
    async fn test_load_genesis_empty() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_genesis_config(&temp_dir, "").await;
        assert!(load_genesis(path.to_str().unwrap()).await.is_err());
    }
}
