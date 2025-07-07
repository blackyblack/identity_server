use std::{
    env,
    io::{Error, Write},
};

use identity_server::{
    config::{self, DEFAULT_CONFIG_PATH, DEFAULT_GENESIS_PATH},
    identity::IdentityService,
    routes::{self, State},
    storage,
    verify::{private_key_to_address, random_keypair},
};
use tide::Server;

pub const DEFAULT_PORT: u32 = 8080;
pub const DEFAULT_HOST: &str = "localhost";

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

    let server_private_key = match env::var("SERVER_PRIVATE_KEY") {
        Ok(key) if !key.is_empty() => match private_key_to_address(&key) {
            Ok(_) => key,
            Err(e) => {
                log::error!("Invalid SERVER_PRIVATE_KEY: {:?}", e);
                panic!("Invalid SERVER_PRIVATE_KEY: {}", e);
            }
        },
        _ => {
            log::warn!("SERVER_PRIVATE_KEY is not set or empty, generating a random key");
            let (key, _) = random_keypair();
            key
        }
    };
    log::info!(
        "Server address: {}",
        private_key_to_address(&server_private_key).expect("Should be valid private key")
    );

    let config = match config::load_config(DEFAULT_CONFIG_PATH).await {
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
    let storage = match storage::create_database_storage(
        config.admins.admins,
        config.admins.moderators,
    )
    .await
    {
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
        external_servers: config.external_servers,
    };

    log::info!("Starting identity server");
    if let Err(err) = start_server(state).await {
        log::error!("Failed to start server: {:?}", err);
        panic!("Failed to start server: {}", err);
    }
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
