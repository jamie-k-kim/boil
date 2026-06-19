use crate::ports::runtime::{RuntimeProvider, RuntimeTrace};
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct TraceEntry {
    pub file: String,
    pub symbol: String,
    pub count: usize,
}

/// A JSON trace provider.
///
/// Ingests a local `trace.json` file generated from external execution environments
/// to map runtime metrics (e.g., call frequencies) onto the static symbols.
pub struct JsonTraceProvider;

impl RuntimeProvider for JsonTraceProvider {
    fn get_traces(&self, project_root: &Path) -> Result<Vec<RuntimeTrace>> {
        let trace_path = project_root.join("trace.json");
        if trace_path.exists() {
            let content = std::fs::read_to_string(&trace_path)?;
            let traces: Vec<TraceEntry> = serde_json::from_str(&content)?;
            return Ok(traces
                .into_iter()
                .map(|t| RuntimeTrace {
                    file: t.file,
                    symbol: t.symbol,
                    count: t.count,
                })
                .collect());
        }
        Ok(Vec::new())
    }
}
