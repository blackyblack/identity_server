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
    verify::nonce::InMemoryNonceManager,
};
use serde::Deserialize;
use tide::Server;

pub const DEFAULT_PORT: u32 = 8080;
pub const DEFAULT_HOST: &str = "localhost";

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
async fn main() -> Result<(), Error> {
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
    let state = State {
        identity_service: IdentityService::default(),
        admin_storage: Arc::new(InMemoryAdminStorage::new(admins, moderators)),
        nonce_manager: Arc::new(InMemoryNonceManager::default()),
    };
    log::info!("Starting identity server");
    start_server(state).await
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
