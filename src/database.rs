use crate::configuration::Settings;
use sqlx::SqlitePool;

pub struct Database {
    pub conn: SqlitePool,
}

impl Database {
    pub async fn connect(config: &Settings) -> Self {
        let conn_string = config.database.connection_string();
        let conn = SqlitePool::connect(&conn_string)
            .await
            .expect("Failed to connect to sqlite");

        sqlx::query("PRAGMA foreign_keys = ON;")
            .execute(&conn)
            .await
            .expect("Failed set foreign keys in sqlite.");

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
        let db = Database::connect(config).await;

        sqlx::migrate!("./migrations")
            .run(&db.conn)
            .await
            .expect("Failed to migrate database.");

        db
    }
}
