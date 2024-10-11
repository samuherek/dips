use crate::models::dir_context::RuntimeDirContext;
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
            return Environment::Production;
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
    /// Build the settings from possible different sources
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
    /// - sqlite://:memory:
    pub path: String,
}

impl DatabaseSettings {
    /// Build the database settings from all available configs
    pub fn build(env: &Environment) -> Self {
        let path = match env {
            Environment::Development => {
                if let Ok(path) = std::env::var("DEBUG_DB_PATH") {
                    path
                } else {
                    DB_NAME.to_string()
                }
            }
            Environment::Production => dirs::home_dir()
                .expect("Failed to find home directory")
                .join(".dips")
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
#[derive(Debug)]
pub struct Application {
    pub db_pool: SqlitePool,
    pub context_dir: RuntimeDirContext,
}

impl Application {
    pub async fn build(config: Settings) -> Result<Self, ConfigError> {
        let curr_path = std::env::current_dir().expect("Failed to read the current directory.");
        let db_pool = get_database_connection(&config).await?;
        migrate_database(&db_pool)
            .await
            .expect("Failed to initialize database");
        let context_dir =
            RuntimeDirContext::try_from(curr_path).expect("Failed to identify current context");

        Ok(Self {
            db_pool,
            context_dir,
        })
    }
}

/// Exclusivelly get the connetion to the database.
async fn get_database_connection(config: &Settings) -> Result<SqlitePool, ConfigError> {
    // TODO: Not sure if this is the right way to do this, but at the moment
    // we don't support custom setup, so we hardcode this value.
    // When we support custom setup for the database, the desting in memory
    // database will need to be rethoguth.
    if !config.database.path.contains(":memory:") {
        let database_path = Path::new(&config.database.path);
        if !database_path.exists() {
            return Err(ConfigError::Uninitialized);
        }
    }

    let db_pool = SqlitePool::connect(&config.database.connection_string())
        .await
        .expect("Failed to connect to the database.");

    Ok(db_pool)
}

/// Specific function to migrate the already established connection
pub async fn migrate_database(conn: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("PRAGMA foreign_keys = ON;")
        .execute(conn)
        .await?;
    sqlx::migrate!("./migrations").run(conn).await?;

    Ok(())
}
