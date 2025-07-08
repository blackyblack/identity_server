use std::{
    collections::HashSet,
    env,
    io::{Error, ErrorKind},
    sync::Arc,
};

use crate::{
    admins::{AdminStorage, db::DatabaseAdminStorage},
    identity::{
        UserAddress,
        proof::{db::DatabaseProofStorage, storage::ProofStorage},
        punish::{db::DatabasePenaltyStorage, storage::PenaltyStorage},
        vouch::{db::DatabaseVouchStorage, storage::VouchStorage},
    },
    servers::{db::DatabaseServerStorage, storage::ServerStorage},
    verify::nonce::{NonceManager, db::DatabaseNonceManager},
};

pub const DEFAULT_MYSQL_USER: &str = "root";
pub const DEFAULT_MYSQL_HOST: &str = "localhost";
pub const DEFAULT_MYSQL_PORT: u32 = 3306;
pub const DEFAULT_MYSQL_DATABASE: &str = "identity";

pub struct Storage {
    pub vouch_storage: Arc<dyn VouchStorage>,
    pub proof_storage: Arc<dyn ProofStorage>,
    pub penalty_storage: Arc<dyn PenaltyStorage>,
    pub admin_storage: Arc<dyn AdminStorage>,
    pub nonce_manager: Arc<dyn NonceManager>,
    pub server_storage: Arc<dyn ServerStorage>,
}

pub async fn create_database_storage(
    admins: HashSet<UserAddress>,
    moderators: HashSet<UserAddress>,
) -> Result<Storage, Error> {
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
    let admin_storage_connect = DatabaseAdminStorage::new(&db_url, admins, moderators)
        .await
        .map_err(|e| Error::new(ErrorKind::NotConnected, e.to_string()))?;
    let server_storage_connect = DatabaseServerStorage::new(&db_url)
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
        server_storage: Arc::new(server_storage_connect),
    })
}

pub fn setup_database_url() -> String {
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
