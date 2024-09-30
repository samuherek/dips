use crate::configuration::Application;
use crate::models::context_group;
use crate::models::{dip, dir_context};
use sqlx::SqlitePool;

async fn value_exists(conn: &SqlitePool, value: &str) -> bool {
    sqlx::query!("SELECT * FROM dips WHERE value = ?1", value)
        .fetch_optional(conn)
        .await
        .expect("Failed to query database:")
        .is_some()
}

pub async fn add(app: &Application, value: &str, group: &Option<String>) {
    // TODO: we should check the existance of the value based on dir context
    // if value_exists(&db.conn, value).await {
    //     println!("This item already eixsts.");
    //     std::process::exit(0);
    // }
    let mut tx = app
        .db_pool
        .begin()
        .await
        .expect("Failed to start transaction in sqlite");
    let current_dir_context = dir_context::get_or_create_current(&mut tx)
        .await
        .expect("Failed to get the current dir context");
    let current_group = if let Some(group) = group {
        Some(
            context_group::get_or_create(&mut tx, group.as_ref(), Some(&current_dir_context.id))
                .await
                .expect("Failed to get or create a group"),
        )
    } else {
        None
    };

    dip::create(
        &mut tx,
        &current_dir_context.id,
        &value,
        None,
        current_group.map(|x| x.id).as_deref(),
    )
    .await
    .expect("Failed to create a dip");

    // Commit the transaction
    tx.commit().await.expect("Failed to commit transaction");

    println!("Dip added to {} context.", current_dir_context.dir_path);

    // TODO:
    // Get the project
    //
    // Based on:
    //  - git remote
    //  - dir path
    //  - git dir name
    //
    //  if not exist -> create one
    //  if exists -> get the ID
}
