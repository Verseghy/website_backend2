use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub async fn connect() -> DatabaseConnection {
    tracing::info!("Trying to connect to database");

    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let opts = ConnectOptions::new(url);
    let db = Database::connect(opts).await.unwrap();

    tracing::info!("Connected to database");

    db
}
