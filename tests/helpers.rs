use dips::configuration::get_configuration;
use dips::database::Database;
use sqlx::SqlitePool;
use std::path::PathBuf;

pub struct TestApp {
    db_pool: SqlitePool,
    src_path: PathBuf,
}

pub async fn setup_app() -> () {
    let configuration = {
       let mut c = get_configuration();
       c.database.path = "sqlite::memory:"
    }
    let application = Application::build(configuration.clone());
    
    let db = Database::connect("sqlite::memory:");
    println!("config {:?}", vec![]);
    // TestApp {
    //
    // }
}
