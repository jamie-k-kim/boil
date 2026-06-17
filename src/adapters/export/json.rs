use crate::core::canon::ProjectGraph;
use crate::ports::export::ExportProvider;
use anyhow::Result;
use std::path::Path;

/// A JSON export provider.
///
/// Serializes the entire canon graph (nodes and edges) into a standard, pretty-printed
/// JSON file (`canon.json`) for downstream programmatic consumption.
pub struct JsonExporter;

impl ExportProvider for JsonExporter {
    fn format_name(&self) -> &str {
        "JSON"
    }
    fn extension(&self) -> &str {
        "json"
    }

    fn export(&self, graph: &ProjectGraph, output_dir: &Path) -> Result<()> {
        let path = output_dir.join("canon.json");
        let json = serde_json::to_string_pretty(&graph.graph)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
