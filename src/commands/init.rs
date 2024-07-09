use crate::configuration::Settings;
use crate::database::Database;

pub async fn init(config: &Settings) {
    Database::init(config).await;
    println!("Database initialzied.");
}
