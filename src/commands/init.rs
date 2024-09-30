use crate::configuration::{Application, Settings};
use std::path::Path;

pub async fn init(settings: Settings) {
    let db_path = Path::new(&settings.database.path);
    if db_path.parent().is_none() {
        panic!("Failed to resolve the parent directory for the database.");
    }

    if !db_path.exists() {
        std::fs::File::create(db_path).expect("Failed to create database file.");
    }

    Application::build(settings)
        .await
        .expect("Failed to build the app with new database");

    println!("Dips got initialized.");
}
