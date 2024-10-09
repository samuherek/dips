use dips::configuration::{Application, Environment, Settings};
use dips::models::dir_context::RuntimeDirContext;

#[derive(Debug)]
pub struct TestApp {
    temp_dir: tempfile::TempDir,
    context_dir: RuntimeDirContext,
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
        let context_dir = RuntimeDirContext::try_from(temp_dir.path())
            .expect("Failed to determine context from temp dir");

        TestApp {
            application,
            context_dir,
            temp_dir,
        }
    }

    pub fn application(&self) -> &Application {
        &self.application
    }
}
