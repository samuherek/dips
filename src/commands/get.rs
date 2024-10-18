use crate::configuration::Application;
use crate::models::dip;
use crate::models::dir_context;

fn render_items(items: Vec<dip::DipRowFull>) {
    if items.len() == 0 {
        println!("No items found.");
    } else {
        for item in items {
            println!("{}", item.value);
        }
    }
}

pub async fn exec(app: &Application, all: bool) {
    if all {
        let items = dip::get_all(&app.db_pool)
            .await
            .expect("Failed to read from database");
        render_items(items);
    } else {
        let scope = dir_context::get_closest(&app.db_pool, &app.context_dir)
            .await
            .expect("Failed to query dir context");
        let filter = dip::DipsFilter::new().with_scope_id(scope.as_ref().map(|x| x.id.clone()));
        let items = dip::get_filtered(&app.db_pool, filter)
            .await
            .expect("Failed to read from database");
        println!(
            "Scope: {}",
            scope
                .as_ref()
                .map(|x| x.dir_path.as_str())
                .unwrap_or("Global")
        );
        render_items(items);
    }
}
