use crate::configuration::Settings;
use crate::database::Database;
use crate::models::dip;
use crate::models::dir_context;

pub async fn get(config: &Settings, all: bool) {
    let db = Database::connect(config).await;
    let items = if all {
        dip::db_all(&db.conn).await
    } else {
        let current_dir = std::env::current_dir().expect("Failed to read current directory");
        let current_context = dir_context::LocalContext::try_from(current_dir)
            .expect("Failed to establish local context.");
        dip::db_context_all(&db.conn, &current_context).await
    };

    if let Some(items) = items {
        for item in items {
            println!("{}", item.format());
        }
    } else {
        println!("No items found");
    }
}
