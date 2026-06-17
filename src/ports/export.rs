use crate::core::canon::ProjectGraph;
use anyhow::Result;
use std::path::Path;

/// A port defining graph export serialization formats.
///
/// Implementations format the ProjectGraph into external formats (e.g., GraphML, CSV, JSON).
pub trait ExportProvider: Send + Sync {
    /// Returns the human-readable name of the format (e.g., "Neo4j CSV").
    fn format_name(&self) -> &str;
    /// Returns the file extension associated with the format (e.g., "csv").
    fn extension(&self) -> &str;
    /// Serializes and exports the graph to the target directory.
    fn export(&self, graph: &ProjectGraph, output_dir: &Path) -> Result<()>;
}
