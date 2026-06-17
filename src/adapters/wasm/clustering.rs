use crate::ports::clustering::{ClusteringProvider, ClusteringResult, NodeId};
use anyhow::{Context, Result};
use extism::{Manifest, Plugin, Wasm};
use serde_json::json;
use std::sync::Mutex;

pub struct WasmClusteringProvider {
    plugin_path: String,
    plugin: Mutex<Plugin>,
}

impl WasmClusteringProvider {
    pub fn new(plugin_path: &str) -> Result<Self> {
        let wasm = Wasm::file(plugin_path);
        let manifest = Manifest::new([wasm]);
        let plugin = Plugin::new(&manifest, [], true)
            .context("Failed to initialize Extism plugin for WasmClusteringProvider")?;

        Ok(Self {
            plugin_path: plugin_path.to_string(),
            plugin: Mutex::new(plugin),
        })
    }
}

impl ClusteringProvider for WasmClusteringProvider {
    fn cluster(
        &self,
        node_count: usize,
        edges: Vec<(NodeId, NodeId, f64)>,
    ) -> Result<ClusteringResult> {
        let mut plugin = self.plugin.lock().unwrap();

        let input = json!({
            "node_count": node_count,
            "edges": edges
        })
        .to_string();

        let json_result: String = plugin.call("cluster", input).context(format!(
            "Failed to execute 'cluster' on Wasm plugin {}",
            self.plugin_path
        ))?;

        let result: ClusteringResult = serde_json::from_str(&json_result)
            .context("Failed to deserialize ClusteringResult from Wasm plugin output")?;

        Ok(result)
    }
}
