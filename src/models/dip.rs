use crate::models::dir_context::{ContextScope, RuntimeDirContext};
use sqlx::{Sqlite, SqlitePool, Transaction};

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow, Debug)]
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

#[derive(serde::Deserialize, sqlx::FromRow, Debug)]
pub struct DisplayDip {
    pub value: String,
    dir_path: String,
}

#[derive(serde::Deserialize, sqlx::FromRow, Debug)]
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
}

impl DisplayDip {
    pub fn format(&self) -> String {
        let dir = std::path::PathBuf::from(&self.dir_path);
        let parent = dir
            .file_name()
            .map_or("Global:".into(), |v| v.to_string_lossy());
        format!("{}: \"{}\"", parent, self.value)
    }
}

pub async fn get_dir_context_all(
    conn: &SqlitePool,
    scope: &ContextScope,
) -> Result<Vec<DipRowFull>, sqlx::Error> {
    sqlx::query_as(
        r"
       select dips.*, 
            dir_contexts.dir_path, 
            dir_contexts.git_remote, 
            dir_contexts.git_dir_name 
        from dips
        left join dir_contexts on dips.dir_context_id = dir_contexts.id
        WHERE dips.dir_context_id = $1
        ",
    )
    .bind(scope.id())
    .fetch_all(conn)
    .await
}

pub async fn get_all(conn: &SqlitePool) -> Result<Vec<DipRowFull>, sqlx::Error> {
    sqlx::query_as(
        r#"
       select dips.*, 
            dir_contexts.dir_path, 
            dir_contexts.git_remote, 
            dir_contexts.git_dir_name 
       from dips 
       left join dir_contexts on dips.dir_context_id = dir_contexts.id
       "#,
    )
    .fetch_all(conn)
    .await
}

pub async fn db_all(conn: &SqlitePool) -> Option<Vec<DisplayDip>> {
    match sqlx::query_as(
        r#"
        SELECT dips.value, dir_contexts.dir_path 
        FROM dips 
        JOIN dir_contexts ON dips.dir_context_id = dir_contexts.id
        "#,
    )
    .fetch_all(conn)
    .await
    {
        Ok(res) => Some(res),
        Err(e) => {
            eprintln!("ERROR: failed to query dips: {e}");
            None
        }
    }
}

pub async fn db_context_all(
    conn: &SqlitePool,
    context: &RuntimeDirContext,
) -> Option<Vec<DisplayDip>> {
    let dir_path = format!("{}%", context.path());
    match sqlx::query_as(
        r#"
        SELECT dips.value, dir_contexts.dir_path 
        FROM dips 
        JOIN dir_contexts ON dips.dir_context_id = dir_contexts.id
        WHERE dir_contexts.dir_path LIKE ?
        "#,
    )
    .bind(dir_path)
    .fetch_all(conn)
    .await
    {
        Ok(res) => Some(res),
        Err(e) => {
            eprintln!("ERROR: failed to query dips: {e}");
            None
        }
    }
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

pub async fn find_one(
    pool: &SqlitePool,
    value: &str,
    dir_context_id: Option<&str>,
) -> Result<Option<Dip>, sqlx::Error> {
    let item = sqlx::query_as(
        r"
        select * from dips where value = $1 and dir_context_id = $2 
        ",
    )
    .bind(value)
    .bind(dir_context_id)
    .fetch_optional(pool)
    .await?;
    Ok(item)
}
