use serde::Serialize;
use anyhow::Context;
use crate::batch::resolver::{Fidelity, Resolver};
use crate::batch::FileInfo;

#[derive(Serialize)]
struct SymbolMatch {
    name: String,
    kind: String,
    file_path: String,
    line_start: usize,
    line_end: usize,
}

fn load_index(batch: &crate::batch::Batch) -> anyhow::Result<Vec<FileInfo>> {
    let index_path = batch.root.join("index.json");
    let index_content = std::fs::read_to_string(&index_path)?;
    let index: Vec<FileInfo> = serde_json::from_str(&index_content)?;
    Ok(index)
}

pub fn run_find(batch: &crate::batch::Batch, symbol_name: String, json: bool) -> anyhow::Result<String> {
    let index = load_index(batch)?;
    run_find_with_index(&index, &symbol_name, json)
}

pub fn run_find_with_index(index: &[FileInfo], symbol_name: &str, json: bool) -> anyhow::Result<String> {
    let mut matches = Vec::new();

    for file in index {
        for sym in &file.symbols {
            if sym.name.contains(symbol_name) {
                matches.push(SymbolMatch {
                    name: sym.name.clone(),
                    kind: format!("{:?}", sym.kind),
                    file_path: file.path.to_string_lossy().to_string(),
                    line_start: sym.line_start,
                    line_end: sym.line_end,
                });
            }
        }
    }

    if json {
        Ok(serde_json::to_string_pretty(&matches)?)
    } else {
        let mut out = String::new();
        out.push_str(&format!("Found {} match(es) for '{}':\n", matches.len(), symbol_name));
        for m in matches {
            out.push_str(&format!("  [{}] {} in {} (Lines {}-{})\n", 
                m.kind, m.name, m.file_path, m.line_start, m.line_end));
        }
        Ok(out)
    }
}

pub fn run_expand(batch: &crate::batch::Batch, symbol_name: String, fidelity_str: Option<String>, id: Option<usize>, json: bool) -> anyhow::Result<String> {
    let index = load_index(batch)?;
    let mut symbol_map: std::collections::HashMap<String, Vec<(usize, usize)>> = std::collections::HashMap::new();
    for (f_idx, file) in index.iter().enumerate() {
        for (s_idx, sym) in file.symbols.iter().enumerate() {
            symbol_map.entry(sym.name.clone()).or_default().push((f_idx, s_idx));
        }
    }
    run_expand_with_index(batch, &index, &symbol_map, &symbol_name, fidelity_str, id, json)
}

pub fn run_expand_with_index(
    batch: &crate::batch::Batch,
    index: &[FileInfo],
    symbol_map: &std::collections::HashMap<String, Vec<(usize, usize)>>,
    symbol_name: &str,
    fidelity_str: Option<String>,
    id: Option<usize>,
    json: bool,
) -> anyhow::Result<String> {
    let mut candidates = Vec::new();

    if let Some(matches) = symbol_map.get(symbol_name) {
        for &(f_idx, s_idx) in matches {
            if let Some(file) = index.get(f_idx) {
                if let Some(sym) = file.symbols.get(s_idx) {
                    candidates.push((file, sym));
                }
            }
        }
    }

    let (file_info, symbol) = match candidates.len() {
        0 => anyhow::bail!("Symbol '{}' not found", symbol_name),
        1 => candidates[0],
        _ => {
            if let Some(i) = id {
                *candidates.get(i).context("Invalid ID")?
            } else {
                if json {
                    return Ok(serde_json::to_string(&serde_json::json!({ "error": "Ambiguous symbol", "candidates": candidates.len() }))?);
                } else {
                    let mut out = String::new();
                    out.push_str(&format!("Ambiguous symbol '{}'. Found {} matches:\n", symbol_name, candidates.len()));
                    for (i, (f, _)) in candidates.iter().enumerate() {
                        out.push_str(&format!("{}: {}\n", i, f.path.display()));
                    }
                    out.push_str("Use --id <ID> to disambiguate.\n");
                    return Ok(out);
                }
            }
        }
    };

    let fidelity = fidelity_str
        .map(|f| Fidelity::from_str(&f))
        .unwrap_or(Ok(Fidelity::Source))?; // Default to source for expand
    
    let resolver = Resolver::new(batch);
    let full_path = resolver.resolve_path(&file_info.path, fidelity)?;
    let content = std::fs::read_to_string(full_path)?;
    let lines: Vec<&str> = content.lines().collect();
    
    let start = symbol.line_start.saturating_sub(1);
    let end = symbol.line_end.min(lines.len());
    let code = lines[start..end].join("\n");

    if json {
        Ok(serde_json::to_string(&serde_json::json!({ "code": code }))?)
    } else {
        Ok(code)
    }
}
