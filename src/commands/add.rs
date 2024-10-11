use crate::configuration::Application;
use crate::models::tag;
use crate::models::{dip, dir_context};

async fn value_exists(app: &Application, value: &str, group: Option<&str>, global: bool) -> bool {
    // - if dips.value is equal $1 -> additional check
    // additional check for equal value:
    //      - dips.dir_context_id
    //          - if null and is_global variable true -> might be match otherwise not
    //          - if string and dir_path, git_remote, git_dir_name matches dir_context ref -> might be
    //          match otherwise not
    //      - dips.context_group_id
    //          - if null and group is null -> might be a match
    //          - if string and group value equals to context_group name -> might be match
    let path = app.context_dir.path();
    let git_remote = app.context_dir.git_remote();
    let git_dir = app.context_dir.git_dir();
    sqlx::query!(
        r"
            select d.id from dips d
            left join dir_contexts c on d.dir_context_id = c.id
            where d.value = $1
              and (
                  (d.dir_context_id IS NOT NULL and (c.dir_path = $2 or c.git_remote = $3 or c.git_dir_name = $4))
                  or 
                  (d.dir_context_id IS NULL and $5)
              )
        ",
        value,
        path,
        git_remote,
        git_dir,
        global,
    )
    .fetch_optional(&app.db_pool)
    .await
    .expect("Failed to execute query")
    .is_some()
}

async fn add_global(app: &Application, value: &str, tag: Option<&str>) {
    let mut tx = app
        .db_pool
        .begin()
        .await
        .expect("Failed to start transaction in sqlite");

    dip::create(&mut tx, None, &value, None)
        .await
        .expect("Failed to create a dip");

    // Commit the transaction
    tx.commit().await.expect("Failed to commit transaction");
}

async fn add_contextual(app: &Application, value: &str, tag: Option<&str>) {
    let mut tx = app
        .db_pool
        .begin()
        .await
        .expect("Failed to start transaction in sqlite");
    let current_dir_context = dir_context::get_or_create_current(&mut tx)
        .await
        .expect("Failed to get the current dir context");

    let item = dip::create(&mut tx, Some(current_dir_context.id.as_ref()), &value, None)
        .await
        .expect("Failed to create a dip");

    if let Some(tag) = tag {
        tag::create_dip_tag(&mut tx, &item.id, tag)
            .await
            .expect("Failed to tag a dip");
    }

    // Commit the transaction
    tx.commit().await.expect("Failed to commit transaction");
}

pub async fn add(app: &Application, value: &str, group: Option<&str>, global: bool) {
    if value_exists(app, value, group, global).await {
        println!("{value} is already added in this context.");
    } else {
        if global {
            add_global(app, value, group).await;
        } else {
            add_contextual(app, value, group).await;
        }

        println!("Dip {value} added.");
    }
}
