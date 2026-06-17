//! Build system adapters.
//!
//! These adapters analyze dependency manifests (e.g., `Cargo.toml`, `package.json`)
//! and extract external dependencies as graph nodes.

pub mod bazel;
pub mod cargo;
pub mod gradle;
pub mod npm;
pub mod python;

pub use bazel::BazelProvider;
pub use cargo::CargoProvider;
pub use gradle::GradleProvider;
pub use npm::NpmProvider;
pub use python::PythonProvider;

use crate::ports::build::{BuildProvider, PackageInfo};
use anyhow::Result;
use std::path::Path;

/// A composite build provider that orchestrates multiple build system adapters.
///
/// It runs all supported build providers sequentially (e.g., Cargo, NPM, Python)
/// and aggregates the extracted package dependencies from all ecosystems.
pub struct CompositeBuildProvider {
    providers: Vec<Box<dyn BuildProvider>>,
}

impl Default for CompositeBuildProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeBuildProvider {
    pub fn new() -> Self {
        Self {
            providers: vec![
                Box::new(CargoProvider),
                Box::new(NpmProvider),
                Box::new(PythonProvider),
                Box::new(BazelProvider),
                Box::new(GradleProvider),
            ],
        }
    }
}

impl BuildProvider for CompositeBuildProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();
        for provider in &self.providers {
            if let Ok(mut pkgs) = provider.analyze_workspace(project_root) {
                packages.append(&mut pkgs);
            }
        }
        Ok(packages)
    }
}
