use crate::git;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

#[derive(serde::Deserialize, sqlx::FromRow, Debug)]
pub struct ContextGroup {
    pub id: String,
    pub name: String,
    pub dir_context_id: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

impl ContextGroup {
    fn new(name: &str, dir_context_id: Option<&str>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().date_naive().into();
        Self {
            id,
            name: name.into(),
            dir_context_id: dir_context_id.map(String::from),
            created_at: now,
        }
    }
}

pub async fn get_or_create(
    tx: &mut Transaction<'_, Sqlite>,
    group: &str,
    dir_context_id: Option<&str>,
) -> Result<ContextGroup, sqlx::Error> {
    if let Some(group) = get(tx, group, dir_context_id).await? {
        Ok(group)
    } else {
        let group = create(tx, group, dir_context_id).await?;
        Ok(group)
    }
}

pub async fn get(
    tx: &mut Transaction<'_, Sqlite>,
    group: &str,
    dir_context_id: Option<&str>,
) -> Result<Option<ContextGroup>, sqlx::Error> {
    let res = sqlx::query_as!(
        ContextGroup,
        r#"
        select * from context_groups where name = $1 and dir_context_id = $2
        "#,
        group,
        dir_context_id,
    )
    .fetch_optional(&mut **tx)
    .await;
    res
}

pub async fn create(
    tx: &mut Transaction<'_, Sqlite>,
    name: &str,
    dir_context_id: Option<&str>,
) -> Result<ContextGroup, sqlx::Error> {
    let context_group = ContextGroup::new(name, dir_context_id);
    sqlx::query!(
        r#"
        insert into context_groups (id, name, dir_context_id, created_at)
        values ($1, $2, $3, $4)
        "#,
        context_group.id,
        context_group.name,
        context_group.dir_context_id,
        context_group.created_at
    )
    .execute(&mut **tx)
    .await?;
    Ok(context_group)
}
