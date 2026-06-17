use crate::ports::runtime::{RuntimeProvider, RuntimeTrace};
use anyhow::{Context, Result};
use extism::{Manifest, Plugin, Wasm};
use std::path::Path;
use std::sync::Mutex;

pub struct WasmRuntimeProvider {
    plugin_path: String,
    plugin: Mutex<Plugin>,
}

impl WasmRuntimeProvider {
    pub fn new(plugin_path: &str) -> Result<Self> {
        let wasm = Wasm::file(plugin_path);
        let manifest = Manifest::new([wasm]);
        let plugin = Plugin::new(&manifest, [], true)
            .context("Failed to initialize Extism plugin for WasmRuntimeProvider")?;

        Ok(Self {
            plugin_path: plugin_path.to_string(),
            plugin: Mutex::new(plugin),
        })
    }
}

impl RuntimeProvider for WasmRuntimeProvider {
    fn get_traces(&self, project_root: &Path) -> Result<Vec<RuntimeTrace>> {
        let root_str = project_root.to_str().unwrap_or(".");
        let mut plugin = self.plugin.lock().unwrap();

        let json_result: String = plugin.call("get_traces", root_str).context(format!(
            "Failed to execute 'get_traces' on Wasm plugin {}",
            self.plugin_path
        ))?;

        let traces: Vec<RuntimeTrace> = serde_json::from_str(&json_result)
            .context("Failed to deserialize Vec<RuntimeTrace> from Wasm plugin output")?;

        Ok(traces)
    }
}
