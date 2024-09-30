use sqlx::SqlitePool;
use std::path::Path;

static DB_NAME: &'static str = "dips.db";

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Dips is not initialized yet")]
    Uninitialized,
}

/// Helper function to figure out what environment is the application currently running. It will
/// create this value dynamically based on the compilation debug mode.
#[derive(Debug)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn current() -> Self {
        #[cfg(debug_assertions)]
        {
            return Environment::Development;
        }
        #[cfg(not(debug_assertions))]
        {
            return Environment::Production(dir);
        }
    }
}

/// This holds the user configurations of the application.
/// Right now, it holds hardcoded values as we don't support
/// custom setup settings. As there is nothing to support yet.
#[derive(Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
}

impl Settings {
    pub fn build(env: &Environment) -> Self {
        let database = DatabaseSettings::build(env);
        Self { database }
    }
}

/// This holds the configuration of the database.
/// In case we switch form sqlite than this holds the
/// coniguration values like the name, password, ...
///
/// In case we support custom configuration of the database
/// for the user, this is where we load the config to. And from
/// this unified interface, we build the settings.
#[derive(Debug)]
pub struct DatabaseSettings {
    /// Expect the path to be something along the lines of:
    /// - sqlite:///absolute/path/to/db.sqlite
    /// - sqlite://relative/path/to/db.sqlite
    /// - sqlite://memory:
    pub path: String,
}

impl DatabaseSettings {
    pub fn build(env: &Environment) -> Self {
        let path = match env {
            Environment::Development => DB_NAME.to_string(),
            Environment::Production => dirs::home_dir()
                .expect("Failed to find home directory")
                .join(DB_NAME)
                .display()
                .to_string(),
        };

        Self { path }
    }

    pub fn connection_string(&self) -> String {
        format!("sqlite://{}", self.path)
    }
}

/// This struct is dedicated to the application global values
/// that are shared across different functions.
/// It holds all the initialized objects like the database pool.
pub struct Application {
    pub db_pool: SqlitePool,
}

impl Application {
    pub async fn build(config: Settings) -> Result<Self, ConfigError> {
        let db_pool = get_database_connection(&config).await?;
            migrate_database(&db_pool)
        .await
        .expect("Failed to initialize database");


        Ok(Self { db_pool })
    }
}

async fn get_database_connection(config: &Settings) -> Result<SqlitePool, ConfigError> {
    let database_path = Path::new(&config.database.path);
    if !database_path.exists() {
        return Err(ConfigError::Uninitialized);
        // std::fs::File::create(database_path).expect("Failed to create database file.");
    }

    let db_pool = SqlitePool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to the database.");

    Ok(db_pool)
}

pub async fn migrate_database(conn: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(conn)
        .await?;
    sqlx::migrate!("./migrations").run(conn).await?;

    Ok(())
}
