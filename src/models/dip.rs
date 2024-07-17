use crate::models::dir_context::LocalContext;
use sqlx::SqlitePool;

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow, Debug)]
pub struct Dip {
    pub id: String,
    pub value: String,
    pub note: Option<String>,
    pub dir_context_id: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Dip {
    pub fn new(context_id: &str, value: &str, note: Option<&str>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().date_naive().into();
        let note = note.map(|v| v.to_string());
        Self {
            id,
            value: value.into(),
            note,
            dir_context_id: context_id.into(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(serde::Deserialize, sqlx::FromRow, Debug)]
pub struct DisplayDip {
    value: String,
    dir_path: String,
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

pub async fn db_context_all(conn: &SqlitePool, context: &LocalContext) -> Option<Vec<DisplayDip>> {
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
