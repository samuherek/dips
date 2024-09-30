use git2::Repository;
use std::path::{Path, PathBuf};

pub struct GitRepository {
    pub path: PathBuf,
    pub remote: Option<String>,
}

impl GitRepository {
    pub fn dir_name(&self) -> Option<String> {
        self.path.file_name().map(|x| x.to_string_lossy().to_string())
    }
}

pub fn git_repository(path: &Path) -> Option<GitRepository> {
    match Repository::discover(path) {
        Ok(repo) => {
            let dir_repo = repo.path();
            let remote = repo
                .find_remote("origin")
                .ok()
                .map(|x| x.url().map(String::from))
                .flatten();
            Ok(GitRepository {
                path: dir_repo.to_path_buf(),
                remote,
            })
        }
        Err(e) => Err(e),
    }
    .ok()
}
