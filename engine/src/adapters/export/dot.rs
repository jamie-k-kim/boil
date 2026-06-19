use boil_core::canon::ProjectGraph;
use crate::ports::export::ExportProvider;
use anyhow::Result;
use petgraph::dot::{Config, Dot};
use std::path::Path;

pub struct DotExporter;

impl ExportProvider for DotExporter {
    fn format_name(&self) -> &str {
        "DOT"
    }
    fn extension(&self) -> &str {
        "dot"
    }

    fn export(&self, graph: &ProjectGraph, output_dir: &Path) -> Result<()> {
        let path = output_dir.join("canon.dot");
        let dot = format!(
            "{:?}",
            Dot::with_config(&graph.graph, &[Config::EdgeNoLabel])
        );
        std::fs::write(path, dot)?;
        Ok(())
    }
}
