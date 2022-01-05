mod database;
mod entity;
mod graphql;
mod utils;

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use hyper::{body::Buf, Body, Method, Request, Response, Server, StatusCode};
use std::{convert::Infallible, future::Future, io, net::SocketAddr, time::Duration};
use tokio::signal::unix::{signal, SignalKind};
use tower::make::Shared;
use tower_http::add_extension::AddExtensionLayer;

use crate::graphql::create_schema;
use crate::graphql::Schema;
use crate::utils::num_threads;

const GRAPHQL_PATH: &str = "/graphql";

async fn handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let schema = req.extensions().get::<Schema>().unwrap().clone();
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
        (&Method::GET, "/readiness") => {
            *res.status_mut() = StatusCode::OK;
        }
        (&Method::GET, "/liveness") => {
            if let Ok(threads) = num_threads().await {
                if threads < 10000 {
                    *res.status_mut() = StatusCode::OK;
                } else {
                    *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                }
            } else {
                *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            }
        }
        _ => *res.status_mut() = StatusCode::NOT_FOUND,
    }

    Ok(res)
}

fn sigterm_signal() -> io::Result<impl Future<Output = ()>> {
    let mut stream = signal(SignalKind::terminate())?;

    Ok(async move {
        stream.recv().await;
        tracing::info!("Got SIGTERM, shutting down");
    })
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .with_env_filter("warn,website_backend=debug")
        .compact()
        .init();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let schema = create_schema().await;

    let service = tower::ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .layer(AddExtensionLayer::new(schema))
        .service_fn(handler);

    tracing::info!("Listening on port {}", addr.port());
    Server::bind(&addr)
        .serve(Shared::new(service))
        .with_graceful_shutdown(sigterm_signal()?)
        .await
        .expect("server error");

    Ok(())
}
