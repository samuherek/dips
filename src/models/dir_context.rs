use crate::git;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

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
    fn new(dir_path: &str, git_dir_name: Option<String>, git_remote: Option<String>) -> Self {
        let now: chrono::NaiveDateTime = chrono::Utc::now().date_naive().into();
        let id: String = uuid::Uuid::new_v4().into();
        Self {
            id,
            dir_path: dir_path.into(),
            git_remote,
            git_dir_name,
            created_at: now,
            updated_at: now,
        }
    }
}

pub async fn get_or_create_current(
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<DirContext, anyhow::Error> {
    let current_path = std::env::current_dir()?;
    let curr_path_string = current_path.display().to_string();
    let dir_context = match git::git_repository(&current_path) {
        Some(repo) => {
            db_find_or_create(tx, &curr_path_string, Some(repo.dir_name), repo.remote).await
        }
        None => db_find_or_create(tx, &curr_path_string, None, None).await,
    }?;
    Ok(dir_context)
}

#[derive(Debug)]
pub struct RuntimeDirContext {
    git_remote: Option<String>,
    git_dir_name: Option<String>,
    git_dir_path: Option<PathBuf>,
    path: PathBuf,
}

impl RuntimeDirContext {
    pub fn path(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    pub fn git_dir(&self) -> Option<&str> {
        self.git_dir_name.as_deref()
    }

    pub fn git_remote(&self) -> Option<&str> {
        self.git_remote.as_deref()
    }
}

impl TryFrom<&Path> for RuntimeDirContext {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let (git_remote, git_dir_name, git_dir_path) =
            if let Some(repo) = git::git_repository(&path) {
                (repo.remote, Some(repo.dir_name), Some(repo.path))
            } else {
                (None, None, None)
            };

        if !path.exists() {
            return Err(Error::new(ErrorKind::NotFound, "Incorrect context path"));
        }

        Ok(Self {
            git_remote,
            git_dir_name,
            git_dir_path,
            path: PathBuf::from(path),
        })
    }
}

impl TryFrom<PathBuf> for RuntimeDirContext {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let (git_remote, git_dir_name, git_dir_path) =
            if let Some(repo) = git::git_repository(&path) {
                (repo.remote, Some(repo.dir_name), Some(repo.path))
            } else {
                (None, None, None)
            };

        if !path.exists() {
            return Err(Error::new(ErrorKind::NotFound, "Incorrect context path"));
        }

        Ok(Self {
            git_remote,
            git_dir_name,
            git_dir_path,
            path,
        })
    }
}

pub async fn db_find_one(
    conn: &SqlitePool,
    current_path: &str,
    git_dir_name: Option<&str>,
    git_remote: Option<&str>,
) -> Option<DirContext> {
    match sqlx::query_as!(
        DirContext,
        "SELECT * FROM dir_contexts WHERE dir_path = $1 OR git_remote = $2 OR git_dir_name = $3",
        current_path,
        git_dir_name,
        git_remote
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

pub async fn db_create(
    tx: &mut Transaction<'_, Sqlite>,
    dir_path: &str,
    git_dir_name: Option<String>,
    git_remote: Option<String>,
) -> Result<DirContext, sqlx::Error> {
    let dir_context = DirContext::new(dir_path, git_dir_name, git_remote);
    println!("dir context to be {:?}", dir_context);
    let res = sqlx::query!(
        r#"
        insert into dir_contexts(
            id, dir_path, git_remote, git_dir_name, created_at, updated_at
        ) values (
            $1, $2, $3, $4, $5, $6
        )"#,
        dir_context.id,
        dir_context.dir_path,
        dir_context.git_remote,
        dir_context.git_dir_name,
        dir_context.created_at,
        dir_context.updated_at,
    )
    .execute(&mut **tx)
    .await?;
    println!("this is the res from the dir context insert {:?}", res);
    // TODO: if res.rows_affected() == 1
    Ok(dir_context)
}

pub async fn db_find_or_create(
    tx: &mut Transaction<'_, Sqlite>,
    current_path: &str,
    git_dir_name: Option<String>,
    git_remote: Option<String>,
) -> Result<DirContext, sqlx::Error> {
    if let Some(res) = sqlx::query_as!(
        DirContext,
        "SELECT * FROM dir_contexts WHERE dir_path = $1 OR git_remote = $2 OR git_dir_name = $3",
        current_path,
        git_dir_name,
        git_remote
    )
    .fetch_optional(&mut **tx)
    .await?
    {
        Ok(res)
    } else {
        db_create(tx, current_path, git_dir_name, git_remote).await
    }
}
