use boil_core::canon::ProjectGraph;
use boil_core::canon::graph::{EdgeData, NodeData};
use crate::ports::export::ExportProvider;
use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct Neo4jProvider;

impl Default for Neo4jProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Neo4jProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ExportProvider for Neo4jProvider {
    fn format_name(&self) -> &str {
        "neo4j"
    }

    fn extension(&self) -> &str {
        "csv"
    }

    fn export(&self, graph: &ProjectGraph, output_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;

        let nodes_path = output_dir.join("nodes.csv");
        let mut nodes_file = File::create(nodes_path)?;
        writeln!(nodes_file, "id:ID,name,type,:LABEL")?;

        for node_idx in graph.graph.node_indices() {
            let id = graph.get_node_id(node_idx);
            let node = &graph.graph[node_idx];

            let (name, node_type, label) = match node {
                NodeData::Repository { .. } => ("repo".to_string(), "repository", "Repository"),
                NodeData::Subsystem { name, .. } => (name.clone(), "subsystem", "Subsystem"),
                NodeData::Module { name, .. } => (name.clone(), "module", "Module"),
                NodeData::File { path, .. } => (path.to_string_lossy().to_string(), "file", "File"),
                NodeData::Symbol { name, .. } => (name.clone(), "symbol", "Symbol"),
                NodeData::Package { name, .. } => (name.clone(), "package", "Package"),
                NodeData::BuildTarget { name, .. } => (name.clone(), "target", "BuildTarget"),
                NodeData::Author { name, .. } => (name.clone(), "author", "Author"),
                NodeData::Owner { name, .. } => (name.clone(), "owner", "Owner"),
                NodeData::Document { path, .. } => {
                    (path.to_string_lossy().to_string(), "document", "Document")
                }
            };

            // Basic escaping for CSV
            let safe_name = name.replace("\"", "\"\"");
            writeln!(
                nodes_file,
                "\"{}\",\"{}\",{},{}",
                id, safe_name, node_type, label
            )?;
        }

        let edges_path = output_dir.join("edges.csv");
        let mut edges_file = File::create(edges_path)?;
        writeln!(edges_file, ":START_ID,:END_ID,:TYPE")?;

        for edge_idx in graph.graph.edge_indices() {
            if let Some((source_idx, target_idx)) = graph.graph.edge_endpoints(edge_idx) {
                let source_id = graph.get_node_id(source_idx);
                let target_id = graph.get_node_id(target_idx);
                let edge_data = &graph.graph[edge_idx];

                let edge_type = match edge_data {
                    EdgeData::Contains => "CONTAINS",
                    EdgeData::Calls => "CALLS",
                    EdgeData::References => "REFERENCES",
                    EdgeData::Imports => "IMPORTS",
                    EdgeData::DependsOnExternal => "DEPENDS_ON",
                    EdgeData::AuthoredBy => "AUTHORED_BY",
                    EdgeData::OwnedBy => "OWNED_BY",
                    EdgeData::ExecutedAtRuntime => "EXECUTED_AT",
                    EdgeData::SemanticSimilarity(_) => "SIMILAR_TO",
                    EdgeData::Describes => "DESCRIBES",
                };

                writeln!(
                    edges_file,
                    "\"{}\",\"{}\",{}",
                    source_id, target_id, edge_type
                )?;
            }
        }

        Ok(())
    }
}
