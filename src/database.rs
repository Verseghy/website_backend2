use sea_orm::{Database, DatabaseConnection};

pub async fn connect() -> DatabaseConnection {
    Database::connect("mysql://root:secret@localhost:33061/backend")
        .await
        .unwrap()
}
