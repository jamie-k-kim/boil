use crate::ports::documentation::{DocumentInfo, DocumentationProvider};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// A local Markdown documentation provider.
///
/// Scans the repository for standard documentation files such as `README.md`,
/// Architecture Decision Records (ADRs), and architecture documents, parsing
/// them into graph nodes.
pub struct MarkdownProvider;

impl DocumentationProvider for MarkdownProvider {
    fn analyze_workspace(&self, project_root: &Path) -> Result<Vec<DocumentInfo>> {
        let mut documents = Vec::new();

        for entry in WalkDir::new(project_root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != ".git" && name != "node_modules" && name != "target"
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                let path_str = path.to_string_lossy().to_lowercase();

                let is_readme = path_str.ends_with("readme.md");
                let is_adr = path_str.contains("/docs/adr/")
                    || path_str.contains("/doc/adr/")
                    || path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .starts_with("ADR-");
                let is_arch = path_str.contains("/docs/architecture/");

                if is_readme || is_adr || is_arch {
                    let kind = if is_readme {
                        "readme".to_string()
                    } else if is_adr {
                        "adr".to_string()
                    } else {
                        "architecture".to_string()
                    };

                    if let Ok(content) = std::fs::read_to_string(path) {
                        println!("Found document: {:?}", path);
                        let sections = parse_markdown_sections(&content);
                        documents.push(DocumentInfo {
                            path: path.to_path_buf(),
                            kind,
                            sections,
                        });
                    }
                }
            }
        }

        Ok(documents)
    }
}

fn parse_markdown_sections(content: &str) -> HashMap<String, String> {
    let mut sections = HashMap::new();
    let mut current_header = "Intro".to_string();
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ") {
            if !current_content.trim().is_empty() {
                sections.insert(current_header.clone(), current_content.trim().to_string());
            }
            current_header = line.trim_start_matches('#').trim().to_string();
            current_content.clear();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    if !current_content.trim().is_empty() {
        sections.insert(current_header, current_content.trim().to_string());
    }

    sections
}
