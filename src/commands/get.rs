use crate::configuration::Application;
use crate::models::dip;
use crate::models::dir_context;

pub async fn get(app: &Application, all: bool) {
    let items = if all {
        dip::db_all(&app.db_pool).await
    } else {
        let current_dir = std::env::current_dir().expect("Failed to read current directory");
        let current_context = dir_context::RuntimeDirContext::try_from(current_dir)
            .expect("Failed to establish local context.");
        dip::db_context_all(&app.db_pool, &current_context).await
    };

    if let Some(items) = items {
        for item in items {
            println!("{}", item.format());
        }
    } else {
        println!("No items found");
    }
}
