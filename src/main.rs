use std::{
    collections::HashSet,
    env,
    io::{Error, Write},
    sync::Arc,
};

use async_std::fs;
use identity_server::{
    admins::InMemoryAdminStorage,
    identity::IdentityService,
    routes::{self, State},
    verify::nonce_db::DatabaseNonceManager,
};
use serde::Deserialize;
use tide::Server;

pub const DEFAULT_PORT: u32 = 8080;
pub const DEFAULT_HOST: &str = "localhost";

pub const DEFAULT_MYSQL_USER: &str = "root";
pub const DEFAULT_MYSQL_HOST: &str = "localhost";
pub const DEFAULT_MYSQL_PORT: u32 = 3306;
pub const DEFAULT_MYSQL_DATABASE: &str = "identity";

pub const DEFAULT_ADMINS_CONFIG_PATH: &str = "admins.json";

#[derive(Deserialize, Default)]
struct AdminConfig {
    admins: HashSet<String>,
    moderators: HashSet<String>,
}

#[async_std::main]
async fn main() {
    // load environment variables from `.env` if present
    let _ = dotenv::dotenv();
    // use INFO log level by default, disable all libraries logging
    // use RUST_LOG env variable to override log level
    let log_env = env_logger::Env::default().default_filter_or("info,tide=off");
    env_logger::Builder::from_env(log_env)
        .format(|buf, record| {
            let ts = buf.timestamp_seconds();
            writeln!(buf, "[{}] {}: {}", record.level(), ts, record.args())
        })
        .init();
    let config = match load_admin_config(DEFAULT_ADMINS_CONFIG_PATH).await {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to load admin configuration: {:?}", e);
            panic!("Failed to load admin configuration: {}", e);
        }
    };

    let db_url = setup_database_url();
    let nonce_manager = match DatabaseNonceManager::new(&db_url).await {
        Ok(nonce_manager) => nonce_manager,
        Err(e) => {
            log::error!("Failed to connect to database: {:?}", e);
            panic!("Failed to connect to database: {}", e);
        }
    };

    let state = State {
        identity_service: IdentityService::default(),
        admin_storage: Arc::new(InMemoryAdminStorage::new(config.admins, config.moderators)),
        nonce_manager: Arc::new(nonce_manager),
    };
    log::info!("Starting identity server");
    if let Err(err) = start_server(state).await {
        log::error!("Failed to start server: {:?}", err);
        panic!("Failed to start server: {}", err);
    }
}

async fn load_admin_config(path: &str) -> Result<AdminConfig, std::io::Error> {
    let content = match fs::read_to_string(path).await {
        Ok(content) => content,
        Err(err) => {
            log::warn!("Failed to read {path}: {}", err);
            return Ok(AdminConfig::default());
        }
    };
    let config: AdminConfig = serde_json::from_str(&content)?;
    Ok(config)
}

fn setup_database_url() -> String {
    let db_user = match env::var("MYSQL_USER").unwrap_or_default().as_str() {
        "" => DEFAULT_MYSQL_USER.to_string(),
        user_str => user_str.to_string(),
    };
    let db_password = env::var("MYSQL_PASSWORD").unwrap_or_default();
    let db_host = match env::var("MYSQL_HOST").unwrap_or_default().as_str() {
        "" => DEFAULT_MYSQL_HOST.to_string(),
        host_str => host_str.to_string(),
    };
    let db_port = match env::var("MYSQL_PORT").unwrap_or_default().as_str() {
        "" => DEFAULT_MYSQL_PORT,
        port_str => port_str.parse::<u32>().unwrap_or(DEFAULT_MYSQL_PORT),
    };
    let db_name = match env::var("MYSQL_DATABASE").unwrap_or_default().as_str() {
        "" => DEFAULT_MYSQL_DATABASE.to_string(),
        db_str => db_str.to_string(),
    };
    format!("mysql://{db_user}:{db_password}@{db_host}:{db_port}/{db_name}")
}

async fn start_server(state: State) -> Result<(), Error> {
    let port = match env::var("PORT").unwrap_or_default().as_str() {
        "" => DEFAULT_PORT,
        port_str => port_str.parse::<u32>().unwrap_or(DEFAULT_PORT),
    };
    let host = match env::var("HOST").unwrap_or_default().as_str() {
        "" => DEFAULT_HOST.to_string(),
        host_str => host_str.to_string(),
    };
    let mut server = tide::with_state(state);
    setup_routes(&mut server).await;
    server.listen(format!("{host}:{port}")).await
}

async fn setup_routes(server: &mut Server<State>) {
    server.at("/idt/:user").get(routes::idt::route);
    server.at("/vouch/:user").post(routes::vouch::route);
    server.at("/forget/:user").post(routes::forget::route);
    server.at("/punish/:user").post(routes::punish::route);
    server
        .at("/is_admin/:user")
        .get(routes::admins::is_admin::route);
    server
        .at("/add_admin/:user")
        .post(routes::admins::add_admin::route);
    server
        .at("/remove_admin/:user")
        .post(routes::admins::remove_admin::route);
    server
        .at("/is_moderator/:user")
        .get(routes::admins::is_moderator::route);
    server
        .at("/add_moderator/:user")
        .post(routes::admins::add_moderator::route);
    server
        .at("/remove_moderator/:user")
        .post(routes::admins::remove_moderator::route);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use async_std::{fs::File, io::WriteExt};
    use tempdir::TempDir;

    use super::*;

    // Helper function to create a temporary file with content
    async fn create_test_config(dir: &TempDir, content: &str) -> PathBuf {
        let config_path = dir.path().join("admins.json");
        let mut file = File::create(&config_path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        config_path
    }

    #[async_std::test]
    async fn test_load_admin_config_nonexistent_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let non_existent_path = temp_dir.path().join("nonexistent.json");

        let config = load_admin_config(non_existent_path.to_str().unwrap())
            .await
            .unwrap();
        assert!(config.admins.is_empty());
        assert!(config.moderators.is_empty());
    }

    #[async_std::test]
    async fn test_load_admin_config_valid_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let config_content = r#"{
            "admins": ["user1", "user2"],
            "moderators": ["mod1", "mod2", "mod3"]
        }"#;

        let config_path = create_test_config(&temp_dir, config_content).await;

        let config = load_admin_config(config_path.to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(config.admins.len(), 2);
        assert!(config.admins.contains("user1"));
        assert!(config.admins.contains("user2"));
        assert_eq!(config.moderators.len(), 3);
        assert!(config.moderators.contains("mod1"));
        assert!(config.moderators.contains("mod2"));
        assert!(config.moderators.contains("mod3"));
    }

    #[async_std::test]
    async fn test_load_admin_config_invalid_json() {
        let temp_dir = TempDir::new("config").unwrap();
        let invalid_content = r#"{
            "admins": ["user1"],
            "moderators": ["mod1"
        }"#;

        let config_path = create_test_config(&temp_dir, invalid_content).await;
        let config_result = load_admin_config(config_path.to_str().unwrap()).await;
        assert!(config_result.is_err());
    }

    #[async_std::test]
    async fn test_load_admin_config_empty_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let config_path = create_test_config(&temp_dir, "").await;
        let config_result = load_admin_config(config_path.to_str().unwrap()).await;
        assert!(config_result.is_err());
    }
}
