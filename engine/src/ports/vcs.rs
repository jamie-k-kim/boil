use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents an author from a version control system.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Author {
    /// The author's name.
    pub name: String,
    /// The author's email address.
    pub email: String,
}

/// Metadata summarizing the history/provenance of a specific file.
#[derive(Clone, Serialize, Deserialize)]
pub struct FileProvenance {
    /// The primary authors (ranked by commit contribution).
    pub primary_authors: Vec<Author>,
    /// Total number of commits affecting the file.
    pub commit_count: usize,
    /// The date the file was originally created.
    pub creation_date: String,
    /// The date the file was last modified.
    pub last_modified_date: String,
}

/// A port defining operations for querying the Version Control System (VCS).
///
/// Implementations (e.g., Git) provide authorship, commit history, and blame information.
pub trait VcsProvider: Send + Sync {
    /// Retrieves the overall history and provenance metadata for a file.
    fn get_file_provenance(&self, project_root: &Path, file_path: &Path) -> Result<FileProvenance>;

    /// Blames the specified line number (1-indexed) to identify its author.
    fn get_author_at_line(
        &self,
        project_root: &Path,
        file_path: &Path,
        line: usize,
    ) -> Result<Author>;
}
