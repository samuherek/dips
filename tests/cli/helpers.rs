use dips::configuration::get_configuration;
use sqlx::SqlitePool;
use std::path::PathBuf;

pub struct TestApp {
    db_pool: SqlitePool,
    src_path: PathBuf,
}

pub async fn setup_app() -> () {
    let config = get_configuration();
    println!("config {:?}", config);
    // TestApp {
    //
    // }
}
