//! Throw-away MySQL databases, one per test.

use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Schema};
use std::{fmt, thread};
use uuid::Uuid;
use website_backend2::entity::{
    canteen_data, canteen_menus, canteen_pivot_menus_data, colleagues_data, events_data,
    menu_items, pages, posts_authors, posts_data, posts_labels, posts_pivot_labels_data,
};

/// Most connections the pool of one test database opens.
///
/// nextest runs every test in its own process and many processes at once, so
/// the pools are kept small to stay well below the server's `max_connections`.
const MAX_CONNECTIONS: u32 = 4;

/// A MySQL database that exists as long as this value does.
///
/// [`TestDatabase::create`] makes a uniquely named database containing every
/// application table; dropping the value drops the database again.
pub struct TestDatabase {
    server: ServerUrl,
    name: String,
    connection: DatabaseConnection,
}

impl TestDatabase {
    /// Creates a uniquely named database with all application tables and no rows.
    ///
    /// * `server_url` - MySQL server URL without a database name, such as
    ///   `mysql://root:secret@127.0.0.1:33061`. The user needs permission to
    ///   create and drop databases.
    ///
    /// The database uses `utf8mb4_unicode_ci`, the collation the Laravel
    /// backend that created the production tables was configured with. It
    /// decides how `LIKE` searches treat case and accents.
    ///
    /// # Panics
    ///
    /// Panics with a hint if the URL is not a server URL, the server cannot be
    /// reached, or a table cannot be created: a test cannot run without its
    /// database.
    pub async fn create(server_url: &str) -> Self {
        let server = ServerUrl::parse(server_url).unwrap_or_else(|message| panic!("{message}"));
        let name = format!("test_{}", Uuid::new_v4().simple());

        let admin = Database::connect(server.to_string())
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "could not connect to the MySQL server in TEST_MYSQL_URL: {error}\n\
                     Start the test services with `docker compose up -d --wait` (see docs/testing.md)."
                )
            });
        admin
            .execute_unprepared(&format!(
                "CREATE DATABASE `{name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
            ))
            .await
            .unwrap_or_else(|error| panic!("could not create database {name}: {error}"));
        // Best effort: the pool is released when `admin` is dropped anyway.
        let _ = admin.close().await;

        let mut options = ConnectOptions::new(server.database_url(&name));
        options.max_connections(MAX_CONNECTIONS).sqlx_logging(false);
        let connection = Database::connect(options)
            .await
            .unwrap_or_else(|error| panic!("could not connect to database {name}: {error}"));

        // Construct the value before creating tables, so the database is
        // dropped even if a statement below panics.
        let database = Self {
            server,
            name,
            connection,
        };
        create_tables(&database.connection).await;
        database
    }

    /// Connection pool for this database.
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// Name of the database on the server.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        // `Drop` cannot await, and it normally runs inside the test's Tokio
        // runtime, where blocking on a future panics. Run the statement on a
        // separate thread with a runtime of its own instead.
        let server_url = self.server.to_string();
        let statement = format!("DROP DATABASE IF EXISTS `{}`", self.name);

        let outcome = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;

            runtime.block_on(async {
                let admin = Database::connect(server_url)
                    .await
                    .map_err(|error| error.to_string())?;
                admin
                    .execute_unprepared(&statement)
                    .await
                    .map_err(|error| error.to_string())?;
                admin.close().await.map_err(|error| error.to_string())
            })
        })
        .join();

        // Panicking in `drop` would abort a test that is already failing, so
        // report problems instead. Leftover `test_*` databases are harmless.
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(error)) => eprintln!("could not drop test database {}: {error}", self.name),
            Err(_) => eprintln!("dropping test database {} panicked", self.name),
        }
    }
}

/// Creates every application table, parents before the tables whose foreign
/// keys reference them.
///
/// The list mirrors `src/entity/mod.rs`; a new entity must be added to both.
async fn create_tables(connection: &DatabaseConnection) {
    let backend = connection.get_database_backend();
    let schema = Schema::new(backend);

    let statements = [
        schema.create_table_from_entity(canteen_data::Entity),
        schema.create_table_from_entity(canteen_menus::Entity),
        schema.create_table_from_entity(canteen_pivot_menus_data::Entity),
        schema.create_table_from_entity(colleagues_data::Entity),
        schema.create_table_from_entity(events_data::Entity),
        schema.create_table_from_entity(pages::Entity),
        schema.create_table_from_entity(menu_items::Entity),
        schema.create_table_from_entity(posts_authors::Entity),
        schema.create_table_from_entity(posts_labels::Entity),
        schema.create_table_from_entity(posts_data::Entity),
        schema.create_table_from_entity(posts_pivot_labels_data::Entity),
    ];

    for statement in &statements {
        let statement = backend.build(statement);
        connection
            .execute(statement.clone())
            .await
            .unwrap_or_else(|error| panic!("could not create a table: {error}\n{statement}"));
    }
}

/// A validated `mysql://` URL that names a server but no database.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ServerUrl {
    /// Scheme and authority, e.g. `mysql://root:secret@127.0.0.1:33061`.
    base: String,
    /// Query string without the leading `?`; empty if there is none.
    query: String,
}

impl ServerUrl {
    /// Validates `url`. The error message says what to change.
    fn parse(url: &str) -> Result<Self, String> {
        let Some(rest) = url.strip_prefix("mysql://") else {
            return Err("TEST_MYSQL_URL must be a mysql:// URL".to_owned());
        };
        let (rest, query) = rest.split_once('?').unwrap_or((rest, ""));
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));

        if authority.is_empty() {
            return Err("TEST_MYSQL_URL must include a host".to_owned());
        }
        if !path.is_empty() {
            return Err(format!(
                "TEST_MYSQL_URL must not name a database (found {path:?}): every test creates its own"
            ));
        }

        Ok(Self {
            base: format!("mysql://{authority}"),
            query: query.to_owned(),
        })
    }

    /// URL of the database `name` on this server.
    fn database_url(&self, name: &str) -> String {
        match self.query.as_str() {
            "" => format!("{}/{name}", self.base),
            query => format!("{}/{name}?{query}", self.base),
        }
    }
}

impl fmt::Display for ServerUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.query.as_str() {
            "" => f.write_str(&self.base),
            query => write!(f, "{}?{query}", self.base),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_url_without_database_is_accepted() {
        let url = ServerUrl::parse("mysql://root:secret@127.0.0.1:33061").unwrap();

        assert_eq!(url.to_string(), "mysql://root:secret@127.0.0.1:33061");
        assert_eq!(
            url.database_url("test_1"),
            "mysql://root:secret@127.0.0.1:33061/test_1"
        );
    }

    #[test]
    fn trailing_slash_is_accepted() {
        let url = ServerUrl::parse("mysql://root@db:3306/").unwrap();

        assert_eq!(url.database_url("test_1"), "mysql://root@db:3306/test_1");
    }

    #[test]
    fn query_string_is_kept() {
        let url = ServerUrl::parse("mysql://root@db:3306?ssl-mode=disabled").unwrap();

        assert_eq!(url.to_string(), "mysql://root@db:3306?ssl-mode=disabled");
        assert_eq!(
            url.database_url("test_1"),
            "mysql://root@db:3306/test_1?ssl-mode=disabled"
        );
    }

    #[test]
    fn other_schemes_are_rejected() {
        let error = ServerUrl::parse("postgres://root@db:5432").unwrap_err();

        assert!(error.contains("mysql://"), "{error}");
    }

    #[test]
    fn url_naming_a_database_is_rejected() {
        let error = ServerUrl::parse("mysql://root@db:3306/backend").unwrap_err();

        assert!(error.contains("\"backend\""), "{error}");
    }

    #[test]
    fn url_without_host_is_rejected() {
        let error = ServerUrl::parse("mysql:///backend").unwrap_err();

        assert!(error.contains("host"), "{error}");
    }
}
