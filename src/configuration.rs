#[derive(serde::Deserialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
}

#[derive(serde::Deserialize, Debug)]
pub struct DatabaseSettings {
    pub path: std::path::PathBuf,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "sqlite:{}/{}.db",
            self.path.to_string_lossy(),
            self.database_name
        )
    }
    pub fn db_path(&self) -> std::path::PathBuf {
        self.path.join(format!("{}.db", self.database_name))
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("configuration"))
        .build()
        .expect("Failed to build configuration object");
    settings.try_deserialize()
}
