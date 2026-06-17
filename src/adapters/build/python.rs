use crate::ports::build::{BuildDependency, BuildProvider, PackageInfo};
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct PyProject {
    project: Option<Project>,
    tool: Option<Tool>,
}

#[derive(Debug, Deserialize)]
struct Project {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Tool {
    poetry: Option<Poetry>,
}

#[derive(Debug, Deserialize)]
struct Poetry {
    name: Option<String>,
    version: Option<String>,
    dependencies: Option<HashMap<String, serde_json::Value>>,
}

pub struct PythonProvider;

impl BuildProvider for PythonProvider {
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
            let file_name = entry.file_name().to_string_lossy();
            if file_name == "pyproject.toml" {
                let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                if let Ok(toml) = toml::from_str::<PyProject>(&content) {
                    let mut found_name = None;
                    let mut found_version = None;
                    let mut dependencies = Vec::new();

                    if let Some(proj) = toml.project {
                        if let (Some(n), Some(v)) = (proj.name, proj.version) {
                            found_name = Some(n);
                            found_version = Some(v);
                        }
                        if let Some(deps) = proj.dependencies {
                            for dep in deps {
                                let name = dep
                                    .split(['=', '<', '>', '~'])
                                    .next()
                                    .unwrap_or(&dep)
                                    .trim()
                                    .to_string();
                                dependencies.push(BuildDependency {
                                    name,
                                    version: "external".to_string(),
                                    is_external: true,
                                });
                            }
                        }
                    } else if let Some(tool) = toml.tool
                        && let Some(poetry) = tool.poetry
                    {
                        if let (Some(n), Some(v)) = (poetry.name, poetry.version) {
                            found_name = Some(n);
                            found_version = Some(v);
                        }
                        if let Some(deps) = poetry.dependencies {
                            for dep_name in deps.keys() {
                                dependencies.push(BuildDependency {
                                    name: dep_name.clone(),
                                    version: "external".to_string(),
                                    is_external: true,
                                });
                            }
                        }
                    }

                    if let (Some(name), Some(version)) = (found_name, found_version) {
                        let path = entry.path().parent().unwrap_or(project_root).to_path_buf();
                        packages.push(PackageInfo {
                            name,
                            version,
                            path,
                            dependencies,
                        });
                    }
                }
            } else if file_name == "requirements.txt" {
                let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                let mut dependencies = Vec::new();
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        let name = line
                            .split(['=', '<', '>', '~'])
                            .next()
                            .unwrap_or(line)
                            .trim()
                            .to_string();
                        dependencies.push(BuildDependency {
                            name,
                            version: "external".to_string(),
                            is_external: true,
                        });
                    }
                }

                if !dependencies.is_empty() {
                    let path = entry.path().parent().unwrap_or(project_root).to_path_buf();
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    packages.push(PackageInfo {
                        name: if name.is_empty() {
                            "python-project".to_string()
                        } else {
                            name
                        },
                        version: "0.0.0".to_string(),
                        path,
                        dependencies,
                    });
                }
            }
        }
        Ok(packages)
    }
}
