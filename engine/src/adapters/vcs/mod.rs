//! Version control adapters.
//!
//! These adapters query Version Control Systems (VCS) to enrich the graph with
//! provenance metadata, such as file creation dates, commit counts, and line-level blame (authorship).

pub mod git;
pub mod mercurial;
pub use git::GitProvider;
