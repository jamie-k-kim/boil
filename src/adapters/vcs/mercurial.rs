use crate::ports::vcs::{Author, FileProvenance, VcsProvider};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

pub struct MercurialProvider {
    blame_cache: Mutex<HashMap<PathBuf, Vec<Option<Author>>>>,
}

impl Default for MercurialProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MercurialProvider {
    pub fn new() -> Self {
        Self {
            blame_cache: Mutex::new(HashMap::new()),
        }
    }
}

impl VcsProvider for MercurialProvider {
    fn get_file_provenance(&self, project_root: &Path, file_path: &Path) -> Result<FileProvenance> {
        let output = Command::new("hg")
            .current_dir(project_root)
            .args([
                "log",
                "-f",
                "--template",
                "{author|person}|{author|email}|{date|isodate}\n",
                file_path.to_str().unwrap(),
            ])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if lines.is_empty() {
            return Err(anyhow::anyhow!("No history found for file in Mercurial"));
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

            // hg annotate -u <file> gives output like:
            // "   Jamie: code line here"
            if let Ok(output) = Command::new("hg")
                .current_dir(project_root)
                .args(["annotate", "-u", file_path.to_str().unwrap()])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for annotate_line in stdout.lines() {
                    // Split at the first colon
                    if let Some((user_part, _)) = annotate_line.split_once(':') {
                        let name = user_part.trim().to_string();
                        let author = Author {
                            name: if name.is_empty() {
                                "Unknown".to_string()
                            } else {
                                name
                            },
                            email: "unknown@example.com".to_string(), // hg annotate doesn't easily provide email
                        };
                        blame_authors.push(Some(author));
                    } else {
                        blame_authors.push(None);
                    }
                }
            }
            cache.insert(file_path.to_path_buf(), blame_authors);
        }

        if let Some(file_blame) = cache.get(file_path)
            && line < file_blame.len()
            && let Some(author) = &file_blame[line]
        {
            return Ok(author.clone());
        }

        Err(anyhow::anyhow!(
            "Could not find blame info for line {} via Mercurial",
            line
        ))
    }
}
