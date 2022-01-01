use dotenv::{var, Result};
use sea_orm::{Database, DatabaseConnection};

fn get_db_uri() -> Result<String> {
    let username = var("DB_USERNAME")?;
    let password = var("DB_PASSWORD")?;
    let host = var("DB_HOST")?;
    let db = var("DB_DATABASE")?;

    Ok(format!("mysql://{}:{}@{}/{}", username, password, host, db))
}

pub async fn connect() -> DatabaseConnection {
    let uri = get_db_uri().expect("Could not create database URI");
    Database::connect(uri).await.unwrap()
}
