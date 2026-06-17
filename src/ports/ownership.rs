use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents an ownership rule mapping a path pattern to its owners.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnershipRule {
    /// Glob pattern matching file paths (e.g., `src/**`).
    pub pattern: String,
    /// Identifiers of the owners (e.g., teams, emails, handles).
    pub owners: Vec<String>,
}

/// A port defining ownership tracking mechanisms.
///
/// Implementations (e.g., CODEOWNERS files, Jira team lookups, PagerDuty plugin)
/// associate paths and sub-architectures with their responsible entities/teams.
pub trait OwnershipProvider: Send + Sync {
    /// Queries and returns the list of ownership rules for the project workspace.
    fn get_ownership_info(&self, project_root: &Path) -> Result<Vec<OwnershipRule>>;
}
