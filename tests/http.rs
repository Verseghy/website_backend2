//! HTTP behaviour around the GraphQL endpoint: health checks, metrics, CORS,
//! compression, limits and routing.

use test_utils::prelude::*;

#[tokio::test]
async fn health_checks_respond_ok() {
    let app = TestApp::empty().await;

    for path in ["/liveness", "/readiness"] {
        let response = app.get(path).send().await.expect("request");

        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }
}

#[tokio::test]
async fn graphiql_is_served_on_get() {
    let app = TestApp::empty().await;

    let response = app.get("/graphql").send().await.expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.expect("body");
    assert!(body.to_lowercase().contains("graphiql"), "{body}");
}

#[tokio::test]
async fn metrics_count_queries_per_resource() {
    let app = TestApp::seeded().await;

    app.graphql("{ colleagues { id } }").await;
    app.graphql("{ colleagues { id } }").await;
    app.graphql(r#"{ page(slug: "rolunk") { id } }"#).await;

    let metrics = app
        .get("/metrics")
        .send()
        .await
        .expect("request")
        .text()
        .await
        .expect("body");
    assert!(
        metrics.contains(r#"query_req_count{resource="colleagues"} 2"#),
        "{metrics}"
    );
    assert!(
        metrics.contains(r#"query_req_count{resource="page"} 1"#),
        "{metrics}"
    );
}

#[tokio::test]
async fn cors_preflight_allows_any_origin() {
    let app = TestApp::empty().await;

    let response = app
        .request(Method::OPTIONS, "/graphql")
        .header(header::ORIGIN, "https://example.test")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .send()
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
}

#[tokio::test]
async fn responses_are_gzip_compressed_when_requested() {
    let app = TestApp::seeded().await;

    let response = app
        .post("/graphql")
        .header(header::ACCEPT_ENCODING, "gzip")
        .json(&json!({ "query": "{ colleagues { name jobs subjects } }" }))
        .send()
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
}

#[tokio::test]
async fn queries_above_the_complexity_limit_are_rejected() {
    // Each aliased `colleagues { id name }` costs 3, so 100 of them exceed 256.
    let app = TestApp::empty().await;
    let selections: String = (0..100)
        .map(|i| format!("c{i}: colleagues {{ id name }} "))
        .collect();

    let response = app.graphql(&format!("{{ {selections} }}")).await;

    assert!(
        error_messages(&response)
            .iter()
            .any(|message| message.contains("too complex")),
        "{response:#}"
    );
}

#[tokio::test]
async fn invalid_arguments_are_reported_as_graphql_errors() {
    let app = TestApp::empty().await;

    let response = app.graphql(r#"{ post(id: "one") { id } }"#).await;

    assert!(!error_messages(&response).is_empty(), "{response:#}");
}

#[tokio::test]
async fn introspection_query_is_answered() {
    let app = TestApp::empty().await;

    let response = app
        .graphql_request(json!({
            "operationName": "IntrospectionQuery",
            "query": "query IntrospectionQuery { __schema { queryType { name } } }",
        }))
        .await;

    assert_eq!(
        expect_data(&response)["__schema"]["queryType"]["name"],
        "Query"
    );
}

#[tokio::test]
async fn operation_named_introspection_query_can_select_data() {
    let app = TestApp::seeded().await;

    let response = app
        .post("/graphql")
        .json(&json!({
            "operationName": "IntrospectionQuery",
            "query": "query IntrospectionQuery { colleagues { id } }",
        }))
        .send()
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("JSON body");
    expect_data(&body);
}

#[tokio::test]
#[ignore = "bug: trailing slashes are trimmed after routing, so /graphql/ is not found"]
async fn trailing_slash_is_ignored() {
    let app = TestApp::seeded().await;

    let response = app
        .post("/graphql/")
        .json(&json!({ "query": "{ colleagues { id } }" }))
        .send()
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
}
