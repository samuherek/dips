use crate::configuration::Settings;
use crate::database::Database;

pub async fn get(config: &Settings, all: bool) {
    let db = Database::connect(config).await;
    println!("all??? {:?}", all);
}
