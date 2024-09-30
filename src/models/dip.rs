use crate::models::dir_context::LocalContext;
use sqlx::{Sqlite, SqlitePool, Transaction};

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow, Debug)]
pub struct Dip {
    pub id: String,
    pub value: String,
    pub note: Option<String>,
    pub dir_context_id: String,
    pub context_group_id: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Dip {
    pub fn new(
        context_id: &str,
        value: &str,
        note: Option<&str>,
        context_group_id: Option<&str>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().date_naive().into();
        let note = note.map(|v| v.to_string());
        Self {
            id,
            value: value.into(),
            note,
            dir_context_id: context_id.into(),
            context_group_id: context_group_id.map(String::from),
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

pub async fn create(
    tx: &mut Transaction<'_, Sqlite>,
    dir_context_id: &str,
    value: &str,
    note: Option<&str>,
    context_group_id: Option<&str>,
) -> Result<Dip, sqlx::Error> {
    let item = Dip::new(dir_context_id, value, note, context_group_id);
    println!("new dip will be {:?}", item);
    let _ = sqlx::query!(
        r#"
        insert into dips(id, value, note, created_at, updated_at, context_group_id, dir_context_id)
        values($1, $2, $3, $4, $4, $5, $6)
        "#,
        item.id,
        item.value,
        item.note,
        item.created_at,
        item.context_group_id,
        item.dir_context_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(item)
}
