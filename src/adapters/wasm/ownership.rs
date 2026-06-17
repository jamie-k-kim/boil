use crate::ports::ownership::{OwnershipProvider, OwnershipRule};
use anyhow::{Context, Result};
use extism::{Manifest, Plugin, Wasm};
use std::path::Path;
use std::sync::Mutex;

pub struct WasmOwnershipProvider {
    plugin_path: String,
    // We keep a mutex over the plugin because Extism Plugins are not strictly Send+Sync safe for concurrent calls without a lock
    plugin: Mutex<Plugin>,
}

impl WasmOwnershipProvider {
    pub fn new(plugin_path: &str) -> Result<Self> {
        let wasm = Wasm::file(plugin_path);
        let manifest = Manifest::new([wasm]);
        let plugin = Plugin::new(&manifest, [], true)
            .context("Failed to initialize Extism plugin for WasmOwnershipProvider")?;

        Ok(Self {
            plugin_path: plugin_path.to_string(),
            plugin: Mutex::new(plugin),
        })
    }
}

impl OwnershipProvider for WasmOwnershipProvider {
    fn get_ownership_info(&self, project_root: &Path) -> Result<Vec<OwnershipRule>> {
        let root_str = project_root.to_str().unwrap_or(".");
        let mut plugin = self.plugin.lock().unwrap();

        let json_result: String = plugin
            .call("get_ownership_info", root_str)
            .context(format!(
                "Failed to execute 'get_ownership_info' on Wasm plugin {}",
                self.plugin_path
            ))?;

        let rules: Vec<OwnershipRule> = serde_json::from_str(&json_result)
            .context("Failed to deserialize OwnershipRule from Wasm plugin output")?;

        Ok(rules)
    }
}
