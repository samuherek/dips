use crate::configuration::Settings;
use crate::database::Database;

pub async fn init() {
    let settings = Settings::init();
    Database::init(&settings).await;
    println!("Dips got initialized.");
}
