//! Binary entry point: reads configuration from the environment, connects to
//! Redis and MySQL, and serves the API until a shutdown signal arrives.

use envconfig::Envconfig;
use std::{error::Error, net::SocketAddr};
use tokio::net::TcpListener;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use website_backend2::{
    AppState, Config, database,
    graphql::{RedisCache, build_schema},
    run,
};

fn init_logger() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(fmt::layer().with_line_number(true).with_filter(env_filter))
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_logger();

    let config = Config::init_from_env()?;

    let cache = RedisCache::new(&config.redis_url)
        .await
        .expect("Could not create redis cache");
    let schema = build_schema(cache);
    let database = database::connect(&config.database_url).await;

    let socket_addr = SocketAddr::new(config.bind_addr, config.bind_port);
    let state = AppState::new(schema, database, config);

    let listener = TcpListener::bind(socket_addr).await?;

    run(listener, state).await?;

    Ok(())
}
