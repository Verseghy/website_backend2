//! The application under test, served on an ephemeral local port.

use crate::{TestConfig, TestDatabase, seed};
use envconfig::Envconfig;
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};
use std::net::{Ipv4Addr, SocketAddr};
use tokio::{net::TcpListener, task::JoinHandle};
use website_backend2::{
    AppState, Config,
    graphql::{RedisCache, build_schema},
};

/// Storage base URL the application renders image URLs with during tests.
///
/// It is fixed so snapshots do not depend on the environment.
pub const STORAGE_BASE_URL: &str = "https://storage.test";

/// The rows a test database starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Data {
    /// All tables exist but are empty.
    Empty,
    /// The standard data set from [`seed`].
    Seeded,
}

/// A running instance of the application with a database of its own.
///
/// Dropping it stops the server and drops the database.
pub struct TestApp {
    address: SocketAddr,
    client: Client,
    server: JoinHandle<()>,
    database: TestDatabase,
}

impl TestApp {
    /// Starts the application on the [`seed`] data, with [`TestConfig`] read
    /// from the environment.
    pub async fn seeded() -> Self {
        Self::start(&config_from_env(), Data::Seeded).await
    }

    /// Starts the application on empty tables, with [`TestConfig`] read from
    /// the environment.
    pub async fn empty() -> Self {
        Self::start(&config_from_env(), Data::Empty).await
    }

    /// Starts the application.
    ///
    /// * `config` - where the MySQL server and Redis are.
    /// * `data` - which rows the new database starts with.
    ///
    /// # Panics
    ///
    /// Panics with a hint if MySQL or Redis cannot be reached, or if the seed
    /// data cannot be inserted.
    pub async fn start(config: &TestConfig, data: Data) -> Self {
        let database = TestDatabase::create(&config.mysql_url).await;

        if data == Data::Seeded {
            seed::insert(database.connection()).await;
        }

        let cache = RedisCache::new(&config.redis_url)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "could not connect to Redis in TEST_REDIS_URL: {error}\n\
                     Start the test services with `docker compose up -d --wait` (see docs/testing.md)."
                )
            });

        let app_config = Config {
            bind_addr: Ipv4Addr::LOCALHOST.into(),
            bind_port: 0,
            // Only the binary uses this to connect; the pool is passed in directly.
            database_url: String::new(),
            redis_url: config.redis_url.clone(),
            storage_base_url: STORAGE_BASE_URL.to_owned(),
        };
        let state = AppState::new(
            build_schema(cache),
            database.connection().clone(),
            app_config,
        );

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("could not bind a local port for the test server");
        let address = listener
            .local_addr()
            .expect("a bound listener has an address");
        let server = tokio::spawn(async move {
            axum::serve(listener, website_backend2::app(state))
                .await
                .expect("test server failed");
        });

        Self {
            address,
            client: Client::new(),
            server,
            database,
        }
    }

    /// Connection to this app's database, for inserting rows a test needs.
    pub fn db(&self) -> &DatabaseConnection {
        self.database.connection()
    }

    /// Absolute URL of `path` (such as `/graphql`) on the running app.
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }

    /// Starts a request with any method to `path`.
    pub fn request(&self, method: Method, path: &str) -> RequestBuilder {
        self.client.request(method, self.url(path))
    }

    /// Starts a GET request to `path`.
    pub fn get(&self, path: &str) -> RequestBuilder {
        self.request(Method::GET, path)
    }

    /// Starts a POST request to `path`.
    pub fn post(&self, path: &str) -> RequestBuilder {
        self.request(Method::POST, path)
    }

    /// Runs a GraphQL document without variables and returns the response body.
    ///
    /// * `query` - the GraphQL document.
    ///
    /// GraphQL errors are part of the returned body; see [`TestApp::graphql_request`]
    /// for what panics.
    pub async fn graphql(&self, query: &str) -> Value {
        self.graphql_with(query, json!({})).await
    }

    /// Runs a GraphQL document with variables and returns the response body.
    ///
    /// * `query` - the GraphQL document.
    /// * `variables` - JSON object of variable values.
    pub async fn graphql_with(&self, query: &str, variables: Value) -> Value {
        self.graphql_request(json!({ "query": query, "variables": variables }))
            .await
    }

    /// Posts a complete GraphQL request body to `/graphql` and returns the
    /// response body.
    ///
    /// * `body` - request object, for when a test needs `operationName` or
    ///   `extensions`.
    ///
    /// # Panics
    ///
    /// Panics if the server does not answer with HTTP 200 and a JSON body.
    /// Tests that expect another status should use [`TestApp::post`].
    pub async fn graphql_request(&self, body: Value) -> Value {
        let response = self
            .post("/graphql")
            .json(&body)
            .send()
            .await
            .expect("could not send the request to the test server");
        let status = response.status();
        let text = response
            .text()
            .await
            .expect("could not read the response body");

        assert_eq!(status, StatusCode::OK, "GraphQL request failed: {text}");

        serde_json::from_str(&text)
            .unwrap_or_else(|error| panic!("response is not JSON ({error}): {text}"))
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Stop serving before the fields, and with them the database, are dropped.
        self.server.abort();
    }
}

fn config_from_env() -> TestConfig {
    TestConfig::init_from_env()
        .unwrap_or_else(|error| panic!("invalid integration test configuration: {error}"))
}
