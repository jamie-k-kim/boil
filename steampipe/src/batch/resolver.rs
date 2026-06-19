use std::path::{Path, PathBuf};
use anyhow::{Context, Result, bail};

pub enum Fidelity {
    Architectural,
    Skeletal,
    Partial,
    Source,
}

impl Fidelity {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "l0" | "partial" | "l0_partial" => Ok(Fidelity::Partial),
            "l1" | "skeletal" | "l1_skeletal" => Ok(Fidelity::Skeletal),
            "l2" | "architectural" | "l2_architectural" => Ok(Fidelity::Architectural),
            "src" | "source" => Ok(Fidelity::Source),
            _ => bail!("Unknown fidelity level: {}", s),
        }
    }

    pub fn to_layer_name(&self) -> String {
        match self {
            Fidelity::Partial => "L0_partial".to_string(),
            Fidelity::Skeletal => "L1_skeletal".to_string(),
            Fidelity::Architectural => "L2_architectural".to_string(),
            Fidelity::Source => "source".to_string(),
        }
    }
}

pub struct Resolver<'a> {
    batch: &'a crate::batch::Batch,
}

impl<'a> Resolver<'a> {
    pub fn new(batch: &'a crate::batch::Batch) -> Self {
        Self { batch }
    }

    /// Finds the project root directory inside a specific fidelity layer.
    fn get_project_root_in_layer(&self, layer_name: &str) -> Result<PathBuf> {
        let layer_root = self.batch.root.join("layers").join(layer_name);
        let entries = std::fs::read_dir(&layer_root)
            .with_context(|| format!("Failed to read layer directory: {:?}", layer_root))?;
        
        entries.filter_map(|e| e.ok())
            .find(|e| e.path().is_dir())
            .map(|e| e.path())
            .context("Could not find project root inside layer")
    }

    pub fn resolve_path(&self, rel_path: &Path, fidelity: Fidelity) -> Result<PathBuf> {
        // Strip source root prefix if the path is absolute and starts with it
        let clean_path = if rel_path.is_absolute() && rel_path.starts_with(&self.batch.source_root) {
            rel_path.strip_prefix(&self.batch.source_root).unwrap_or(rel_path)
        } else {
            rel_path
        };

        // Normalize: Strip leading slash to treat absolute-looking paths as relative to the base
        let path_str = clean_path.to_string_lossy();
        let normalized_path = Path::new(path_str.trim_start_matches('/'));

        if matches!(fidelity, Fidelity::Source) {
            return Ok(self.batch.source_root.join(normalized_path));
        }

        // Strictly forbid absolute paths (after normalization)
        if normalized_path.is_absolute() {
            anyhow::bail!("Absolute paths are not allowed. Please provide a path relative to the batch project root.");
        }

        let project_root = self.get_project_root_in_layer(&fidelity.to_layer_name())?;

        let mut full_path = project_root.join(normalized_path);

        if !full_path.exists() {
            let dstl_path = project_root.join(format!("{}.dstl", normalized_path.display()));
            if dstl_path.exists() {
                full_path = dstl_path;
            }
        }
        
        Ok(full_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::BatchManifest;
    use std::fs;

    #[test]
    fn test_fidelity_from_str() {
        assert!(matches!(Fidelity::from_str("l0").unwrap(), Fidelity::Partial));
        assert!(matches!(Fidelity::from_str("partial").unwrap(), Fidelity::Partial));
        assert!(matches!(Fidelity::from_str("l0_partial").unwrap(), Fidelity::Partial));
        assert!(matches!(Fidelity::from_str("l1").unwrap(), Fidelity::Skeletal));
        assert!(matches!(Fidelity::from_str("skeletal").unwrap(), Fidelity::Skeletal));
        assert!(matches!(Fidelity::from_str("l1_skeletal").unwrap(), Fidelity::Skeletal));
        assert!(matches!(Fidelity::from_str("l2").unwrap(), Fidelity::Architectural));
        assert!(matches!(Fidelity::from_str("architectural").unwrap(), Fidelity::Architectural));
        assert!(matches!(Fidelity::from_str("l2_architectural").unwrap(), Fidelity::Architectural));
        assert!(matches!(Fidelity::from_str("src").unwrap(), Fidelity::Source));
        assert!(matches!(Fidelity::from_str("source").unwrap(), Fidelity::Source));
        assert!(Fidelity::from_str("invalid").is_err());
    }

    #[test]
    fn test_resolve_path() {
        let temp = tempfile::tempdir().unwrap();
        let source_root = temp.path().join("src");
        fs::create_dir(&source_root).unwrap();
        
        let layers_dir = temp.path().join("layers");
        let l2_dir = layers_dir.join("L2_architectural").join("proj");
        fs::create_dir_all(&l2_dir).unwrap();

        // Write normal and distilled files
        let file_normal = l2_dir.join("lib.rs");
        fs::write(&file_normal, "normal").unwrap();
        let file_distilled = l2_dir.join("main.rs.dstl");
        fs::write(&file_distilled, "distilled").unwrap();

        let batch = crate::batch::Batch {
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

        let resolver = Resolver::new(&batch);

        // 1. Test source path resolution
        let resolved_src = resolver.resolve_path(Path::new("src/main.rs"), Fidelity::Source).unwrap();
        assert_eq!(resolved_src, source_root.join("src/main.rs"));

        // 2. Test L2 normal file resolution
        let resolved_normal = resolver.resolve_path(Path::new("lib.rs"), Fidelity::Architectural).unwrap();
        assert_eq!(resolved_normal, file_normal);

        // 3. Test L2 distilled file resolution (.dstl suffix fallback)
        let resolved_dstl = resolver.resolve_path(Path::new("main.rs"), Fidelity::Architectural).unwrap();
        assert_eq!(resolved_dstl, file_distilled);
    }
}
