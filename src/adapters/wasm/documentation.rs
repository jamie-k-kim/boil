use crate::ports::documentation::{DocumentInfo, DocumentationProvider};
use anyhow::{Context, Result};
use extism::{Manifest, Plugin, Wasm};
use std::path::Path;
use std::sync::Mutex;

pub struct WasmDocumentationProvider {
    plugin_path: String,
    plugin: Mutex<Plugin>,
}

impl WasmDocumentationProvider {
    pub fn new(plugin_path: &str) -> Result<Self> {
        let wasm = Wasm::file(plugin_path);
        let manifest = Manifest::new([wasm]);
        let plugin = Plugin::new(&manifest, [], true)
            .context("Failed to initialize Extism plugin for WasmDocumentationProvider")?;

        Ok(Self {
            plugin_path: plugin_path.to_string(),
            plugin: Mutex::new(plugin),
        })
    }
}

impl DocumentationProvider for WasmDocumentationProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<DocumentInfo>> {
        let root_str = project_root.to_str().unwrap_or(".");
        let mut plugin = self.plugin.lock().unwrap();

        let json_result: String = plugin.call("analyze_workspace", root_str).context(format!(
            "Failed to execute 'analyze_workspace' on Wasm plugin {}",
            self.plugin_path
        ))?;

        let docs: Vec<DocumentInfo> = serde_json::from_str(&json_result)
            .context("Failed to deserialize Vec<DocumentInfo> from Wasm plugin output")?;

        Ok(docs)
    }
}
