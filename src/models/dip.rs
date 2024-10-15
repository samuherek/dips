use crate::models::tag;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::ops::Deref;

#[derive(Debug)]
pub struct DipsFilter {
    scope_id: Option<String>,
    search: Option<String>,
}

impl DipsFilter {
    pub fn new() -> Self {
        Self {
            scope_id: None,
            search: None,
        }
    }

    pub fn with_scope_id(self, id: Option<String>) -> Self {
        Self {
            scope_id: id,
            ..self
        }
    }

    pub fn with_search(self, value: &str) -> Self {
        Self {
            search: Some(value.to_owned()),
            ..self
        }
    }
}

#[derive(serde::Serialize, Debug)]
pub struct Dip {
    pub id: String,
    pub value: String,
    pub note: Option<String>,
    pub dir_context_id: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Dip {
    pub fn new(context_id: Option<&str>, value: &str, note: Option<&str>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().date_naive().into();
        let note = note.map(|v| v.to_string());
        Self {
            id,
            value: value.into(),
            note,
            dir_context_id: context_id.map(|x| x.to_string()),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug)]
pub struct DipTags(Vec<tag::TagMeta>);

#[derive(Debug, sqlx::FromRow)]
pub struct DipRowFull {
    pub id: String,
    pub value: String,
    pub note: Option<String>,
    pub dir_context_id: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub git_remote: Option<String>,
    pub git_dir_name: Option<String>,
    pub dir_path: String,
    #[sqlx(try_from = "String")]
    pub tags: DipTags,
}

impl TryFrom<String> for DipTags {
    type Error = std::convert::Infallible;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let tags = s
            .split(',')
            .filter_map(|tag| {
                let parts: Vec<&str> = tag.split(':').collect();
                if parts.len() == 2 {
                    Some(tag::TagMeta {
                        id: parts[0].parse().ok()?,
                        name: parts[1].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(DipTags(tags))
    }
}

impl Deref for DipTags {
    type Target = Vec<tag::TagMeta>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn get_filterd(
    conn: &SqlitePool,
    filter: DipsFilter,
) -> Result<Vec<DipRowFull>, sqlx::Error> {
    let search = format!("%{}%", filter.search.unwrap_or_default());
    sqlx::query_as(
        r"
       select dips.*, 
            dir_contexts.dir_path, 
            dir_contexts.git_remote, 
            dir_contexts.git_dir_name,
            GROUP_CONCAT(tags.id || ':' || tags.name) as tags
        from dips
        left join dir_contexts on dips.dir_context_id = dir_contexts.id
        LEFT JOIN dips_tags ON dips.id = dips_tags.dip_id
        LEFT JOIN tags ON dips_tags.tag_id = tags.id
        WHERE dips.dir_context_id = $1
        and LOWER(dips.value) LIKE LOWER($2)
        GROUP BY dips.id
        ",
    )
    .bind(filter.scope_id)
    .bind(search)
    .fetch_all(conn)
    .await
}

pub async fn get_all(conn: &SqlitePool) -> Result<Vec<DipRowFull>, sqlx::Error> {
    sqlx::query_as(
        r#"
       select dips.*, 
            dir_contexts.dir_path, 
            dir_contexts.git_remote, 
            dir_contexts.git_dir_name,
            GROUP_CONCAT(tags.id || ':' || tags.name) as tags
       from dips 
       left join dir_contexts on dips.dir_context_id = dir_contexts.id
       LEFT JOIN dips_tags ON dips.id = dips_tags.dip_id
       LEFT JOIN tags ON dips_tags.tag_id = tags.id
       GROUP BY dips.id
       "#,
    )
    .fetch_all(conn)
    .await
}

pub async fn create(
    tx: &mut Transaction<'_, Sqlite>,
    dir_context_id: Option<&str>,
    value: &str,
    note: Option<&str>,
) -> Result<Dip, sqlx::Error> {
    let item = Dip::new(dir_context_id, value, note);
    let _ = sqlx::query!(
        r#"
        insert into dips(id, value, note, created_at, updated_at, dir_context_id)
        values($1, $2, $3, $4, $4, $5)
        "#,
        item.id,
        item.value,
        item.note,
        item.created_at,
        item.dir_context_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(item)
}
