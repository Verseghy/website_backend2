use crate::{graphql::Schema, utils::num_threads, GRAPHQL_PATH};
use actix_web::{http::StatusCode, web, HttpResponse};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::sync::Arc;

pub async fn graphql(
    database: web::Data<DatabaseConnection>,
    schema: web::Data<Schema>,
    request: GraphQLRequest,
) -> GraphQLResponse {
    let request = request.into_inner();

    let tx = Arc::new(database.begin().await.unwrap());
    let res = schema.execute(request.data(Arc::clone(&tx))).await.into();
    Arc::try_unwrap(tx).unwrap().commit().await.unwrap();

    res
}

pub async fn graphql_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(GraphQLPlaygroundConfig::new(
            GRAPHQL_PATH,
        )))
}

pub async fn readiness() -> HttpResponse {
    HttpResponse::new(StatusCode::OK)
}

pub async fn liveness() -> HttpResponse {
    if let Ok(threads) = num_threads().await {
        tracing::debug!("Liveness thread count: {}", threads);
        if threads < 10000 {
            return HttpResponse::new(StatusCode::OK);
        }
    }

    HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
}
