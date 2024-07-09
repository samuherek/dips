use crate::configuration::Settings;
use crate::database::Database;
use crate::models::dip::Dip;
use sqlx::SqliteConnection;

async fn value_exists(conn: &mut SqliteConnection, value: &str) -> bool {
    sqlx::query!("SELECT * FROM dips WHERE value = ?1", value)
        .fetch_optional(conn)
        .await
        .expect("Failed to query database:")
        .is_some()
}

pub async fn add(config: &Settings, value: &str) {
    let mut db = Database::connect(config).await;

    // TODO: we should check the existance of the value based on dir context
    if value_exists(&mut db.conn, value).await {
        println!("This item already eixsts.");
        std::process::exit(0);
    }

    let item = Dip::new(value, None);
    match sqlx::query!(
        "INSERT INTO dips (id, value, note, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        item.id,
        item.value,
        item.note,
        item.created_at,
        item.updated_at
    )
    .execute(&mut db.conn)
    .await
    {
        Ok(_) => {
            println!("Item saved: '{}'", value);
        }
        Err(e) => {
            eprintln!("Failed to insert into databse: {e}");
        }
    }

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
