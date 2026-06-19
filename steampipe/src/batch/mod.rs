use anyhow::Context;

pub mod fidelity;
pub mod resolver;
pub mod distillation;

pub struct Batch {
    pub root: std::path::PathBuf,
    pub source_root: std::path::PathBuf,
    pub manifest: crate::batch::BatchManifest,
    pub embedding_provider: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub enum SymbolKind {
    Function, Method, Class, Struct, Enum, Interface, Trait, Module, Constant, Variable,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line_start: usize,
    pub line_end: usize,
    pub exported: bool,
    pub signature: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct FileInfo {
    pub path: std::path::PathBuf,
    pub symbols: Vec<Symbol>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct BatchManifest {
    pub source: String,
    pub canon: String,
    pub created: String,
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,
}

fn default_embedding_provider() -> String {
    "none".to_string()
}

impl Batch {
    pub fn load(root: std::path::PathBuf) -> anyhow::Result<Self> {
        // 1. Validate structure
        let manifest_path = root.join("batch_manifest.toml");
        if !manifest_path.exists() {
            anyhow::bail!("Invalid batch (batch_manifest.toml not found)");
        }

        // 2. Load and validate manifest content
        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest: BatchManifest = toml::from_str(&content)
            .context("Invalid batch (batch_manifest.toml is malformed)")?;

        // 3. Validate layers on disk
        let layers_dir = root.join("layers");
        for fidelity in crate::batch::fidelity::Fidelity::all() {
            let layer = fidelity.label();
            if !layers_dir.join(layer).exists() {
                anyhow::bail!("Invalid batch (missing layer: {})", layer);
            }
        }
        
        let source_root = std::path::PathBuf::from(&manifest.source);
        let embedding_provider = manifest.embedding_provider.clone();

        Ok(Self { root, source_root, manifest, embedding_provider })
    }
}
