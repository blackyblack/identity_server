use std::{
    env,
    io::{Error, Write},
};

use identity_server::{routes, state::State};
use tide::Server;

pub const DEFAULT_PORT: u32 = 8080;
pub const DEFAULT_HOST: &str = "localhost";

#[async_std::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().expect("Should initialize env variables from .env");
    // use INFO log level by default, disable all libraries logging
    // use RUST_LOG env variable to override log level
    let log_env = env_logger::Env::default().default_filter_or("info,tide=off");
    env_logger::Builder::from_env(log_env)
        .format(|buf, record| {
            let ts = buf.timestamp_seconds();
            writeln!(buf, "[{}] {}: {}", record.level(), ts, record.args())
        })
        .init();
    let state = State::default();
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
}
