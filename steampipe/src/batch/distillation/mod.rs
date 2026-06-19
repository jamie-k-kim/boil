pub mod transforms;
pub mod compression;
pub mod target_planner;
pub mod scoring;

use boil_core::canon::{FileInfo, ProjectGraph};
use boil_core::utils::{count_tokens, matches_globset};
use boil_engine::adapters::input::syntax::parser;
use anyhow::Result;
use std::path::{Path, PathBuf};
use chrono::{Local, Utc};
use std::collections::{HashMap, HashSet};
use serde::Serialize;
use tokenizers::Tokenizer;
use std::sync::OnceLock;

use crate::batch::fidelity::Fidelity;

pub fn get_shared_tokenizer() -> Option<&'static Tokenizer> {
    static TOKENIZER: OnceLock<Option<Tokenizer>> = OnceLock::new();
    TOKENIZER.get_or_init(|| Tokenizer::from_pretrained("gpt2", None).ok()).as_ref()
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    Bytes(usize),
    Percent(f64),
}

pub struct DistillationConfig {
    pub raw: globset::GlobSet,
    pub focus: globset::GlobSet,
    pub target: Option<Target>,
    pub force_timestamp: Option<String>,
    pub silent: bool,
    pub embedding_provider: String,
}

#[derive(Serialize, serde::Deserialize)]
struct Manifest {
    source: String,
    canon: String,
    embedding_provider: String,
    created: String,
    original_tokens: usize,
    final_tokens: usize,
    fidelity: Option<Fidelity>,
    compression_mode: Option<String>,
    compression_target: Option<f64>,
    target_tokens: Option<usize>,
    target_achieved: Option<bool>,
    actual_compression: f64,
    focused_files: Vec<String>,
    user_ignored_files: Vec<String>,
    system_ignored_files: Vec<String>,
    #[serde(default)]
    layers: Vec<String>,
}

pub fn distill_canon_batch(
    project_root: &Path,
    distillery_root: &Path,
    canon_path: &Path,
    config: &DistillationConfig,
    file_infos: &mut Vec<FileInfo>,
    _graph: &mut ProjectGraph,
) -> Result<()> {
    let tokenizer = get_shared_tokenizer();
    let abs_project_root = std::fs::canonicalize(project_root).unwrap_or_else(|_| project_root.to_path_buf());

    let mut source_map = HashMap::new();
    let mut tree_map = HashMap::new();
    let mut raw_paths = HashSet::new();
    let mut focus_paths = HashSet::new();
    let mut focused_files = Vec::new();
    let user_ignored_files = Vec::new();
    let system_ignored_files = Vec::new();
    let mut original_token_counts = HashMap::new();

    let mut total_before_tokens = 0;

    for info in file_infos.iter() {
        let relative = boil_core::utils::relative_path(&abs_project_root, &info.path);
        let raw = matches_globset(&config.raw, &relative, &info.path);
        let focused = matches_globset(&config.focus, &relative, &info.path);
        
        if raw { raw_paths.insert(info.path.clone()); }
        if focused { 
            focus_paths.insert(info.path.clone()); 
            focused_files.push(relative.to_string_lossy().to_string());
        }

        if let Ok(source) = std::fs::read_to_string(&info.path) {
            let tokens = count_tokens(tokenizer, &source);
            total_before_tokens += tokens;
            original_token_counts.insert(info.path.clone(), tokens);
            source_map.insert(info.path.clone(), source.clone());
            
            let p = parser::create_parser(&info.language);
            if let Some(mut p) = p {
                if let Ok(tree) = parser::parse_source(&mut p, &source) {
                    tree_map.insert(info.path.clone(), tree);
                }
            }
        }
    }

    let now = Utc::now();
    let timestamp = config.force_timestamp.clone().unwrap_or_else(|| Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
    let abs_input = abs_project_root.clone();

    let boil_root = distillery_root.join(format!("batch_{}", timestamp));
    
    for fidelity in Fidelity::all() {
        let mut layer_sources = HashMap::new();
        let mut layer_after_tokens = 0;
        let layer_root = boil_root.join("layers").join(fidelity.label());

        for file_info in file_infos.iter() {
            if raw_paths.contains(&file_info.path) { continue; }
            let Some(source) = source_map.get(&file_info.path) else { continue; };
            let Some(tree) = tree_map.get(&file_info.path) else { continue; };
            let lang = &file_info.language;

            let mut edits: Vec<boil_engine::adapters::input::syntax::parser::edits::Edit> = Vec::new();
            let mut strategies: Vec<Box<dyn crate::batch::distillation::transforms::CompressionStrategy>> = Vec::new();

            match fidelity {
                Fidelity::L0 => {
                    strategies.push(Box::new(crate::batch::distillation::transforms::comments_ast::CommentRemoval));
                    strategies.push(Box::new(crate::batch::distillation::transforms::debug_remover::DebugRemover));
                    strategies.push(Box::new(crate::batch::distillation::transforms::literals::LiteralShrinker::new(100)));
                }
                Fidelity::L1 => {
                    strategies.push(Box::new(crate::batch::distillation::transforms::comments_ast::CommentRemoval));
                    strategies.push(Box::new(crate::batch::distillation::transforms::debug_remover::DebugRemover));
                    strategies.push(Box::new(crate::batch::distillation::transforms::literals::LiteralShrinker::new(100)));
                    strategies.push(Box::new(crate::batch::distillation::transforms::skeleton::Skeletonizer::new(
                        crate::batch::distillation::transforms::skeleton::SkeletonMode::TypesAndSignatures,
                    )));
                }
                Fidelity::L2 => {
                    strategies.push(Box::new(crate::batch::distillation::transforms::comments_ast::CommentRemoval));
                    strategies.push(Box::new(crate::batch::distillation::transforms::debug_remover::DebugRemover));
                    strategies.push(Box::new(crate::batch::distillation::transforms::literals::LiteralShrinker::new(100)));
                    strategies.push(Box::new(crate::batch::distillation::transforms::import_simplifier::ImportSimplifier));
                    strategies.push(Box::new(crate::batch::distillation::transforms::skeleton::Skeletonizer::new(
                        crate::batch::distillation::transforms::skeleton::SkeletonMode::ArchitecturalOnly,
                    )));
                }
            }

            for strategy in strategies {
                if strategy.is_safe_for_lang(lang) {
                    edits.extend(strategy.get_edits(source, tree, lang, file_info));
                }
            }

            if !edits.is_empty() {
                layer_sources.insert(file_info.path.clone(), parser::apply_edits(source, edits));
            }
        }

        for info in file_infos.iter() {
            let compressed = if let Some(comp) = layer_sources.get(&info.path) {
                layer_after_tokens += count_tokens(tokenizer, comp);
                comp
            } else {
                if let Some(tokens) = original_token_counts.get(&info.path) {
                    layer_after_tokens += *tokens;
                }
                source_map.get(&info.path).unwrap()
            };

            let out = build_output_path(&info.path, &abs_project_root, &layer_root, true);
            if let Some(parent) = out.parent() { std::fs::create_dir_all(parent)?; }
            std::fs::write(&out, compressed)?;
        }

        let raw_compression = if total_before_tokens > 0 {
            (1.0 - (layer_after_tokens as f64 / total_before_tokens as f64)) * 100.0
        } else {
            0.0
        };

        let manifest = Manifest {
            source: abs_input.to_string_lossy().to_string(),
            canon: canon_path.to_string_lossy().to_string(),
            embedding_provider: config.embedding_provider.clone(),
            created: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            original_tokens: total_before_tokens,
            final_tokens: layer_after_tokens,
            fidelity: Some(fidelity),
            compression_mode: None,
            compression_target: None,
            target_tokens: None,
            target_achieved: None,
            actual_compression: (raw_compression * 100.0).round() / 100.0,
            focused_files: focused_files.clone(),
            user_ignored_files: user_ignored_files.clone(),
            system_ignored_files: system_ignored_files.clone(),
            layers: Vec::new(),
        };

        std::fs::write(layer_root.join("dstl_manifest.toml"), toml::to_string_pretty(&manifest)?)?;
        std::fs::write(layer_root.join("index.json"), serde_json::to_string_pretty(file_infos)?)?;
    }

    let batch_manifest = crate::batch::BatchManifest {
        source: abs_input.to_string_lossy().to_string(),
        canon: canon_path.to_string_lossy().to_string(),
        created: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        embedding_provider: config.embedding_provider.clone(),
    };
    std::fs::write(boil_root.join("batch_manifest.toml"), toml::to_string_pretty(&batch_manifest)?)?;
    std::fs::write(boil_root.join("index.json"), serde_json::to_string_pretty(file_infos)?)?;

    if !config.silent {
        println!("Distilled batch layers to {}", boil_root.display());
    }

    Ok(())
}

pub fn distill_canon(
    project_root: &Path,
    distillery_root: &Path,
    canon_path: &Path,
    config: &DistillationConfig,
    file_infos: &mut Vec<FileInfo>,
    graph: &mut ProjectGraph,
) -> Result<()> {
    let tokenizer = get_shared_tokenizer();
    let abs_project_root = std::fs::canonicalize(project_root).unwrap_or_else(|_| project_root.to_path_buf());

    let mut source_map = HashMap::new();
    let mut tree_map = HashMap::new();
    let mut raw_paths = HashSet::new();
    let mut focus_paths = HashSet::new();
    let mut focused_files = Vec::new();
    let user_ignored_files = Vec::new();
    let system_ignored_files = Vec::new();
    let mut original_token_counts = HashMap::new();

    let mut total_before_tokens = 0;

    for info in file_infos.iter() {
        let relative = boil_core::utils::relative_path(&abs_project_root, &info.path);
        let raw = matches_globset(&config.raw, &relative, &info.path);
        let focused = matches_globset(&config.focus, &relative, &info.path);
        
        if raw { raw_paths.insert(info.path.clone()); }
        if focused { 
            focus_paths.insert(info.path.clone()); 
            focused_files.push(relative.to_string_lossy().to_string());
        }

        if let Ok(source) = std::fs::read_to_string(&info.path) {
            let tokens = count_tokens(tokenizer, &source);
            total_before_tokens += tokens;
            original_token_counts.insert(info.path.clone(), tokens);
            source_map.insert(info.path.clone(), source.clone());
            
            let p = parser::create_parser(&info.language);
            if let Some(mut p) = p {
                if let Ok(tree) = parser::parse_source(&mut p, &source) {
                    tree_map.insert(info.path.clone(), tree);
                }
            }
        }
    }

    let now = Utc::now();
    let timestamp = config.force_timestamp.clone().unwrap_or_else(|| Local::now().format("%Y-%m-%d_%H-%M-%S").to_string());
    let abs_input = abs_project_root.clone();

    let mut final_sources = source_map.clone();
    let mut target_tokens = None;
    let mut target_achieved = None;

    if let Some(target) = &config.target {
        let planner = crate::batch::distillation::target_planner::TargetPlanner::new(
            vec![
                Box::new(crate::batch::distillation::transforms::comments_ast::CommentRemoval),
                Box::new(crate::batch::distillation::transforms::debug_remover::DebugRemover),
                Box::new(crate::batch::distillation::transforms::literals::LiteralShrinker::new(100)),
                Box::new(crate::batch::distillation::transforms::skeleton::Skeletonizer::new(
                    crate::batch::distillation::transforms::skeleton::SkeletonMode::TypesAndSignatures,
                )),
                Box::new(crate::batch::distillation::transforms::skeleton::Skeletonizer::new(
                    crate::batch::distillation::transforms::skeleton::SkeletonMode::SignaturesOnly,
                )),
            ],
            tokenizer.cloned(),
        );
        let target_val = match target {
            Target::Bytes(b) => *b / 4,
            Target::Percent(p) => (total_before_tokens as f64 * (1.0 - p / 100.0)) as usize,
        };
        let result = planner.achieve_target(
            file_infos,
            graph,
            target_val,
            &source_map,
            &raw_paths,
            &focus_paths,
            None, // No PB
        );
        target_tokens = Some(result.target_tokens);
        target_achieved = Some(result.achieved);
        final_sources = result.sources;
    } else {
        // Pre-allocate strategies once outside the file loop
        let s_comment_removal = crate::batch::distillation::transforms::comments_ast::CommentRemoval;
        let s_debug_remover = crate::batch::distillation::transforms::debug_remover::DebugRemover;
        let s_literal_shrinker = crate::batch::distillation::transforms::literals::LiteralShrinker::new(100);
        let s_import_simplifier = crate::batch::distillation::transforms::import_simplifier::ImportSimplifier;
        let s_skeleton_types = crate::batch::distillation::transforms::skeleton::Skeletonizer::new(
            crate::batch::distillation::transforms::skeleton::SkeletonMode::TypesAndSignatures,
        );
        let s_skeleton_sigs = crate::batch::distillation::transforms::skeleton::Skeletonizer::new(
            crate::batch::distillation::transforms::skeleton::SkeletonMode::SignaturesOnly,
        );
        let s_symbol_pruner = crate::batch::distillation::transforms::symbol_pruner::SymbolPruner::new(graph);

        for file_info in file_infos.iter() {
            if raw_paths.contains(&file_info.path) { continue; }
            let is_focused = focus_paths.contains(&file_info.path);
            let importance = scoring::calculate_importance(file_info, graph, is_focused);
            let mut level = compression::CompressionLevel::from_score(importance.total());
            if is_focused {
                level = std::cmp::max(level, compression::CompressionLevel::StripComments);
            }

            if level == compression::CompressionLevel::DropEntirely {
                final_sources.insert(file_info.path.clone(), String::new());
                continue;
            }

            let Some(source) = source_map.get(&file_info.path) else { continue; };
            let Some(tree) = tree_map.get(&file_info.path) else { continue; };
            let lang = &file_info.language;

            let mut edits: Vec<boil_engine::adapters::input::syntax::parser::edits::Edit> = Vec::new();
            let mut strategies: Vec<&dyn crate::batch::distillation::transforms::CompressionStrategy> = Vec::new();

            if level <= compression::CompressionLevel::StripComments {
                strategies.push(&s_comment_removal);
                strategies.push(&s_debug_remover);
                strategies.push(&s_literal_shrinker);
                strategies.push(&s_import_simplifier);
            }
            if level == compression::CompressionLevel::Skeletonize {
                strategies.push(&s_skeleton_types);
            }
            if level <= compression::CompressionLevel::KeepSignatures {
                strategies.push(&s_skeleton_sigs);
                strategies.push(&s_symbol_pruner);
            }

            for strategy in strategies {
                if strategy.is_safe_for_lang(lang) {
                    edits.extend(strategy.get_edits(source, tree, lang, file_info));
                }
            }
            final_sources.insert(file_info.path.clone(), parser::apply_edits(source, edits));
        }
    }

    let boil_root = distillery_root.join(format!("dstl_{}", timestamp));
    let mut total_after_tokens = 0;

    for info in file_infos.iter() {
        let compressed = final_sources.get(&info.path).unwrap();
        total_after_tokens += count_tokens(tokenizer, compressed);

        let out = build_output_path(&info.path, &abs_project_root, &boil_root, true);
        if let Some(parent) = out.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(&out, compressed)?;
    }

    let raw_compression = if total_before_tokens > 0 {
        (1.0 - (total_after_tokens as f64 / total_before_tokens as f64)) * 100.0
    } else {
        0.0
    };

    let manifest = Manifest {
        source: abs_input.to_string_lossy().to_string(),
        canon: canon_path.to_string_lossy().to_string(),
        embedding_provider: config.embedding_provider.clone(),
        created: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        original_tokens: total_before_tokens,
        final_tokens: total_after_tokens,
        fidelity: None,
        compression_mode: config.target.as_ref().map(|t| match t { Target::Bytes(_) => "bytes", Target::Percent(_) => "percent" }.to_string()),
        compression_target: config.target.as_ref().map(|t| match t { Target::Bytes(b) => *b as f64, Target::Percent(p) => *p }),
        target_tokens,
        target_achieved,
        actual_compression: (raw_compression * 100.0).round() / 100.0,
        focused_files: focused_files.clone(),
        user_ignored_files: user_ignored_files.clone(),
        system_ignored_files: system_ignored_files.clone(),
        layers: Vec::new(),
    };

    std::fs::write(boil_root.join("dstl_manifest.toml"), toml::to_string_pretty(&manifest)?)?;
    std::fs::write(boil_root.join("index.json"), serde_json::to_string_pretty(file_infos)?)?;

    if !config.silent {
        println!("Distilled to {}", boil_root.display());
    }

    Ok(())
}

fn build_output_path(abs_file: &Path, abs_base: &Path, out_root: &Path, distilled: bool) -> PathBuf {
    let abs_file_canonical = std::fs::canonicalize(abs_file).unwrap_or_else(|_| abs_file.to_path_buf());
    let abs_base_canonical = std::fs::canonicalize(abs_base).unwrap_or_else(|_| abs_base.to_path_buf());

    let base_name = abs_base_canonical.components().next_back()
        .and_then(|c| match c { std::path::Component::Normal(n) => Some(n.to_string_lossy().to_string()), _ => None })
        .unwrap_or_else(|| "input".to_string());

    let relative = abs_file_canonical.strip_prefix(&abs_base_canonical).unwrap_or(&abs_file_canonical);
    
    // Safety check: if the relative path still starts with the project base_name, strip it to prevent double nesting
    let mut clean_relative = relative;
    if relative.starts_with(&base_name) {
        if let Ok(stripped) = relative.strip_prefix(&base_name) {
            clean_relative = stripped;
        }
    }
    
    // Also strip a leading slash if it became an absolute path that didn't match the prefix
    if clean_relative.is_absolute() {
        if let Ok(stripped) = clean_relative.strip_prefix("/") {
            clean_relative = stripped;
        }
    }

    let mut out_path = if abs_base_canonical.is_dir() {
        out_root.join(&base_name).join(clean_relative)
    } else {
        out_root.join(abs_file_canonical.file_name().unwrap_or(abs_file_canonical.as_os_str()))
    };

    if distilled {
        let file_name = out_path.file_name().unwrap().to_string_lossy();
        out_path.set_file_name(format!("{}.dstl", file_name));
    }
    out_path
}

pub fn patch_batch_layers(
    batch: &crate::batch::Batch,
    changed_path: &Path,
    canon_state: &boil_core::canon::CanonState,
) -> Result<()> {
    use petgraph::visit::EdgeRef;

    let project_root = &batch.source_root;
    let tokenizer = get_shared_tokenizer();

    let abs_project_root = std::fs::canonicalize(project_root).unwrap_or_else(|_| project_root.to_path_buf());
    let abs_changed_path = std::fs::canonicalize(changed_path).unwrap_or_else(|_| changed_path.to_path_buf());

    // 1. Find affected files (changed path + 1-hop neighbors)
    let mut affected_paths = HashSet::new();
    affected_paths.insert(abs_changed_path.clone());

    let file_id = format!("file:{}", abs_changed_path.display());
    if let Some(&file_idx) = canon_state.graph.node_index.get(&file_id) {
        // Direct Imports (incoming or outgoing)
        for &dir in &[petgraph::Direction::Incoming, petgraph::Direction::Outgoing] {
            for edge in canon_state.graph.graph.edges_directed(file_idx, dir) {
                if let boil_core::canon::graph::EdgeData::Imports = edge.weight() {
                    let neighbor_idx = if edge.source() == file_idx { edge.target() } else { edge.source() };
                    if let boil_core::canon::graph::NodeData::File { path, .. } = &canon_state.graph.graph[neighbor_idx] {
                        affected_paths.insert(path.clone());
                    }
                }
            }
        }

        // Collect symbols in this file
        let mut symbols = Vec::new();
        for edge in canon_state.graph.graph.edges_directed(file_idx, petgraph::Direction::Outgoing) {
            if let boil_core::canon::graph::EdgeData::Contains = edge.weight() {
                let child_idx = edge.target();
                if let boil_core::canon::graph::NodeData::Symbol { name, .. } = &canon_state.graph.graph[child_idx] {
                    symbols.push((child_idx, name.clone()));
                }
            }
        }

        // Calls/References from/to symbols in this file
        for (sym_idx, _name) in symbols {
            for &dir in &[petgraph::Direction::Incoming, petgraph::Direction::Outgoing] {
                for edge in canon_state.graph.graph.edges_directed(sym_idx, dir) {
                    match edge.weight() {
                        boil_core::canon::graph::EdgeData::Calls | boil_core::canon::graph::EdgeData::References => {
                            let other_idx = if edge.source() == sym_idx { edge.target() } else { edge.source() };
                            if let boil_core::canon::graph::NodeData::Symbol { .. } = &canon_state.graph.graph[other_idx] {
                                // Find parent file of other_idx
                                for parent_edge in canon_state.graph.graph.edges_directed(other_idx, petgraph::Direction::Incoming) {
                                    if let boil_core::canon::graph::EdgeData::Contains = parent_edge.weight() {
                                        if let boil_core::canon::graph::NodeData::File { path, .. } = &canon_state.graph.graph[parent_edge.source()] {
                                            affected_paths.insert(path.clone());
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Pre-read, tokenize, and parse affected files once to avoid repeating per-layer
    let mut parsed_affected = HashMap::new();
    for path in &affected_paths {
        if let Some(file_info) = canon_state.file_infos.iter().find(|info| &info.path == path) {
            if let Ok(source) = std::fs::read_to_string(path) {
                let original_tokens = count_tokens(tokenizer, &source);
                let mut tree = None;
                if let Some(mut p) = parser::create_parser(&file_info.language) {
                    if let Ok(t) = parser::parse_source(&mut p, &source) {
                        tree = Some(t);
                    }
                }
                parsed_affected.insert(path.clone(), (source, original_tokens, tree));
            }
        }
    }

    // Load batch root index.json once to retrieve old original_tokens counts
    let batch_index_path = batch.root.join("index.json");
    let batch_file_infos: Vec<FileInfo> = if batch_index_path.exists() {
        let s = std::fs::read_to_string(&batch_index_path)?;
        serde_json::from_str(&s).unwrap_or_default()
    } else {
        Vec::new()
    };

    // Pre-allocate strategies once outside the layer loop
    let s_comment_removal = crate::batch::distillation::transforms::comments_ast::CommentRemoval;
    let s_debug_remover = crate::batch::distillation::transforms::debug_remover::DebugRemover;
    let s_literal_shrinker = crate::batch::distillation::transforms::literals::LiteralShrinker::new(100);
    let s_import_simplifier = crate::batch::distillation::transforms::import_simplifier::ImportSimplifier;
    let s_skeleton_arch = crate::batch::distillation::transforms::skeleton::Skeletonizer::new(
        crate::batch::distillation::transforms::skeleton::SkeletonMode::ArchitecturalOnly,
    );
    let s_skeleton_types = crate::batch::distillation::transforms::skeleton::Skeletonizer::new(
        crate::batch::distillation::transforms::skeleton::SkeletonMode::TypesAndSignatures,
    );

    // 2. Re-distill affected files for each layer
    for fidelity in crate::batch::fidelity::Fidelity::all() {
        let layer_name = fidelity.label();
        let layer_root = batch.root.join("layers").join(layer_name);
        let manifest_path = layer_root.join("dstl_manifest.toml");

        // Load layer manifest if it exists
        if !manifest_path.exists() {
            continue;
        }
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let mut manifest: Manifest = toml::from_str(&manifest_content)?;

        let mut token_diff: isize = 0;
        let mut original_token_diff: isize = 0;

        for path in &affected_paths {
            let file_info = canon_state.file_infos.iter().find(|info| &info.path == path);
            let out_path = build_output_path(path, &abs_project_root, &layer_root, true);

            // Read old distilled token count if file existed
            let mut old_distilled_tokens = 0;
            if out_path.exists() {
                if let Ok(old_content) = std::fs::read_to_string(&out_path) {
                    old_distilled_tokens = count_tokens(tokenizer, &old_content);
                }
            }

            // Retrieve old original token count from the preloaded batch_file_infos
            let mut old_original_tokens = 0;
            if let Some(info) = batch_file_infos.iter().find(|info| &info.path == path) {
                old_original_tokens = info.original_tokens;
            }

            if let Some(file_info) = file_info {
                if let Some((source, new_original_tokens, tree_opt)) = parsed_affected.get(path) {
                    original_token_diff += (*new_original_tokens as isize) - (old_original_tokens as isize);

                    let mut edits = Vec::new();
                    if let Some(tree) = tree_opt {
                        let mut strategies: Vec<&dyn crate::batch::distillation::transforms::CompressionStrategy> = Vec::new();
                        match fidelity {
                            Fidelity::L0 => {
                                strategies.push(&s_comment_removal);
                                strategies.push(&s_debug_remover);
                                strategies.push(&s_literal_shrinker);
                            }
                            Fidelity::L1 => {
                                strategies.push(&s_comment_removal);
                                strategies.push(&s_debug_remover);
                                strategies.push(&s_literal_shrinker);
                                strategies.push(&s_skeleton_types);
                            }
                            Fidelity::L2 => {
                                strategies.push(&s_comment_removal);
                                strategies.push(&s_debug_remover);
                                strategies.push(&s_literal_shrinker);
                                strategies.push(&s_import_simplifier);
                                strategies.push(&s_skeleton_arch);
                            }
                        }

                        for strategy in strategies {
                            if strategy.is_safe_for_lang(&file_info.language) {
                                edits.extend(strategy.get_edits(source, tree, &file_info.language, file_info));
                            }
                        }
                    }

                    let compressed = parser::apply_edits(source, edits);
                    let new_distilled_tokens = count_tokens(tokenizer, &compressed);
                    token_diff += (new_distilled_tokens as isize) - (old_distilled_tokens as isize);

                    if let Some(parent) = out_path.parent() { std::fs::create_dir_all(parent)?; }
                    std::fs::write(&out_path, compressed)?;
                }
            } else {
                // File was deleted
                original_token_diff -= old_original_tokens as isize;
                token_diff -= old_distilled_tokens as isize;
                if out_path.exists() {
                    let _ = std::fs::remove_file(&out_path);
                }
            }
        }

        // Update manifest
        let new_orig = (manifest.original_tokens as isize + original_token_diff).max(0) as usize;
        let new_final = (manifest.final_tokens as isize + token_diff).max(0) as usize;

        manifest.original_tokens = new_orig;
        manifest.final_tokens = new_final;
        manifest.actual_compression = if new_orig > 0 {
            let compression_ratio = (1.0 - (new_final as f64 / new_orig as f64)) * 100.0;
            (compression_ratio * 100.0).round() / 100.0
        } else {
            0.0
        };

        std::fs::write(&manifest_path, toml::to_string_pretty(&manifest)?)?;

        // Write index.json in the layer directory
        let index_path = layer_root.join("index.json");
        std::fs::write(&index_path, serde_json::to_string_pretty(&canon_state.file_infos)?)?;
    }

    // Write index.json in the batch root
    std::fs::write(batch.root.join("index.json"), serde_json::to_string_pretty(&canon_state.file_infos)?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::batch::distillation::transforms::{
        debug_remover::DebugRemover,
        import_simplifier::ImportSimplifier,
        skeleton::{Skeletonizer, SkeletonMode},
        strategy::CompressionStrategy,
    };
    use boil_core::language::Language;
    use boil_core::canon::FileInfo;
    use boil_engine::adapters::input::syntax::parser;
    use std::path::PathBuf;

    #[test]
    fn test_new_languages_distillation() {
        let get_strategy_edits = |strategy: &dyn CompressionStrategy, source: &str, lang: Language, info: &FileInfo| -> String {
            let mut p = parser::create_parser(&lang).expect("Failed to create parser");
            let tree = parser::parse_source(&mut p, source).expect("Failed to parse source");
            let edits = strategy.get_edits(source, &tree, &lang, info);
            parser::apply_edits(source, edits)
        };

        let empty_info = |lang: Language| FileInfo {
            path: PathBuf::from("test"),
            language: lang,
            symbols: vec![],
            imports: vec![],
            references: vec![],
            original_tokens: 0,
        };

        // 1. C# (CSharp) Tests
        {
            let source = "class Program {\n    void Main() {\n        Console.WriteLine(\"Debug log\");\n    }\n}";
            
            // Test Skeletonizer (ArchitecturalOnly)
            let s_arch = Skeletonizer::new(SkeletonMode::ArchitecturalOnly);
            let res = get_strategy_edits(&s_arch, source, Language::CSharp, &empty_info(Language::CSharp));
            assert!(res.contains("class Program"));
            assert!(!res.contains("Console.WriteLine"));

            // Test DebugRemover
            let debug_rem = DebugRemover;
            let res = get_strategy_edits(&debug_rem, source, Language::CSharp, &empty_info(Language::CSharp));
            assert!(!res.contains("Console.WriteLine"));
        }

        // 2. Kotlin Tests
        {
            let source = "import foo.bar.UsedClass\nimport foo.bar.UnusedClass\nfun test(param: String): Int {\n    println(\"debug log\")\n    return 0\n}";
            
            // Test ImportSimplifier
            let info = FileInfo {
                path: PathBuf::from("test.kt"),
                language: Language::Kotlin,
                symbols: vec![],
                imports: vec![],
                references: vec![
                    boil_core::canon::Reference {
                        name: "UsedClass".to_string(),
                        byte_offset: 0,
                        kind: boil_core::canon::ReferenceKind::Type,
                    }
                ],
                original_tokens: 0,
            };
            let imp_simp = ImportSimplifier;
            let res = get_strategy_edits(&imp_simp, source, Language::Kotlin, &info);
            assert!(res.contains("import foo.bar.UsedClass"));
            assert!(res.contains("// Unused import foo.bar.UnusedClass pruned"));

            // Test Skeletonizer (TypesAndSignatures)
            let s_types = Skeletonizer::new(SkeletonMode::TypesAndSignatures);
            let res = get_strategy_edits(&s_types, source, Language::Kotlin, &info);
            assert!(res.contains("fun test(param: String): Int") && res.contains("{ ... }"));
        }

        // 3. Swift Tests
        {
            let source = "func hello() {\n    print(\"debug log\")\n}";
            let res = get_strategy_edits(&DebugRemover, source, Language::Swift, &empty_info(Language::Swift));
            assert!(!res.contains("print(\"debug log\")"));
        }

        // 4. Ruby Tests
        {
            let source = "def my_method\n    puts \"debug log\"\nend";
            
            // Test DebugRemover
            let res = get_strategy_edits(&DebugRemover, source, Language::Ruby, &empty_info(Language::Ruby));
            assert!(!res.contains("puts"));

            // Test Skeletonizer (TypesAndSignatures)
            let s_types = Skeletonizer::new(SkeletonMode::TypesAndSignatures);
            let res = get_strategy_edits(&s_types, source, Language::Ruby, &empty_info(Language::Ruby));
            assert!(res.contains("def my_method") && res.contains("# ...") && res.contains("end"));
        }

        // 5. Test ImportSimplifier multi-import grouping (Rust)
        {
            let source = "use std::path::{Path, PathBuf};\nfn main() { let x: PathBuf = PathBuf::new(); }";
            let info = FileInfo {
                path: PathBuf::from("main.rs"),
                language: Language::Rust,
                symbols: vec![],
                imports: vec![],
                references: vec![
                    boil_core::canon::Reference {
                        name: "PathBuf".to_string(),
                        byte_offset: 0,
                        kind: boil_core::canon::ReferenceKind::Type,
                    }
                ],
                original_tokens: 0,
            };
            let imp_simp = ImportSimplifier;
            let res = get_strategy_edits(&imp_simp, source, Language::Rust, &info);
            // Path is unused but PathBuf is used, so the statement MUST NOT be pruned!
            assert!(res.contains("use std::path::{Path, PathBuf};"));
            assert!(!res.contains("pruned"));
        }

        // 6. Test has_overlap fix
        {
            use crate::batch::distillation::transforms::strategy::has_overlap;
            use boil_engine::adapters::input::syntax::parser::edits::Edit;
            let edits = vec![Edit { start: 10, end: 25, replacement: "".to_string() }];
            // Check partial overlap [20, 30] which overlaps with [10, 25]
            assert!(has_overlap(&edits, 20, 30));
        }
    }
}
