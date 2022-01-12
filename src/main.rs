mod database;
mod entity;
mod graphql;
mod http;
mod utils;

use crate::{graphql::create_schema, http::handlers};
use actix_web::{
    web::{self, Data},
    App, HttpServer,
};
use std::{io, net::SocketAddr};

const GRAPHQL_PATH: &str = "/graphql";

#[actix_web::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .with_env_filter("warn,website_backend=debug")
        .compact()
        .init();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let schema = create_schema().await;
    let database = database::connect().await;

    tracing::info!("Listening on port {}", addr.port());
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(schema.clone()))
            .app_data(Data::new(database.clone()))
            .route(GRAPHQL_PATH, web::post().to(handlers::graphql))
            .route(GRAPHQL_PATH, web::get().to(handlers::graphql_playground))
            .route("/readiness", web::get().to(handlers::readiness))
            .route("/liveness", web::get().to(handlers::liveness))
    })
    .bind(addr)?
    .run()
    .await
}
