use git2::Repository;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct GitRepository {
    pub path: PathBuf,
    pub dir_name: String,
    pub remote: Option<String>,
}

pub fn git_repository(path: &Path) -> Option<GitRepository> {
    match Repository::discover(path) {
        Ok(repo) => {
            let path = repo.path().parent().unwrap().to_path_buf();
            let dir_name = path
                .file_name()
                .map(|x| x.to_string_lossy().to_string())
                .unwrap();
            let remote = repo
                .find_remote("origin")
                .ok()
                .and_then(|x| x.url().map(String::from));
            Ok(GitRepository {
                path,
                dir_name,
                remote,
            })
        }
        Err(e) => Err(e),
    }
    .ok()
}

#[cfg(test)]
mod test {
    use super::*;
    use fake::faker::internet::en::{DomainSuffix, FreeEmail};
    use fake::faker::lorem::en::Word;
    use fake::Fake;
    use rand::thread_rng;

    fn remote() -> String {
        let mut rng = thread_rng();
        let domain: String = DomainSuffix().fake_with_rng(&mut rng);
        let email_user: String = FreeEmail().fake_with_rng(&mut rng);

        format!(
            "https://{}.{}/repo.git",
            email_user.split('@').next().unwrap(),
            domain
        )
    }

    fn temp_repo(repo_name: &str, repo_origin: &str) -> (tempfile::TempDir, PathBuf) {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create a temp directory.");
        let repo_path = temp_dir.path().join(repo_name);
        let repo = git2::Repository::init(&repo_path).expect("Failed to init temp repository.");
        repo.remote("origin", repo_origin)
            .expect("Failed to add origin to temp git repo.");
        (temp_dir, repo_path.to_path_buf())
    }

    #[test]
    fn find_git_repository() {
        let repo_name = Word().fake();
        let repo_remote = remote();
        let (_dir, path) = temp_repo(repo_name, &repo_remote);

        let repo = git_repository(&path);
        assert!(repo.is_some());

        let repo = repo.unwrap();
        assert_eq!(repo.dir_name, repo_name);
        assert_eq!(repo.remote, Some(repo_remote));
        assert_eq!(
            repo.path,
            // Had to add in the "private" as the tempfile adds it. It's not part of the test.
            std::path::Path::new("/private").join(path.strip_prefix("/").unwrap())
        );
    }
}
