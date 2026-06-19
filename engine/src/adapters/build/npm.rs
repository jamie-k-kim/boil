use crate::ports::build::{BuildDependency, BuildProvider, PackageInfo};
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct PackageJson {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
}

pub struct NpmProvider;

impl BuildProvider for NpmProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();

        for entry in walkdir::WalkDir::new(project_root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
                    && name != "target"
                    && name != "node_modules"
                    && name != "venv"
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == "package.json" {
                let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                if let Ok(pkg) = serde_json::from_str::<PackageJson>(&content)
                    && let (Some(name), Some(version)) = (pkg.name, pkg.version)
                {
                    let mut dependencies = Vec::new();

                    if let Some(deps) = pkg.dependencies {
                        for (dep_name, dep_version) in deps {
                            dependencies.push(BuildDependency {
                                name: dep_name,
                                version: dep_version,
                                is_external: true,
                            });
                        }
                    }

                    if let Some(dev_deps) = pkg.dev_dependencies {
                        for (dep_name, dep_version) in dev_deps {
                            dependencies.push(BuildDependency {
                                name: dep_name,
                                version: dep_version,
                                is_external: true,
                            });
                        }
                    }

                    let path = entry.path().parent().unwrap_or(project_root).to_path_buf();
                    packages.push(PackageInfo {
                        name,
                        version,
                        path,
                        dependencies,
                    });
                }
            }
        }
        Ok(packages)
    }
}
