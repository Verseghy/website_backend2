use crate::AppState;
use async_graphql::{Response, ServerError, http::GraphiQLSource};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use prometheus::TextEncoder;
use sea_orm::TransactionTrait;
use std::sync::Arc;

const GRAPHQL_PATH: &str = "/graphql";

async fn graphql(State(state): State<AppState>, request: GraphQLRequest) -> GraphQLResponse {
    let request = request.into_inner();

    if request.operation_name == Some("IntrospectionQuery".into()) {
        return state.schema.execute(request).await.into();
    }

    let res = match state.database.begin().await {
        Ok(tx) => {
            let tx = Arc::new(tx);
            let res = state
                .schema
                .execute(
                    request
                        .data(Arc::clone(&tx))
                        .data(state.counter)
                        .data(state.config),
                )
                .await;

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

async fn graphiql() -> Html<String> {
    Html(GraphiQLSource::build().endpoint(GRAPHQL_PATH).finish())
}

async fn metrics(State(state): State<AppState>) -> Result<String, StatusCode> {
    let encoder = TextEncoder::new();
    let metric_families = state.prometheus_registry.gather();

    encoder
        .encode_to_string(&metric_families)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(GRAPHQL_PATH, get(graphiql))
        .route(GRAPHQL_PATH, post(graphql))
        .route("/metrics", get(metrics))
        .route("/readiness", get(|| async {}))
        .route("/liveness", get(|| async {}))
}
