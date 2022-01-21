use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::env::{var, VarError};

fn get_db_uri() -> Result<String, VarError> {
    let username = var("DB_USERNAME")?;
    let password = var("DB_PASSWORD")?;
    let host = var("DB_HOST")?;
    let db = var("DB_DATABASE")?;

    Ok(format!("mysql://{}:{}@{}/{}", username, password, host, db))
}

pub async fn connect() -> DatabaseConnection {
    let uri = get_db_uri().expect("Could not create database URI");

    tracing::info!("Trying to connect to database");

    let opts = ConnectOptions::new(uri);
    let db = Database::connect(opts).await.unwrap();

    tracing::info!("Connected to database");

    db
}
