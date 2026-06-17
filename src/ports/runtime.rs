use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents a hotspot trace node from a runtime trace source (e.g., eBPF, OpenTelemetry).
#[derive(Serialize, Deserialize)]
pub struct RuntimeTrace {
    /// The file path executing this trace.
    pub file: String,
    /// The specific symbol/function executing in this trace.
    pub symbol: String,
    /// The execution frequency count (hotspot weight).
    pub count: usize,
}

/// A port defining operations to load runtime execution profiles.
///
/// Implementations load execution trace statistics to enrich the graph nodes with real performance weights.
pub trait RuntimeProvider: Send + Sync {
    /// Loads execution hotness traces from the project workspace.
    fn get_traces(&self, project_root: &Path) -> Result<Vec<RuntimeTrace>>;
}
