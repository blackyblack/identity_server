use std::{
    collections::HashSet,
    env,
    fs,
    io::{Error, Write},
    sync::Arc,
};

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

#[derive(Deserialize)]
struct AdminConfig {
    admins: Vec<String>,
    moderators: Vec<String>,
}

fn load_admin_config() -> (HashSet<String>, HashSet<String>) {
    match fs::read_to_string("admins.json") {
        Ok(content) => match serde_json::from_str::<AdminConfig>(&content) {
            Ok(cfg) => (
                cfg.admins.into_iter().collect(),
                cfg.moderators.into_iter().collect(),
            ),
            Err(err) => {
                log::error!("Failed to parse admins.json: {}", err);
                (HashSet::new(), HashSet::new())
            }
        },
        Err(err) => {
            log::warn!("Failed to read admins.json: {}", err);
            (HashSet::new(), HashSet::new())
        }
    }
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
    let (admins, moderators) = load_admin_config();

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
        admin_storage: Arc::new(InMemoryAdminStorage::new(admins, moderators)),
        nonce_manager: Arc::new(nonce_manager),
    };
    log::info!("Starting identity server");
    if let Err(err) = start_server(state).await {
        log::error!("Failed to start server: {:?}", err);
        panic!("Failed to start server: {}", err);
    }
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
