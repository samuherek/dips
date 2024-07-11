use std::path::PathBuf;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
}

impl Settings {
    fn new(config_directory: &PathBuf) -> Self {
        Settings {
            database: DatabaseSettings {
                directory: config_directory.clone(),
                name: "dips".to_string(),
            },
        }
    }

    pub fn init() -> Self {
        let configuration_directory = get_config_directory();
        let config_file_path = configuration_directory.join("config.yaml");

        if config_file_path.exists() {
            println!("Looks like your dips has already been initialized.");
            std::process::exit(0);
        }

        let settings = Settings::new(&configuration_directory);
        let settings_yaml =
            serde_yaml::to_string(&settings).expect("Failed to serialize yaml config");

        std::fs::create_dir_all(&configuration_directory)
            .expect("Failed to create settings directory.");

        std::fs::write(config_file_path, settings_yaml)
            .expect("Failed to save configuration settings");

        settings
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct DatabaseSettings {
    pub directory: std::path::PathBuf,
    pub name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "sqlite:{}/{}.db",
            self.directory.to_string_lossy(),
            self.name
        )
    }
    pub fn db_path(&self) -> std::path::PathBuf {
        self.directory.join(format!("{}.db", self.name))
    }
}

fn get_config_directory() -> PathBuf {
    if cfg!(debug_assertions) {
        std::env::current_dir().expect("Failed to read current directory")
    } else {
        dirs::home_dir()
            .expect("Failed to find home directory")
            .join(".dips")
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let configuration_directory = get_config_directory();

    let settings = config::Config::builder()
        .add_source(config::File::from(
            configuration_directory.join("config.yaml"),
        ))
        .build()
        .expect("Failed to build configuration object.");

    settings.try_deserialize()
}
