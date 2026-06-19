use boil_core::canon::ProjectGraph;
use crate::core::engine::{Engine, EngineConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents structural differences between two canonical graphs (e.g., across commits).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReport {
    /// Subsystems added.
    pub added_subsystems: Vec<String>,
    /// Subsystems removed.
    pub removed_subsystems: Vec<String>,
    /// Symbols added.
    pub added_symbols: Vec<String>,
    /// Symbols removed.
    pub removed_symbols: Vec<String>,
    /// Moved symbols mapped from name to a path change description.
    pub moved_symbols: Vec<(String, String)>,
    /// Number of new edges added in the head graph.
    pub new_edges: usize,
}

/// A port defining operations for temporal (historical) graph building and comparison.
pub trait TemporalProvider: Send + Sync {
    /// Checks out the repository at a specific commit and builds its canonical project graph.
    fn build_graph_from_commit(
        &self,
        engine: &Engine,
        config: &EngineConfig,
        repo_path: &Path,
        commit_rev: &str,
    ) -> Result<ProjectGraph>;

    /// Compares two graphs (base vs head) and produces a structural diff report.
    fn compare_graphs(&self, base: &ProjectGraph, head: &ProjectGraph) -> Result<DiffReport>;
}
