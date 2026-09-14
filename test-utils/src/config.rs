//! Where the services the integration tests need can be reached.

use envconfig::Envconfig;

/// Connection settings for the MySQL server and Redis the tests use.
///
/// [`TestApp::seeded`](crate::TestApp::seeded) and
/// [`TestApp::empty`](crate::TestApp::empty) read these from the environment.
/// The defaults match `compose.yaml`, so `docker compose up -d --wait` is all a
/// local run needs.
#[derive(Debug, Clone, Envconfig)]
pub struct TestConfig {
    /// URL of the MySQL *server*, without a database name (`TEST_MYSQL_URL`).
    ///
    /// Every test creates and drops its own database there, so the user needs
    /// permission to do both.
    #[envconfig(
        from = "TEST_MYSQL_URL",
        default = "mysql://root:secret@127.0.0.1:33061"
    )]
    pub mysql_url: String,
    /// Redis URL for the persisted-query cache (`TEST_REDIS_URL`).
    #[envconfig(from = "TEST_REDIS_URL", default = "redis://127.0.0.1:6379")]
    pub redis_url: String,
}
