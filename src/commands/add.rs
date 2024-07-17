use crate::configuration::Settings;
use crate::database::Database;
use crate::models::{
    dip::Dip,
    dir_context::{self, DirContext},
};
use sqlx::SqlitePool;

async fn value_exists(conn: &SqlitePool, value: &str) -> bool {
    sqlx::query!("SELECT * FROM dips WHERE value = ?1", value)
        .fetch_optional(conn)
        .await
        .expect("Failed to query database:")
        .is_some()
}

pub async fn add(config: &Settings, value: &str) {
    let db = Database::connect(config).await;

    // TODO: we should check the existance of the value based on dir context
    if value_exists(&db.conn, value).await {
        println!("This item already eixsts.");
        std::process::exit(0);
    }

    let dir = std::env::current_dir().expect("Failed to load current directory.");
    let local_ctx = DirContext::find_local(&dir).expect("Failed to identify context.");
    // We first want to find as we want to create within a transaction
    let db_ctx = dir_context::db_find_one(&db.conn, &local_ctx).await;
    let mut dir_context_id = local_ctx.id.clone();
    let mut dir_context_path = local_ctx.dir_path.clone();
    let mut tx = db
        .conn
        .begin()
        .await
        .expect("Failed to start transaction in sqlite");

    if let Some(ctx) = db_ctx {
        dir_context_id = ctx.id;
        dir_context_path = ctx.dir_path;
    } else {
        sqlx::query!(r#"
            INSERT INTO dir_contexts (id, git_remote, git_dir_name, dir_path, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "# ,
            local_ctx.id, 
            local_ctx.git_remote, 
            local_ctx.git_dir_name, 
            local_ctx.dir_path, 
            local_ctx.created_at, 
            local_ctx.updated_at
        )
            .execute(&mut *tx)
            .await
            .expect("Failed to execute insert query");
    }

    let item = Dip::new(&local_ctx.id, value, None);

    match sqlx::query!(
        r#"
        INSERT INTO dips (id, value, note, dir_context_id, created_at, updated_at) 
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
        item.id,
        item.value,
        item.note,
        dir_context_id, 
        item.created_at,
        item.updated_at
    )
    .execute(&mut *tx)
    .await
    {
        Ok(_) => {
            println!("Dip added to {dir_context_path} context.");
        }
        Err(e) => {
            eprintln!("Failed to insert into databse: {e}");
        }
    }

    // Commit the transaction
    tx.commit().await.expect("Failed to commit transaction");

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
