use crate::{graphql::Schema, utils::num_threads, GRAPHQL_PATH};
use actix_web::{web, HttpResponse};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};

pub async fn graphql(schema: web::Data<Schema>, request: GraphQLRequest) -> GraphQLResponse {
    schema.execute(request.into_inner()).await.into()
}

pub async fn graphql_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(GraphQLPlaygroundConfig::new(
            GRAPHQL_PATH,
        )))
}

pub async fn readiness() -> HttpResponse {
    HttpResponse::Ok().body("")
}

pub async fn liveness() -> HttpResponse {
    if let Ok(threads) = num_threads().await {
        if threads < 10000 {
            tracing::debug!("Liveness thread count: {}", threads);
            return HttpResponse::Ok().body("");
        }
    }

    HttpResponse::InternalServerError().body("")
}
