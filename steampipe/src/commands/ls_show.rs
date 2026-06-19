use serde::Serialize;
use crate::batch::resolver::{Fidelity, Resolver};

#[derive(Serialize)]
struct FileEntry {
    name: String,
    is_dir: bool,
}

pub fn run_ls(batch: &crate::batch::Batch, path: Option<String>, fidelity_str: Option<String>, json: bool) -> anyhow::Result<String> {
    let fidelity = fidelity_str
        .map(|f| Fidelity::from_str(&f))
        .unwrap_or(Ok(Fidelity::Architectural))?;
    
    let resolver = Resolver::new(batch);
    let search_path = resolver.resolve_path(&std::path::PathBuf::from(path.unwrap_or_default()), fidelity)?;

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(search_path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.path().is_dir();
        entries.push(FileEntry { name, is_dir });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    if json {
        Ok(serde_json::to_string_pretty(&entries)?)
    } else {
        let mut out = String::new();
        for entry in entries {
            if entry.is_dir {
                out.push_str(&format!("  {}/\n", console::style(entry.name).bold().blue()));
            } else {
                let display_name = entry.name.strip_suffix(".dstl").unwrap_or(&entry.name);
                out.push_str(&format!("  {}\n", display_name));
            }
        }
        Ok(out)
    }
}

pub fn run_show(batch: &crate::batch::Batch, file: String, fidelity_str: Option<String>, json: bool) -> anyhow::Result<String> {
    let fidelity = fidelity_str
        .map(|f| Fidelity::from_str(&f))
        .unwrap_or(Ok(Fidelity::Partial))?;
        
    let resolver = Resolver::new(batch);
    let full_path = resolver.resolve_path(&std::path::PathBuf::from(&file), fidelity)?;
    let content = std::fs::read_to_string(full_path)?;

    if json {
        Ok(serde_json::to_string(&serde_json::json!({ "content": content }))?)
    } else {
        Ok(content)
    }
}
