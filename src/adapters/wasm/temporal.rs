use crate::core::canon::ProjectGraph;
use crate::ports::temporal::{DiffReport, TemporalProvider};
use anyhow::{Context, Result};
use extism::{Manifest, Plugin, Wasm};
use serde_json::json;
use std::path::Path;
use std::sync::Mutex;

pub struct WasmTemporalProvider {
    plugin_path: String,
    plugin: Mutex<Plugin>,
}

impl WasmTemporalProvider {
    pub fn new(plugin_path: &str) -> Result<Self> {
        let wasm = Wasm::file(plugin_path);
        let manifest = Manifest::new([wasm]);
        let plugin = Plugin::new(&manifest, [], true)
            .context("Failed to initialize Extism plugin for WasmTemporalProvider")?;

        Ok(Self {
            plugin_path: plugin_path.to_string(),
            plugin: Mutex::new(plugin),
        })
    }
}

impl TemporalProvider for WasmTemporalProvider {
    fn build_graph_from_commit(
        &self,
        _engine: &crate::core::Engine,
        _config: &crate::core::EngineConfig,
        repo_path: &Path,
        commit_rev: &str,
    ) -> Result<ProjectGraph> {
        let root_str = repo_path.to_str().unwrap_or(".");
        let mut plugin = self.plugin.lock().unwrap();

        let input = json!({
            "repo_path": root_str,
            "commit_rev": commit_rev
        })
        .to_string();

        let json_result: String =
            plugin
                .call("build_graph_from_commit", input)
                .context(format!(
                    "Failed to execute 'build_graph_from_commit' on Wasm plugin {}",
                    self.plugin_path
                ))?;

        let graph: ProjectGraph = serde_json::from_str(&json_result)
            .context("Failed to deserialize ProjectGraph from Wasm plugin output")?;

        Ok(graph)
    }

    fn compare_graphs(&self, base: &ProjectGraph, head: &ProjectGraph) -> Result<DiffReport> {
        let mut plugin = self.plugin.lock().unwrap();

        let input = json!({
            "base": base,
            "head": head
        })
        .to_string();

        let json_result: String = plugin.call("compare_graphs", input).context(format!(
            "Failed to execute 'compare_graphs' on Wasm plugin {}",
            self.plugin_path
        ))?;

        let report: DiffReport = serde_json::from_str(&json_result)
            .context("Failed to deserialize DiffReport from Wasm plugin output")?;

        Ok(report)
    }
}
