use crate::configuration::Settings;
use sqlx::{Connection, SqliteConnection};

pub struct Database {
    pub conn: SqliteConnection,
}

impl Database {
    pub async fn connect(config: &Settings) -> Self {
        let conn_string = config.database.connection_string();
        let conn = SqliteConnection::connect(&conn_string)
            .await
            .expect("Failed to connect to sqlite");
        Self { conn }
    }

    fn create_database_file(config: &Settings) {
        let file_path = config.database.db_path();

        if !file_path.exists() {
            std::fs::File::create(file_path).expect("Failed to create database file.");
        }
    }

    pub async fn init(config: &Settings) -> Self {
        Database::create_database_file(config);
        let mut db = Database::connect(config).await;

        sqlx::migrate!("./migrations")
            .run(&mut db.conn)
            .await
            .expect("Failed to migrate database.");

        db
    }
}
