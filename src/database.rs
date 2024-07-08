use crate::configuration::Settings;
use sqlx::{Connection, SqliteConnection};

pub struct Database {
    conn: SqliteConnection,
}

impl Database {
    pub async fn connect(config: &Settings) -> Self {
        let conn_string = config.database.connection_string();
        println!("connection string {}", conn_string);
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
