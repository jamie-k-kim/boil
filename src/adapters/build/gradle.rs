use crate::ports::build::{BuildDependency, BuildProvider, PackageInfo};
use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub struct GradleProvider;

impl BuildProvider for GradleProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<PackageInfo>> {
        let has_wrapper =
            project_root.join("gradlew").exists() || project_root.join("gradlew.bat").exists();
        let has_build_file = project_root.join("build.gradle").exists()
            || project_root.join("build.gradle.kts").exists();

        if !has_build_file {
            return Ok(vec![]);
        }

        let cmd = if has_wrapper {
            if cfg!(windows) {
                "gradlew.bat"
            } else {
                "./gradlew"
            }
        } else {
            "gradle"
        };

        let output = Command::new(cmd)
            .current_dir(project_root)
            .arg("dependencies")
            .output();

        let mut packages = Vec::new();
        let mut dependencies = Vec::new();

        if let Ok(output) = output
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Gradle dependencies output lines usually look like:
                // \--- org.springframework:spring-core:5.3.9
                // +--- org.springframework:spring-core:5.3.9
                if line.contains("--- ") {
                    let parts: Vec<&str> = line.split("--- ").collect();
                    if parts.len() == 2 {
                        let dep_str = parts[1].trim();
                        // dep_str could be `org.springframework:spring-core:5.3.9` or `org.springframework:spring-core:5.3.9 (*)`, etc.
                        let clean_dep = dep_str.split(' ').next().unwrap_or(dep_str);
                        let dep_parts: Vec<&str> = clean_dep.split(':').collect();

                        let (name, version) = if dep_parts.len() >= 3 {
                            (
                                format!("{}:{}", dep_parts[0], dep_parts[1]),
                                dep_parts[2].to_string(),
                            )
                        } else if dep_parts.len() == 2 {
                            (
                                format!("{}:{}", dep_parts[0], dep_parts[1]),
                                "unknown".to_string(),
                            )
                        } else {
                            (dep_parts[0].to_string(), "unknown".to_string())
                        };

                        dependencies.push(BuildDependency {
                            name,
                            version,
                            is_external: true,
                        });
                    }
                }
            }

            packages.push(PackageInfo {
                name: "gradle-workspace".to_string(),
                version: "unknown".to_string(),
                path: project_root.to_path_buf(),
                dependencies,
            });
        }

        Ok(packages)
    }
}
