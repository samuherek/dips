use crate::configuration::Settings;
use crate::database::Database;

pub async fn init(config: &Settings) {
    let mut db = Database::connect(config).await;
    db.init().await;
    println!("Database initialzied.");
}
