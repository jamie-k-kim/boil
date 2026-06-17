use crate::ports::build::{BuildDependency, BuildProvider, PackageInfo};
use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub struct BazelProvider;

impl BuildProvider for BazelProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<PackageInfo>> {
        // If there's no WORKSPACE or MODULE.bazel file, it's not a bazel project.
        if !project_root.join("WORKSPACE").exists()
            && !project_root.join("WORKSPACE.bazel").exists()
            && !project_root.join("MODULE.bazel").exists()
        {
            return Ok(vec![]);
        }

        let output = Command::new("bazel")
            .current_dir(project_root)
            .arg("query")
            .arg("deps(//...)")
            .output();

        let mut packages = Vec::new();
        let mut dependencies = Vec::new();

        if let Ok(output) = output
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    dependencies.push(BuildDependency {
                        name: line.to_string(),
                        version: "unknown".to_string(),
                        is_external: line.starts_with('@'),
                    });
                }
            }

            // For simplicity, we lump all dependencies into a single workspace-level package
            // since standard `deps(//...)` returns a flat list of labels.
            packages.push(PackageInfo {
                name: "bazel-workspace".to_string(),
                version: "unknown".to_string(),
                path: project_root.to_path_buf(),
                dependencies,
            });
        }

        Ok(packages)
    }
}
