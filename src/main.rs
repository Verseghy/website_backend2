mod database;
mod entity;
mod graphql;
mod utils;

use actix_web::{
    web::{self, Data},
    App, HttpResponse, HttpServer, Responder,
};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use std::io;

use crate::graphql::create_schema;
use crate::graphql::Schema;
use crate::utils::num_threads;

const GRAPHQL_PATH: &str = "/graphql";
const PORT: u16 = 3000;

async fn graphql(schema: Data<Schema>, request: GraphQLRequest) -> GraphQLResponse {
    schema.execute(request.into_inner()).await.into()
}

async fn graphql_playground() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(GraphQLPlaygroundConfig::new(
            GRAPHQL_PATH,
        )))
}

async fn readiness() -> HttpResponse {
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    HttpResponse::Ok().body("")
}

async fn liveness() -> HttpResponse {
    if let Ok(threads) = num_threads().await {
        if threads < 10000 {
            tracing::debug!("Liveness thread count: {}", threads);
            return HttpResponse::Ok().body("");
        }
    }

    HttpResponse::InternalServerError().body("")
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .with_env_filter("warn,website_backend=debug")
        .compact()
        .init();

    let schema = create_schema().await;

    tracing::info!("Listening on port {}", PORT);
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(schema.clone()))
            .route(GRAPHQL_PATH, web::post().to(graphql))
            .route(GRAPHQL_PATH, web::get().to(graphql_playground))
            .route("/readiness", web::get().to(readiness))
            .route("/liveness", web::get().to(liveness))
    })
    .bind(("0.0.0.0", PORT))?
    .run()
    .await
}
