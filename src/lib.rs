//! Website Backend 2: the read-only GraphQL API behind the Verseghy school website.
//!
//! The crate is a library with a thin binary (`src/main.rs`) on top, so the
//! complete HTTP application can be built from tests. The convention this module
//! embodies: nothing in the library reads the process environment or opens
//! connections on its own. The binary loads [`Config`], connects to MySQL and
//! Redis, and passes the results into [`graphql::build_schema`] and
//! [`AppState::new`]; tests do the same with their own databases and caches.

pub mod database;
pub mod entity;
pub mod graphql;
mod http;
mod utils;

use crate::{graphql::Schema, utils::SignalHandler};
use axum::Router;
use envconfig::Envconfig;
use prometheus::{IntCounterVec, Opts, Registry};
use sea_orm::DatabaseConnection;
use std::net::IpAddr;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{ServiceBuilderExt, cors::CorsLayer};

/// Runtime configuration.
///
/// The binary reads it from the environment once at startup with
/// [`Envconfig::init_from_env`]; library code only ever receives it as a value.
#[derive(Debug, Clone, Envconfig)]
pub struct Config {
    /// Address the HTTP server binds to (`BIND_ADDR`, default `::`).
    #[envconfig(from = "BIND_ADDR", default = "::")]
    pub bind_addr: IpAddr,
    /// Port the HTTP server binds to (`BIND_PORT`, default `3000`).
    #[envconfig(from = "BIND_PORT", default = "3000")]
    pub bind_port: u16,
    /// MySQL connection URL (`DATABASE_URL`, required).
    #[envconfig(from = "DATABASE_URL")]
    pub database_url: String,
    /// Redis connection URL backing the persisted-query cache (`REDIS_URL`, required).
    #[envconfig(from = "REDIS_URL")]
    pub redis_url: String,
    /// Base URL uploaded images are served from (`STORAGE_BASE_URL`, required).
    ///
    /// Image fields are rendered as `{storage_base_url}/{directory}/{file}`, so
    /// the value is expected not to end with a slash.
    #[envconfig(from = "STORAGE_BASE_URL")]
    pub storage_base_url: String,
}

/// State shared by every HTTP handler.
#[derive(Clone)]
pub struct AppState {
    /// GraphQL schema served at `/graphql`.
    pub schema: Schema,
    /// MySQL pool; every GraphQL request runs inside its own transaction on it.
    pub database: DatabaseConnection,
    /// Runtime configuration, exposed to resolvers (e.g. for image URLs).
    pub config: Config,
    /// Per-resource request counter exported at `/metrics`.
    pub counter: IntCounterVec,
    /// Registry the `/metrics` endpoint renders.
    pub prometheus_registry: Registry,
}

impl AppState {
    /// Assembles handler state and registers the application's Prometheus metrics.
    ///
    /// * `schema` - schema built by [`graphql::build_schema`].
    /// * `database` - MySQL connection pool the resolvers query.
    /// * `config` - runtime configuration made available to resolvers.
    ///
    /// Each state gets its own Prometheus registry, so independent instances
    /// (such as tests running in parallel) never share counters.
    pub fn new(schema: Schema, database: DatabaseConnection, config: Config) -> Self {
        let counter = IntCounterVec::new(
            Opts::new("query_req_count", "count of resource queries"),
            &["resource"],
        )
        .expect("Could not create Prometheus counter");

        let prometheus_registry = Registry::new();

        prometheus_registry
            .register(Box::new(counter.clone()))
            .expect("Could not register counter to Prometheus registry");

        Self {
            schema,
            database,
            config,
            counter,
            prometheus_registry,
        }
    }
}

fn middlewares(router: Router) -> Router {
    let middlewares = ServiceBuilder::new()
        .catch_panic()
        .compression()
        .decompression()
        .layer(CorsLayer::permissive())
        .trim_trailing_slash()
        .into_inner();

    router.layer(middlewares)
}

/// Builds the complete HTTP application: routes, handler state and middleware.
///
/// * `state` - handler state, see [`AppState::new`].
///
/// This is exactly what [`run`] serves, so requests sent to it exercise routing
/// and middleware as well as the GraphQL layer.
pub fn app(state: AppState) -> Router {
    middlewares(http::routes().with_state(state))
}

/// Serves the application until SIGINT, SIGTERM or SIGQUIT is received.
///
/// * `listener` - bound TCP listener. Binding is left to the caller so the
///   address comes from [`Config`] in production and can be an ephemeral port
///   in tests.
/// * `state` - handler state, see [`AppState::new`].
pub async fn run(listener: TcpListener, state: AppState) -> std::io::Result<()> {
    tracing::info!("Listening on port {}", listener.local_addr()?.port());

    axum::serve(listener, app(state))
        .with_graceful_shutdown(SignalHandler::new())
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, net::Ipv4Addr, net::Ipv6Addr};

    /// The variables that have no default, with plausible values.
    fn required_vars() -> HashMap<String, String> {
        [
            (
                "DATABASE_URL",
                "mysql://backend:secret@localhost:3306/backend",
            ),
            ("REDIS_URL", "redis://localhost"),
            ("STORAGE_BASE_URL", "https://example.test/storage"),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
    }

    #[test]
    fn required_values_are_read_and_defaults_bind_all_interfaces_on_port_3000() {
        let config = Config::init_from_hashmap(&required_vars()).unwrap();

        assert_eq!(config.bind_addr, IpAddr::V6(Ipv6Addr::UNSPECIFIED));
        assert_eq!(config.bind_port, 3000);
        assert_eq!(
            config.database_url,
            "mysql://backend:secret@localhost:3306/backend"
        );
        assert_eq!(config.redis_url, "redis://localhost");
        assert_eq!(config.storage_base_url, "https://example.test/storage");
    }

    #[test]
    fn bind_address_and_port_can_be_overridden() {
        let mut vars = required_vars();
        vars.insert("BIND_ADDR".to_owned(), "127.0.0.1".to_owned());
        vars.insert("BIND_PORT".to_owned(), "8080".to_owned());

        let config = Config::init_from_hashmap(&vars).unwrap();

        assert_eq!(config.bind_addr, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(config.bind_port, 8080);
    }

    #[test]
    fn each_missing_required_variable_is_named_in_the_error() {
        for name in ["DATABASE_URL", "REDIS_URL", "STORAGE_BASE_URL"] {
            let mut vars = required_vars();
            vars.remove(name);

            assert_eq!(
                Config::init_from_hashmap(&vars).unwrap_err(),
                envconfig::Error::EnvVarMissing { name },
            );
        }
    }

    #[test]
    fn unparsable_values_are_named_in_the_error() {
        for (name, value) in [
            ("BIND_PORT", "70000"),
            ("BIND_PORT", "http"),
            ("BIND_ADDR", "localhost"),
        ] {
            let mut vars = required_vars();
            vars.insert(name.to_owned(), value.to_owned());

            assert_eq!(
                Config::init_from_hashmap(&vars).unwrap_err(),
                envconfig::Error::ParseError { name },
                "{name}={value}",
            );
        }
    }
}
