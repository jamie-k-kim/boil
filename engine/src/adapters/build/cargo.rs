use crate::ports::build::{BuildDependency, BuildProvider, PackageInfo};
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct CargoToml {
    package: Option<Package>,
    dependencies: Option<HashMap<String, toml::Value>>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
}

pub struct CargoProvider;

impl BuildProvider for CargoProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();

        for entry in walkdir::WalkDir::new(project_root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                e.depth() == 0 || (!name.starts_with('.')
                    && name != "target"
                    && name != "node_modules"
                    && name != "venv")
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == "Cargo.toml" {
                let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                if let Ok(cargo) = toml::from_str::<CargoToml>(&content)
                    && let Some(pkg) = cargo.package
                {
                    let mut dependencies = Vec::new();
                    if let Some(cargo_deps) = cargo.dependencies {
                        for dep_name in cargo_deps.keys() {
                            dependencies.push(BuildDependency {
                                name: dep_name.clone(),
                                version: "external".to_string(), // Cargo.toml parsing is complex
                                is_external: true, // We will refine this in build.rs if it's an internal package
                            });
                        }
                    }

                    let path = entry.path().parent().unwrap_or(project_root).to_path_buf();
                    packages.push(PackageInfo {
                        name: pkg.name,
                        version: pkg.version,
                        path,
                        dependencies,
                    });
                }
            }
        }
        Ok(packages)
    }
}
