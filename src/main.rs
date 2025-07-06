use std::{
    env,
    io::{Error, ErrorKind, Write},
    sync::Arc,
};

use identity_server::{
    admins::{AdminStorage, db::DatabaseAdminStorage},
    config::{self, DEFAULT_CONFIG_PATH, DEFAULT_GENESIS_PATH},
    identity::{
        IdentityService, IdtAmount, UserAddress,
        proof::{db::DatabaseProofStorage, storage::ProofStorage},
        punish::{db::DatabasePenaltyStorage, storage::PenaltyStorage},
        vouch::{db::DatabaseVouchStorage, storage::VouchStorage},
    },
    routes::{self, State},
    verify::nonce::{NonceManager, db::DatabaseNonceManager},
};
use tide::Server;

pub const DEFAULT_PORT: u32 = 8080;
pub const DEFAULT_HOST: &str = "localhost";

pub const DEFAULT_MYSQL_USER: &str = "root";
pub const DEFAULT_MYSQL_HOST: &str = "localhost";
pub const DEFAULT_MYSQL_PORT: u32 = 3306;
pub const DEFAULT_MYSQL_DATABASE: &str = "identity";

struct Storage {
    vouch_storage: Arc<dyn VouchStorage>,
    proof_storage: Arc<dyn ProofStorage>,
    penalty_storage: Arc<dyn PenaltyStorage>,
    admin_storage: Arc<dyn AdminStorage>,
    nonce_manager: Arc<dyn NonceManager>,
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
    let mut config = match config::load_config(DEFAULT_CONFIG_PATH).await {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to load configuration: {:?}", e);
            panic!("Failed to load configuration: {}", e);
        }
    };
    let genesis = match config::load_genesis(DEFAULT_GENESIS_PATH).await {
        Ok(genesis) => genesis,
        Err(e) => {
            log::error!("Failed to load genesis configuration: {:?}", e);
            panic!("Failed to load genesis configuration: {}", e);
        }
    };
    let _external_servers = config.external_servers.clone();
    let storage = match create_storage(config.admins).await {
        Ok(storage) => storage,
        Err(e) => {
            log::error!("Failed to connect to database: {:?}", e);
            panic!("Failed to connect to database: {}", e);
        }
    };

    let identity_service = IdentityService {
        vouches: storage.vouch_storage,
        proofs: storage.proof_storage,
        penalties: storage.penalty_storage,
    };
    identity_service
        .set_genesis(genesis)
        .await
        .unwrap_or_else(|e| {
            log::error!("Failed to set genesis balances: {:?}", e);
            panic!("Failed to set genesis balances: {}", e);
        });

    let state = State {
        identity_service,
        admin_storage: storage.admin_storage,
        nonce_manager: storage.nonce_manager,
    };

    log::info!("Starting identity server");
    if let Err(err) = start_server(state).await {
        log::error!("Failed to start server: {:?}", err);
        panic!("Failed to start server: {}", err);
    }
}

async fn create_storage(config: config::AdminsSection) -> Result<Storage, Error> {
    let db_url = setup_database_url();
    let vouch_storage_connect = DatabaseVouchStorage::new(&db_url)
        .await
        .map_err(|e| Error::new(ErrorKind::NotConnected, e.to_string()))?;
    let proof_storage_connect = DatabaseProofStorage::new(&db_url)
        .await
        .map_err(|e| Error::new(ErrorKind::NotConnected, e.to_string()))?;
    let penalty_storage_connect = DatabasePenaltyStorage::new(&db_url)
        .await
        .map_err(|e| Error::new(ErrorKind::NotConnected, e.to_string()))?;
    let admin_storage_connect =
        DatabaseAdminStorage::new(&db_url, config.admins, config.moderators)
            .await
            .map_err(|e| Error::new(ErrorKind::NotConnected, e.to_string()))?;
    let nonce_manager = DatabaseNonceManager::new(&db_url)
        .await
        .map_err(|e| Error::new(ErrorKind::NotConnected, e.to_string()))?;
    Ok(Storage {
        vouch_storage: Arc::new(vouch_storage_connect),
        proof_storage: Arc::new(proof_storage_connect),
        penalty_storage: Arc::new(penalty_storage_connect),
        admin_storage: Arc::new(admin_storage_connect),
        nonce_manager: Arc::new(nonce_manager),
    })
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
    use super::*;
    use async_std::{fs::File, io::WriteExt};
    use std::path::PathBuf;
    use tempdir::TempDir;

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

    #[async_std::test]
    async fn test_load_config_nonexistent_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = temp_dir.path().join("nonexistent.json");
        let cfg = config::load_config(path.to_str().unwrap()).await.unwrap();
        assert!(cfg.admins.admins.is_empty());
        assert!(cfg.admins.moderators.is_empty());
        assert!(cfg.external_servers.servers.is_empty());
    }

    #[async_std::test]
    async fn test_load_config_valid_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let content = r#"{
            "admins": {"admins": ["user"], "moderators": ["mod"]},
            "external_servers": {"allow_all": false, "servers": [{"url": "http://a.com"}]}
        }"#;
        let path = create_test_config(&temp_dir, content).await;
        let cfg = config::load_config(path.to_str().unwrap()).await.unwrap();
        assert_eq!(cfg.admins.admins.len(), 1);
        assert_eq!(cfg.admins.moderators.len(), 1);
        assert_eq!(cfg.external_servers.servers.len(), 1);
    }

    #[async_std::test]
    async fn test_load_config_invalid_json() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_config(&temp_dir, "{").await;
        assert!(config::load_config(path.to_str().unwrap()).await.is_err());
    }

    #[async_std::test]
    async fn test_load_config_empty_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_config(&temp_dir, "").await;
        assert!(config::load_config(path.to_str().unwrap()).await.is_err());
    }

    #[async_std::test]
    async fn test_load_genesis_nonexistent_file() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = temp_dir.path().join("nonexistent.json");
        let balances = config::load_genesis(path.to_str().unwrap()).await.unwrap();
        assert!(balances.is_empty());
    }

    #[async_std::test]
    async fn test_load_genesis_valid() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_genesis_config(&temp_dir, "{\"alice\":1}").await;
        let balances = config::load_genesis(path.to_str().unwrap()).await.unwrap();
        assert_eq!(balances["alice"], 1);
    }

    #[async_std::test]
    async fn test_load_genesis_invalid() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_genesis_config(&temp_dir, "{").await;
        assert!(config::load_genesis(path.to_str().unwrap()).await.is_err());
    }

    #[async_std::test]
    async fn test_load_genesis_empty() {
        let temp_dir = TempDir::new("config").unwrap();
        let path = create_test_genesis_config(&temp_dir, "").await;
        assert!(config::load_genesis(path.to_str().unwrap()).await.is_err());
    }
}
