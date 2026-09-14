use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::log::LevelFilter;

/// Opens the MySQL connection pool the resolvers query.
///
/// * `url` - MySQL connection URL, e.g. `mysql://user:password@host:3306/database`.
///
/// # Panics
///
/// Panics if the database cannot be reached. The service cannot answer GraphQL
/// requests without it, so startup fails fast instead.
pub async fn connect(url: &str) -> DatabaseConnection {
    tracing::info!("Trying to connect to database");

    let mut opts = ConnectOptions::new(url);
    opts.sqlx_logging_level(LevelFilter::Debug);
    let db = Database::connect(opts).await.unwrap();

    tracing::info!("Connected to database");

    db
}
