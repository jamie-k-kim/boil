use crate::ports::embeddings::EmbeddingProvider;
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Mutex;

/// A local embedding provider leveraging the FastEmbed Rust crate and ONNX Runtime.
///
/// It lazily downloads and runs a compact dense embedding model (e.g., `AllMiniLML6V2`)
/// entirely locally, ensuring data privacy and fast semantic feature extraction without network overhead.
pub struct FastEmbedProvider {
    model: Mutex<Option<TextEmbedding>>,
}

impl Default for FastEmbedProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl FastEmbedProvider {
    pub fn new() -> Self {
        Self {
            model: Mutex::new(None),
        }
    }
}

impl EmbeddingProvider for FastEmbedProvider {
    fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut model_guard = self
            .model
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock failed: {:?}", e))?;

        if model_guard.is_none() {
            #[cfg(target_os = "macos")]
            {
                if std::env::var("ORT_DYLIB_PATH").is_err() {
                    let paths = [
                        "/opt/homebrew/lib/libonnxruntime.dylib",
                        "/usr/local/lib/libonnxruntime.dylib",
                    ];
                    for path in &paths {
                        if std::path::Path::new(path).exists() {
                            unsafe {
                                std::env::set_var("ORT_DYLIB_PATH", path);
                            }
                            break;
                        }
                    }
                }
            }

            let _ = ort::init().commit();

            println!(
                "  {} Initializing FastEmbed (will download ~100MB model on first run)...",
                console::style(">>").yellow()
            );

            let model = TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
            )
            .map_err(|e| anyhow::anyhow!("Failed to initialize FastEmbed: {:?}", e))?;

            *model_guard = Some(model);
        }

        model_guard
            .as_mut()
            .unwrap()
            .embed(texts, None)
            .map_err(|e| anyhow::anyhow!("Embedding failed: {:?}", e))
    }
}
