use crate::ports::ownership::{OwnershipProvider, OwnershipRule};
use anyhow::Result;
use std::path::Path;

/// A provider that parses standard `CODEOWNERS` files.
///
/// It scans `.github/CODEOWNERS`, `docs/CODEOWNERS`, or `CODEOWNERS` files to map
/// source files and directories to their responsible code owners (e.g., GitHub teams).
pub struct CodeownersProvider;

impl OwnershipProvider for CodeownersProvider {
    fn get_ownership_info(&self, project_root: &Path) -> Result<Vec<OwnershipRule>> {
        let mut rules = Vec::new();

        let paths_to_check = [
            project_root.join("CODEOWNERS"),
            project_root.join(".github").join("CODEOWNERS"),
            project_root.join("docs").join("CODEOWNERS"),
        ];

        let mut content = None;
        for path in paths_to_check {
            if path.exists() {
                content = Some(std::fs::read_to_string(path)?);
                break;
            }
        }

        if let Some(text) = content {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let mut parts = line.split_whitespace();
                if let Some(pattern) = parts.next() {
                    let owners: Vec<String> = parts.map(|s| s.to_string()).collect();
                    if !owners.is_empty() {
                        rules.push(OwnershipRule {
                            pattern: pattern.to_string(),
                            owners,
                        });
                    }
                }
            }
        }

        Ok(rules)
    }
}
