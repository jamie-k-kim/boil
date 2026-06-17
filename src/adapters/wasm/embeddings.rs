use crate::ports::embeddings::EmbeddingProvider;
use anyhow::{Context, Result};
use extism::{Manifest, Plugin, Wasm};
use std::sync::Mutex;

pub struct WasmEmbeddingProvider {
    plugin_path: String,
    plugin: Mutex<Plugin>,
}

impl WasmEmbeddingProvider {
    pub fn new(plugin_path: &str) -> Result<Self> {
        let wasm = Wasm::file(plugin_path);
        let manifest = Manifest::new([wasm]);
        let plugin = Plugin::new(&manifest, [], true)
            .context("Failed to initialize Extism plugin for WasmEmbeddingProvider")?;

        Ok(Self {
            plugin_path: plugin_path.to_string(),
            plugin: Mutex::new(plugin),
        })
    }
}

impl EmbeddingProvider for WasmEmbeddingProvider {
    fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut plugin = self.plugin.lock().unwrap();

        let input = serde_json::to_string(&texts)?;

        let json_result: String = plugin.call("embed", input).context(format!(
            "Failed to execute 'embed' on Wasm plugin {}",
            self.plugin_path
        ))?;

        let embeddings: Vec<Vec<f32>> = serde_json::from_str(&json_result)
            .context("Failed to deserialize embeddings from Wasm plugin output")?;

        Ok(embeddings)
    }
}
