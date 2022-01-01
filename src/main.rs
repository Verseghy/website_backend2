mod database;
mod entity;
mod graphql;

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use hyper::service::{make_service_fn, service_fn};
use hyper::{body::Buf, Body, Method, Request, Response, Server, StatusCode};
use std::{convert::Infallible, net::SocketAddr};

use crate::graphql::create_schema;

const GRAPHQL_PATH: &str = "/graphql";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .init();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let schema = create_schema().await;

    let make_svc = make_service_fn(move |_conn| {
        let schema = schema.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let schema = schema.clone();
                async move {
                    let mut res = Response::new(Body::empty());

                    match (req.method(), req.uri().path()) {
                        (&Method::GET, GRAPHQL_PATH) => {
                            let source =
                                playground_source(GraphQLPlaygroundConfig::new(GRAPHQL_PATH));
                            *res.body_mut() = Body::from(source);
                        }
                        (&Method::POST, GRAPHQL_PATH) => {
                            *res.status_mut() = StatusCode::BAD_REQUEST;

                            let body = hyper::body::to_bytes(req.into_body()).await;
                            if let Ok(body) = body {
                                let reader = body.reader();
                                if let Ok(request) =
                                    serde_json::from_reader::<_, async_graphql::Request>(reader)
                                {
                                    let response = schema.execute(request).await;
                                    let response_json = serde_json::to_string(&response).unwrap();

                                    *res.body_mut() = Body::from(response_json);
                                    *res.status_mut() = StatusCode::OK;
                                }
                            }
                        }
                        _ => *res.status_mut() = StatusCode::NOT_FOUND,
                    }

                    Ok::<_, Infallible>(res)
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
