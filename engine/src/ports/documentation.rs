use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Metadata and extracted sections from a documentation document (e.g. README, ADR).
#[derive(Serialize, Deserialize)]
pub struct DocumentInfo {
    /// The path to the document file.
    pub path: std::path::PathBuf,
    /// The document kind (e.g., "markdown", "text").
    pub kind: String,
    /// Extracted document sections mapped from header/topic name to its content.
    pub sections: std::collections::HashMap<String, String>,
}

/// A port defining operations for scanning and parsing project documentation.
pub trait DocumentationProvider: Send + Sync {
    /// Scans the project root for documentation files and returns their structured contents.
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<DocumentInfo>>;
}
