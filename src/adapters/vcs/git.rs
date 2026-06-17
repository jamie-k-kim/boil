use crate::ports::vcs::{Author, FileProvenance, VcsProvider};
use anyhow::Result;
use git2::{BlameOptions, Repository};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A version control provider using Git (`libgit2`).
///
/// Executes highly parallelized `git blame` and `git log` operations to link
/// symbols and files back to their original authors and history.
pub struct GitProvider {
    blame_cache: Mutex<HashMap<PathBuf, Vec<Option<Author>>>>,
}

impl Default for GitProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GitProvider {
    pub fn new() -> Self {
        Self {
            blame_cache: Mutex::new(HashMap::new()),
        }
    }
}

impl VcsProvider for GitProvider {
    fn get_file_provenance(&self, project_root: &Path, file_path: &Path) -> Result<FileProvenance> {
        let output = std::process::Command::new("git")
            .current_dir(project_root)
            .args([
                "log",
                "--follow",
                "--format=%an|%ae|%aI",
                "--",
                file_path.to_str().unwrap(),
            ])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if lines.is_empty() {
            return Err(anyhow::anyhow!("No history found for file"));
        }

        let mut authors = HashSet::new();
        let mut commit_count = 0;
        let last_modified_date = lines
            .first()
            .unwrap()
            .split('|')
            .nth(2)
            .unwrap_or("")
            .to_string();
        let creation_date = lines
            .last()
            .unwrap()
            .split('|')
            .nth(2)
            .unwrap_or("")
            .to_string();

        for line in lines {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 2 {
                authors.insert(Author {
                    name: parts[0].to_string(),
                    email: parts[1].to_string(),
                });
                commit_count += 1;
            }
        }

        Ok(FileProvenance {
            primary_authors: authors.into_iter().collect(),
            commit_count,
            creation_date,
            last_modified_date,
        })
    }

    fn get_author_at_line(
        &self,
        project_root: &Path,
        file_path: &Path,
        line: usize,
    ) -> Result<Author> {
        let mut cache = self.blame_cache.lock().unwrap();

        if !cache.contains_key(file_path) {
            let mut blame_authors = Vec::new();
            if let Ok(repo) = Repository::open(project_root) {
                let mut options = BlameOptions::new();
                if let Ok(blame) = repo.blame_file(file_path, Some(&mut options)) {
                    for hunk in blame.iter() {
                        let commit_id = hunk.final_commit_id();
                        if let Ok(commit) = repo.find_commit(commit_id) {
                            let signature = commit.author();
                            let author = Author {
                                name: signature.name().unwrap_or("Unknown").to_string(),
                                email: signature.email().unwrap_or("Unknown").to_string(),
                            };
                            let start = hunk.final_start_line();
                            let count = hunk.lines_in_hunk();
                            for i in 0..count {
                                let l = start + i;
                                if l >= blame_authors.len() {
                                    blame_authors.resize(l + 1, None);
                                }
                                blame_authors[l] = Some(author.clone());
                            }
                        }
                    }
                }
            }
            cache.insert(file_path.to_path_buf(), blame_authors);
        }

        if let Some(file_blame) = cache.get(file_path) {
            let blame_line = line + 1; // 1-indexed
            if blame_line < file_blame.len()
                && let Some(author) = &file_blame[blame_line]
            {
                return Ok(author.clone());
            }
        }

        Err(anyhow::anyhow!(
            "Could not find blame info for line {}",
            line
        ))
    }
}
