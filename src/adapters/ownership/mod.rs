//! Code ownership providers.
//!
//! These adapters map directories, files, or symbols to teams and individual developers
//! by parsing ownership configurations (e.g., `CODEOWNERS`) or querying external APIs.

pub mod codeowners;
pub mod github_teams;
pub mod jira;

pub use codeowners::CodeownersProvider;
pub use github_teams::GithubTeamsProvider;
pub use jira::JiraProvider;
