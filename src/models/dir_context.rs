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
        let now = chrono::Utc::now().date_naive().into();

        // TODO: find the git repo data

        Ok(Self {
            id,
            git_remote: None,
            git_dir_name: None,
            dir_path,
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Debug)]
pub struct LocalContext {
    git_remote: Option<String>,
    git_dir_name: Option<String>,
    path: String,
}

impl LocalContext {
    pub fn path(&self) -> String {
        self.path.to_string()
    }
}

impl TryFrom<PathBuf> for LocalContext {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if !path.exists() {
            return Err(Error::new(ErrorKind::NotFound, "Incorrect context path"));
        }

        let dir_path = path.display().to_string();
        Ok(Self {
            git_remote: None,
            git_dir_name: None,
            path: dir_path,
        })
    }
}

pub async fn db_find_one(conn: &SqlitePool, ctx: &DirContext) -> Option<DirContext> {
    match sqlx::query_as!(
        DirContext,
        "SELECT * FROM dir_contexts WHERE dir_path = ?",
        ctx.dir_path
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
