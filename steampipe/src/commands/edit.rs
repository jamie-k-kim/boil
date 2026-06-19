use std::fs;
use std::path::{Path, PathBuf};
use crate::batch::{Batch, resolver::{Fidelity, Resolver}};
use boil_core::canon::state::CanonState;
use boil_engine::core::EngineConfig;

pub fn run_write(batch: &Batch, file: String, line: usize, content: Option<String>) -> anyhow::Result<()> {
    let resolver = Resolver::new(batch);
    let path = resolver.resolve_path(&PathBuf::from(&file), Fidelity::Source)?;
    
    if line == 0 {
        anyhow::bail!("Line number must be 1 or greater");
    }
    
    let mut lines = fs::read_to_string(&path)?
        .lines()
        .map(|l| l.to_string())
        .collect::<Vec<String>>();

    let new_line = content.unwrap_or_default();
    
    if line - 1 >= lines.len() {
        while lines.len() < line - 1 {
            lines.push(String::new());
        }
        lines.push(new_line);
    } else {
        lines.insert(line - 1, new_line);
    }

    fs::write(&path, lines.join("\n"))?;

    if let Err(e) = apply_canon_patch(batch, &path) {
        eprintln!("Warning: canon patch failed (source edit was still applied): {}", e);
    }

    Ok(())
}

pub fn run_delete(batch: &Batch, file: String, line: usize) -> anyhow::Result<()> {
    let resolver = Resolver::new(batch);
    let path = resolver.resolve_path(&PathBuf::from(&file), Fidelity::Source)?;
    
    let mut lines = fs::read_to_string(&path)?
        .lines()
        .map(|l| l.to_string())
        .collect::<Vec<String>>();

    if line > 0 && line <= lines.len() {
        lines.remove(line - 1);
    }

    fs::write(&path, lines.join("\n"))?;

    if let Err(e) = apply_canon_patch(batch, &path) {
        eprintln!("Warning: canon patch failed (source edit was still applied): {}", e);
    }

    Ok(())
}

/// Surgically updates canon.bin after a source file edit.
/// Uses the embedding provider recorded in the batch manifest.
/// Prints a warning (but does not abort) if the canon cannot be patched.
fn apply_canon_patch(batch: &Batch, changed_file: &Path) -> anyhow::Result<()> {
    let mut canon_bin = PathBuf::from(&batch.manifest.canon);
    if canon_bin.is_dir() {
        canon_bin = canon_bin.join("canon.bin");
    }
    if !canon_bin.exists() {
        anyhow::bail!("canon.bin not found at {}", canon_bin.display());
    }

    let mut state = CanonState::load(&canon_bin)?;

    let syntax_provider = boil_engine::adapters::syntax::TreeSitterProvider::new();

    let embedding_provider: Box<dyn boil_engine::ports::EmbeddingProvider> =
        match batch.embedding_provider.as_str() {
            "openai"     => Box::new(boil_engine::adapters::embeddings::openai::OpenAiProvider::new()),
            "ollama"     => Box::new(boil_engine::adapters::embeddings::ollama::OllamaProvider::new()),
            "fastembed"  => Box::new(boil_engine::adapters::embeddings::FastEmbedProvider::new()),
            "voyageai"   => Box::new(boil_engine::adapters::embeddings::voyageai::VoyageAiProvider::new()),
            "cohere"     => Box::new(boil_engine::adapters::embeddings::cohere::CohereProvider::new()),
            _            => Box::new(boil_engine::adapters::null::NullEmbeddingProvider),
        };

    let reasoning_modules: Vec<Box<dyn boil_engine::ports::ReasoningModule>> = vec![
        Box::new(boil_engine::adapters::reasoning::SemanticsModule::new(embedding_provider)),
        Box::new(boil_engine::adapters::reasoning::ArchitectureAnalyzer::new(
            Box::new(boil_engine::adapters::clustering::LeidenClusteringProvider),
        )),
    ];

    let config = EngineConfig::default();

    boil_engine::core::canon::patch::patch_file(
        &mut state,
        &canon_bin,
        changed_file,
        &syntax_provider,
        &reasoning_modules,
        &config,
    )?;

    // Patch distilled layers
    if let Err(e) = crate::batch::distillation::patch_batch_layers(batch, changed_file, &state) {
        eprintln!("Warning: Failed to patch batch layers: {}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::batch::BatchManifest;

    #[test]
    fn test_write_and_delete() {
        let temp = tempfile::tempdir().unwrap();
        let source_root = temp.path().join("src");
        fs::create_dir(&source_root).unwrap();

        let file_path = source_root.join("main.rs");
        fs::write(&file_path, "line 1\nline 2\nline 3").unwrap();

        let batch = Batch {
            root: temp.path().to_path_buf(),
            source_root: source_root.to_path_buf(),
            manifest: BatchManifest {
                source: source_root.to_string_lossy().to_string(),
                canon: temp.path().join("canon.bin").to_string_lossy().to_string(),
                created: "test".to_string(),
                embedding_provider: "none".to_string(),
            },
            embedding_provider: "none".to_string(),
        };

        // 1. Test run_write insertion at line 1 (front of file)
        run_write(&batch, "main.rs".to_string(), 1, Some("inserted line 1".to_string())).unwrap();
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "inserted line 1\nline 1\nline 2\nline 3");

        // 2. Test run_write insertion with exclamation mark (should be allowed now!)
        run_write(&batch, "main.rs".to_string(), 2, Some("println!(\"hello\");".to_string())).unwrap();
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "inserted line 1\nprintln!(\"hello\");\nline 1\nline 2\nline 3");

        // 3. Test run_write appending/padding beyond current file length
        run_write(&batch, "main.rs".to_string(), 8, Some("appended line 8".to_string())).unwrap();
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            content,
            "inserted line 1\nprintln!(\"hello\");\nline 1\nline 2\nline 3\n\n\nappended line 8"
        );

        // 4. Test run_delete
        run_delete(&batch, "main.rs".to_string(), 1).unwrap(); // removes "inserted line 1"
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            content,
            "println!(\"hello\");\nline 1\nline 2\nline 3\n\n\nappended line 8"
        );

        run_delete(&batch, "main.rs".to_string(), 1).unwrap(); // removes "println!("hello");"
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            content,
            "line 1\nline 2\nline 3\n\n\nappended line 8"
        );
    }
}
