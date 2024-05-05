mod database;
mod entity;
mod graphql;
mod http;
mod utils;

use crate::{graphql::create_schema, http::handlers};
use actix_web::{
    middleware,
    web::{self, Data},
    App, HttpServer,
};
use prometheus::{IntCounterVec, Opts, Registry};
use std::{io, net::SocketAddr};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

const GRAPHQL_PATH: &str = "/graphql";

fn init_logger() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(fmt::layer().with_line_number(true).with_filter(env_filter))
        .init();
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    dotenv::dotenv().ok();
    init_logger();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

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

    tracing::info!("Listening on port {}", addr.port());
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .app_data(Data::new(schema.clone()))
            .app_data(Data::new(database.clone()))
            .app_data(Data::new(prometheus_registry.clone()))
            .app_data(Data::new(counter.clone()))
            .route(GRAPHQL_PATH, web::post().to(handlers::graphql))
            .route(GRAPHQL_PATH, web::get().to(handlers::graphql_playground))
            .route("/readiness", web::get().to(handlers::readiness))
            .route("/liveness", web::get().to(handlers::liveness))
            .route("/metrics", web::get().to(handlers::metrics))
    })
    .bind(addr)?
    .run()
    .await
}
