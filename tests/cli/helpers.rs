use dips::configuration::{Application, Environment, Settings};

#[derive(Debug)]
pub struct TestApp {
    src_path: tempfile::TempDir,
    application: Application,
}

impl TestApp {
    pub async fn setup() -> Self {
        let settings = {
            let mut s = Settings::build(&Environment::current());
            s.database.path = "sqlite::memory:".to_string();
            s
        };
        let application = Application::build(settings)
            .await
            .expect("Failed to build the application.");
        let temp_dir = tempfile::TempDir::new().expect("Failed to create a temp directory.");

        println!("config {:?}", "hey");
        TestApp {
            application,
            src_path: temp_dir,
        }
    }

    pub fn application(&self) -> &Application {
        &self.application
    }
}
