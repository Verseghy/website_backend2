//! Integration-test harness for website_backend2.
//!
//! Each [`TestApp`] serves the real application router
//! ([`website_backend2::app`]) on an ephemeral local port, backed by a MySQL
//! database created for that test alone. The database's tables are generated
//! from the application's sea-orm entities, so the entities double as the
//! schema definition the tests trust, and they have to be kept in line with
//! production.
//!
//! Tests are independent of each other. They share only the MySQL server and
//! Redis, and Redis holds nothing but persisted queries keyed by the hash of
//! their text, so one test cannot observe another's data.
//!
//! The services are expected at the addresses in `compose.yaml`; see
//! [`TestConfig`] to point the tests elsewhere.

mod app;
mod config;
mod database;
pub mod prelude;
mod response;
pub mod seed;
mod snapshot;

pub use app::{Data, STORAGE_BASE_URL, TestApp};
pub use config::TestConfig;
pub use database::TestDatabase;
pub use response::{error_messages, expect_data, node_ids, unique_name};

/// Crates the exported macros expand to, so tests need no direct dependency on
/// them. Not part of the public API.
#[doc(hidden)]
pub mod macro_support {
    pub use insta;
}
