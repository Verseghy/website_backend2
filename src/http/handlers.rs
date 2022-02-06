use crate::{graphql::Schema, utils::num_threads, GRAPHQL_PATH};
use actix_web::{http::StatusCode, web, HttpResponse};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    Response, ServerError,
};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use sea_orm::{DatabaseConnection, TransactionTrait};
use std::sync::Arc;

pub async fn graphql(
    database: web::Data<DatabaseConnection>,
    schema: web::Data<Schema>,
    request: GraphQLRequest,
) -> GraphQLResponse {
    let request = request.into_inner();

    if request.operation_name == Some("IntrospectionQuery".into()) {
        return schema.execute(request).await.into();
    }

    let res = match database.begin().await {
        Ok(tx) => {
            let tx = Arc::new(tx);
            let res = schema.execute(request.data(Arc::clone(&tx))).await;

            if let Err(err) = Arc::try_unwrap(tx).unwrap().commit().await {
                tracing::error!("Could not commit transaction: {:?}", err);
            }

            res
        }
        Err(err) => {
            tracing::error!("Could not start transaction: {:?}", err);
            return Response::from_errors(vec![ServerError::new("Transaction begin failed", None)])
                .into();
        }
    };

    if res.is_err() {
        tracing::warn!("GraphQL request failed: {:?}", res.errors);
    }

    res.into()
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
        if threads < 10000 {
            return HttpResponse::new(StatusCode::OK);
        }
    }

    HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
}
