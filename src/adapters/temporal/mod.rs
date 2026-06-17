//! Temporal evolution adapters.
//!
//! These adapters checkout codebases across different points in time (revisions, commits)
//! and compare multiple graph snapshots to calculate architectural drift.

pub mod git;

pub use git::GitTemporalProvider;
