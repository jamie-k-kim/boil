use crate::ports::vcs::{Author, FileProvenance, VcsProvider};
use anyhow::{Context, Result};
use extism::{Manifest, Plugin, Wasm};
use serde_json::json;
use std::path::Path;
use std::sync::Mutex;

pub struct WasmVcsProvider {
    plugin_path: String,
    plugin: Mutex<Plugin>,
}

impl WasmVcsProvider {
    pub fn new(plugin_path: &str) -> Result<Self> {
        let wasm = Wasm::file(plugin_path);
        let manifest = Manifest::new([wasm]);
        let plugin = Plugin::new(&manifest, [], true)
            .context("Failed to initialize Extism plugin for WasmVcsProvider")?;

        Ok(Self {
            plugin_path: plugin_path.to_string(),
            plugin: Mutex::new(plugin),
        })
    }
}

impl VcsProvider for WasmVcsProvider {
    fn get_file_provenance(&self, project_root: &Path, file_path: &Path) -> Result<FileProvenance> {
        let mut plugin = self.plugin.lock().unwrap();

        let input = json!({
            "project_root": project_root,
            "file_path": file_path
        })
        .to_string();

        let json_result: String = plugin.call("get_file_provenance", input).context(format!(
            "Failed to execute 'get_file_provenance' on Wasm plugin {}",
            self.plugin_path
        ))?;

        let prov: FileProvenance = serde_json::from_str(&json_result)
            .context("Failed to deserialize FileProvenance from Wasm plugin output")?;

        Ok(prov)
    }

    fn get_author_at_line(
        &self,
        project_root: &Path,
        file_path: &Path,
        line: usize,
    ) -> Result<Author> {
        let mut plugin = self.plugin.lock().unwrap();

        let input = json!({
            "project_root": project_root,
            "file_path": file_path,
            "line": line
        })
        .to_string();

        let json_result: String = plugin.call("get_author_at_line", input).context(format!(
            "Failed to execute 'get_author_at_line' on Wasm plugin {}",
            self.plugin_path
        ))?;

        let author: Author = serde_json::from_str(&json_result)
            .context("Failed to deserialize Author from Wasm plugin output")?;

        Ok(author)
    }
}
