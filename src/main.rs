mod database;
mod entity;
mod graphql;
mod http;
mod utils;

use crate::{graphql::create_schema, utils::SignalHandler};
use axum::{Router, ServiceExt};
use graphql::Schema;
use prometheus::{IntCounterVec, Opts, Registry};
use sea_orm::DatabaseConnection;
use std::{
    error::Error,
    net::{IpAddr, Ipv6Addr, SocketAddr},
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, normalize_path::NormalizePath, ServiceBuilderExt};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

fn init_logger() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(fmt::layer().with_line_number(true).with_filter(env_filter))
        .init();
}

fn middlewares(router: Router) -> Router {
    let middlewares = ServiceBuilder::new()
        .catch_panic()
        .compression()
        .decompression()
        .layer(CorsLayer::permissive())
        .into_inner();

    router.layer(middlewares)
}

#[derive(Clone)]
struct AppState {
    pub schema: Schema,
    pub database: DatabaseConnection,
    pub counter: IntCounterVec,
    pub prometheus_registry: Registry,
}

const SOCKET_ADDR: SocketAddr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 3000);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    init_logger();

    let counter = IntCounterVec::new(
        Opts::new("query_req_count", "count of resource queries"),
        &["resource"],
    )
    .expect("Could not create Prometheus counter");

    let prometheus_registry = Registry::new();

    prometheus_registry
        .register(Box::new(counter.clone()))
        .expect("Could not register counter to Prometheus registry");

    let schema = create_schema().await;
    let database = database::connect().await;

    tracing::info!("Listening on port {}", SOCKET_ADDR.port());

    let state = AppState {
        schema,
        database,
        counter,
        prometheus_registry,
    };

    let app = http::routes().with_state(state);
    let app = middlewares(app);
    let app = NormalizePath::trim_trailing_slash(app);

    let listener = TcpListener::bind(SOCKET_ADDR).await?;

    axum::Server::from_tcp(listener.into_std()?)
        .expect("failed to start server")
        .serve(app.into_make_service())
        .with_graceful_shutdown(SignalHandler::new())
        .await?;

    Ok(())
}
