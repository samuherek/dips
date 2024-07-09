use crate::configuration::Settings;
use sqlx::{Connection, SqliteConnection};

pub struct Database {
    conn: SqliteConnection,
}

impl Database {
    pub async fn connect(config: &Settings) -> Self {
        let conn_string = config.database.connection_string();
        let file_path = config.database.db_path(); 

        if !file_path.exists() {
            std::fs::File::create(file_path).expect("Failed to create database file.");
        }

        let conn = SqliteConnection::connect(&conn_string)
            .await
            .expect("Failed to connect to sqlite");
        Self { conn }
    }

    pub async fn init(&mut self) {
        sqlx::migrate!("./migrations")
            .run(&mut self.conn)
            .await
            .expect("Failed to migrate database.");
    }
}
