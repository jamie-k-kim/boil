use crate::core::canon::FileInfo;
use crate::ports::syntax::SyntaxProvider;
use anyhow::{Context, Result};
use extism::{Manifest, Plugin, Wasm};
use serde_json::json;
use std::path::Path;
use std::sync::Mutex;

pub struct WasmSyntaxProvider {
    plugin_path: String,
    plugin: Mutex<Plugin>,
}

impl WasmSyntaxProvider {
    pub fn new(plugin_path: &str) -> Result<Self> {
        let wasm = Wasm::file(plugin_path);
        let manifest = Manifest::new([wasm]);
        let plugin = Plugin::new(&manifest, [], true)
            .context("Failed to initialize Extism plugin for WasmSyntaxProvider")?;

        Ok(Self {
            plugin_path: plugin_path.to_string(),
            plugin: Mutex::new(plugin),
        })
    }
}

impl SyntaxProvider for WasmSyntaxProvider {
    fn parse_file(&self, path: &Path, source: &str) -> Result<FileInfo> {
        let mut plugin = self.plugin.lock().unwrap();

        let input = json!({
            "path": path,
            "source": source
        })
        .to_string();

        let json_result: String = plugin.call("parse_file", input).context(format!(
            "Failed to execute 'parse_file' on Wasm plugin {}",
            self.plugin_path
        ))?;

        let file_info: FileInfo = serde_json::from_str(&json_result)
            .context("Failed to deserialize FileInfo from Wasm plugin output")?;

        Ok(file_info)
    }
}
