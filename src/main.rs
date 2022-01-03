mod database;
mod entity;
mod graphql;
mod service;
mod utils;

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use hyper::{body::Buf, Body, Method, Request, Response, Server, StatusCode};
use std::{convert::Infallible, net::SocketAddr, time::Duration};
use tower::make::Shared;

use crate::graphql::create_schema;
use crate::graphql::Schema;
use crate::service::GraphQLService;

const GRAPHQL_PATH: &str = "/graphql";

async fn handler(req: Request<Body>, schema: Schema) -> Result<Response<Body>, Infallible> {
    let mut res = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, GRAPHQL_PATH) => {
            let source = playground_source(GraphQLPlaygroundConfig::new(GRAPHQL_PATH));
            *res.body_mut() = Body::from(source);
        }
        (&Method::POST, GRAPHQL_PATH) => {
            *res.status_mut() = StatusCode::BAD_REQUEST;

            let body = hyper::body::to_bytes(req.into_body()).await;
            if let Ok(body) = body {
                let reader = body.reader();
                if let Ok(request) = serde_json::from_reader::<_, async_graphql::Request>(reader) {
                    let response = schema.execute(request).await;
                    let response_json = serde_json::to_string(&response).unwrap();

                    *res.body_mut() = Body::from(response_json);
                    *res.status_mut() = StatusCode::OK;
                }
            }
        }
        _ => *res.status_mut() = StatusCode::NOT_FOUND,
    }

    Ok(res)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .init();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let schema = create_schema().await;

    let service = tower::ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .service(GraphQLService::new(handler, schema));

    Server::bind(&addr)
        .serve(Shared::new(service))
        .await
        .expect("server error");
}
