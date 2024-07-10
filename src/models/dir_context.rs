use sqlx::SqlitePool;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

#[derive(serde::Deserialize, sqlx::FromRow, Debug)]
pub struct DirContext {
    pub id: String,
    pub git_remote: Option<String>,
    pub git_dir_name: Option<String>,
    pub dir_path: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl DirContext {
    pub fn find_local(path: &PathBuf) -> Result<Self, Error> {
        if !path.exists() {
            return Err(Error::new(ErrorKind::NotFound, "Incorrect context path"));
        }

        let id = uuid::Uuid::new_v4().into();
        let dir_path = path.display().to_string();
        let now = chrono::Utc::now();

        // TODO: find the git repo data

        Ok(Self {
            id,
            git_remote: None,
            git_dir_name: None,
            dir_path,
            created_at: now.date_naive().into(),
            updated_at: now.date_naive().into(),
        })
    }
}

pub async fn db_find_one(conn: &SqlitePool, ctx: &DirContext) -> Option<DirContext> {
    match sqlx::query_as!(
        DirContext,
        "SELECT * FROM dir_contexts WHERE id = ?",
        ctx.id
    )
    .fetch_optional(conn)
    .await
    {
        Ok(res) => res,
        Err(e) => {
            eprintln!("ERROR: failed to query dir_contexts: {e}");
            None
        }
    }
}
