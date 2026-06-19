use anyhow::Result;
use petgraph::Direction;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use boil_core::canon::graph::{EdgeData, NodeData, ProjectGraph};
use boil_core::canon::state::CanonStateRef;
use boil_core::canon::{CanonState, FileInfo, ReferenceKind};
use crate::core::engine::EngineConfig;
use crate::ports::{ReasoningModule, SyntaxProvider};

/// Surgically patches `canon.bin` after a single source file has been edited on disk.
///
/// # Steps
/// 1. Re-parse the changed file using the provided syntax provider.
/// 2. Remove all stale nodes/edges for the changed file from the graph.
/// 3. Re-insert the new file node, symbol nodes, and Contains edges.
/// 4. Re-resolve outgoing Imports/Calls/References edges for the changed file.
/// 5. Rescan every other file that referenced any symbol from the changed file and
///    rebuild their outgoing edges so that reference counts stay globally accurate.
/// 6. Re-run all reasoning modules (semantics, clustering) on the full updated graph.
/// 7. Serialize the updated state back to `canon_bin_path`.
pub fn patch_file(
    state: &mut CanonState,
    canon_bin_path: &Path,
    changed_file: &Path,
    syntax_provider: &dyn SyntaxProvider,
    reasoning_modules: &[Box<dyn ReasoningModule>],
    config: &EngineConfig,
) -> Result<()> {
    // -------------------------------------------------------------------------
    // 1. Re-parse the changed file
    // -------------------------------------------------------------------------
    let source = std::fs::read_to_string(changed_file)?;
    let new_file_info = syntax_provider.parse_file(changed_file, &source)?;

    let file_id = format!("file:{}", changed_file.display());

    // Collect old + new symbol names so we know which referencing files to rescan.
    let old_symbol_names: HashSet<String> = state
        .file_infos
        .iter()
        .find(|fi| fi.path == changed_file)
        .map(|fi| fi.symbols.iter().map(|s| s.name.clone()).collect())
        .unwrap_or_default();

    let new_symbol_names: HashSet<String> = new_file_info
        .symbols
        .iter()
        .map(|s| s.name.clone())
        .collect();

    let affected_symbol_names: HashSet<String> =
        old_symbol_names.union(&new_symbol_names).cloned().collect();

    // -------------------------------------------------------------------------
    // 2. Remove stale graph nodes for the changed file
    // -------------------------------------------------------------------------
    if let Some(&file_node) = state.graph.node_index.get(&file_id) {
        // Collect child symbol nodes reachable via Contains edges.
        let symbol_nodes: Vec<_> = state
            .graph
            .graph
            .edges_directed(file_node, Direction::Outgoing)
            .filter(|e| matches!(state.graph.graph[e.id()], EdgeData::Contains))
            .map(|e| e.target())
            .collect();

        // Remove symbol node_index entries.
        let symbol_node_set: HashSet<_> = symbol_nodes.iter().cloned().collect();
        state
            .graph
            .node_index
            .retain(|_, v| !symbol_node_set.contains(v));

        for sym_node in &symbol_nodes {
            state.graph.reverse_index.remove(sym_node);
        }

        // Remove symbol nodes (StableGraph removes their incident edges automatically).
        for sym_node in symbol_nodes {
            state.graph.graph.remove_node(sym_node);
        }

        // Remove the file node and its index entry.
        state.graph.graph.remove_node(file_node);
        state.graph.node_index.remove(&file_id);
        state.graph.reverse_index.remove(&file_node);
    }

    let old_file_info = state
        .file_infos
        .iter()
        .find(|fi| fi.path == changed_file)
        .cloned();

    // -------------------------------------------------------------------------
    // 3. Remove old FileInfo entry
    // -------------------------------------------------------------------------
    state.file_infos.retain(|fi| fi.path != changed_file);

    // -------------------------------------------------------------------------
    // 4. Re-insert file node + symbol nodes + Contains edges
    // -------------------------------------------------------------------------
    let repo_node = *state
        .graph
        .node_index
        .get("repo:root")
        .ok_or_else(|| anyhow::anyhow!("Canon is corrupt: repo:root node missing"))?;

    let file_node = state.graph.graph.add_node(NodeData::File {
        path: new_file_info.path.clone(),
        language: format!("{:?}", new_file_info.language),
        metadata: HashMap::new(),
    });
    state.graph.node_index.insert(file_id.clone(), file_node);
    state.graph.reverse_index.insert(file_node, file_id.clone());
    state
        .graph
        .graph
        .add_edge(repo_node, file_node, EdgeData::Contains);

    for symbol in &new_file_info.symbols {
        let symbol_id = format!("symbol:{}:{}", new_file_info.path.display(), symbol.name);
        let sym_node = state.graph.graph.add_node(NodeData::Symbol {
            name: symbol.name.clone(),
            kind: symbol.kind.clone(),
            exported: symbol.exported,
            references: 0, // recomputed below
            metadata: HashMap::new(),
        });
        state.graph.node_index.insert(symbol_id.clone(), sym_node);
        state.graph.reverse_index.insert(sym_node, symbol_id);
        state
            .graph
            .graph
            .add_edge(file_node, sym_node, EdgeData::Contains);
    }

    // -------------------------------------------------------------------------
    // 5. Build helper maps and resolve outgoing edges for the changed file
    // -------------------------------------------------------------------------
    let global_sym_map = build_global_symbol_map(&state.graph);
    let file_stem_map = build_file_stem_map(&state.graph);

    resolve_file_edges(
        &mut state.graph,
        file_node,
        &new_file_info,
        &file_stem_map,
        &global_sym_map,
    );

    // -------------------------------------------------------------------------
    // 6. Push new FileInfo and apply delta reference counting
    // -------------------------------------------------------------------------
    state.file_infos.push(new_file_info.clone());

    let mut delta_counts: HashMap<String, isize> = HashMap::new();
    if let Some(old_fi) = &old_file_info {
        for r in &old_fi.references {
            *delta_counts.entry(r.name.clone()).or_insert(0) -= 1;
        }
    }
    for r in &new_file_info.references {
        *delta_counts.entry(r.name.clone()).or_insert(0) += 1;
    }

    for (name, delta) in delta_counts {
        if delta == 0 {
            continue;
        }
        if let Some(targets) = global_sym_map.get(&name) {
            for &(_, sym_node) in targets {
                if let NodeData::Symbol { references, .. } = &mut state.graph.graph[sym_node] {
                    *references = references.saturating_add_signed(delta);
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // 7. Rescan all files that referenced symbols from the changed file
    // -------------------------------------------------------------------------
    // Collect referencing file infos first (to avoid borrow conflicts).
    let referencing: Vec<FileInfo> = state
        .file_infos
        .iter()
        .filter(|fi| fi.path != changed_file)
        .filter(|fi| {
            fi.references
                .iter()
                .any(|r| affected_symbol_names.contains(&r.name))
        })
        .cloned()
        .collect();

    for ref_fi in &referencing {
        let ref_file_id = format!("file:{}", ref_fi.path.display());
        if let Some(&ref_file_node) = state.graph.node_index.get(&ref_file_id) {
            // Remove all outgoing Calls/References/Imports edges from the file node.
            remove_outgoing_call_edges(&mut state.graph, ref_file_node);

            // Remove outgoing Calls/References from the file's symbol nodes too.
            let sym_nodes: Vec<_> = state
                .graph
                .graph
                .edges_directed(ref_file_node, Direction::Outgoing)
                .filter(|e| matches!(state.graph.graph[e.id()], EdgeData::Contains))
                .map(|e| e.target())
                .collect();
            for sym_node in sym_nodes {
                remove_outgoing_call_edges(&mut state.graph, sym_node);
            }

            // Rebuild with the global symbol map (the changed file's new symbols are already in it).
            resolve_file_edges(
                &mut state.graph,
                ref_file_node,
                ref_fi,
                &file_stem_map,
                &global_sym_map,
            );
        }
    }

    // -------------------------------------------------------------------------
    // 8. Re-run reasoning modules (semantics + clustering)
    // -------------------------------------------------------------------------
    for module in reasoning_modules {
        module.process(config, &mut state.file_infos, &mut state.graph)?;
    }

    // -------------------------------------------------------------------------
    // 9. Serialize updated state back to canon.bin
    // -------------------------------------------------------------------------
    let state_ref = CanonStateRef::new(&state.file_infos, &state.graph);
    state_ref.save(canon_bin_path)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds a map from symbol name → Vec<(file_node, symbol_node)> for all
/// symbol nodes currently in the graph.
fn build_global_symbol_map(
    graph: &ProjectGraph,
) -> HashMap<
    String,
    Vec<(
        petgraph::stable_graph::NodeIndex,
        petgraph::stable_graph::NodeIndex,
    )>,
> {
    let mut map: HashMap<String, Vec<_>> = HashMap::new();

    for node_idx in graph.graph.node_indices() {
        if let NodeData::Symbol { name, .. } = &graph.graph[node_idx] {
            // Find the parent file node via the incoming Contains edge.
            if let Some(file_node) = graph
                .graph
                .edges_directed(node_idx, Direction::Incoming)
                .find(|e| matches!(graph.graph[e.id()], EdgeData::Contains))
                .map(|e| e.source())
            {
                map.entry(name.clone())
                    .or_default()
                    .push((file_node, node_idx));
            }
        }
    }

    map
}

/// Builds a map from a file's stem name → its NodeIndex, for import resolution.
fn build_file_stem_map(graph: &ProjectGraph) -> HashMap<String, petgraph::stable_graph::NodeIndex> {
    graph
        .node_index
        .iter()
        .filter(|(k, _)| k.starts_with("file:"))
        .map(|(k, &v)| {
            let path_str = k.strip_prefix("file:").unwrap();
            let stem = std::path::PathBuf::from(path_str)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (stem, v)
        })
        .collect()
}

/// Removes all outgoing Calls, References, and Imports edges from `node`.
fn remove_outgoing_call_edges(graph: &mut ProjectGraph, node: petgraph::stable_graph::NodeIndex) {
    let to_remove: Vec<_> = graph
        .graph
        .edges_directed(node, Direction::Outgoing)
        .filter(|e| {
            matches!(
                graph.graph[e.id()],
                EdgeData::Calls | EdgeData::References | EdgeData::Imports
            )
        })
        .map(|e| e.id())
        .collect();

    for edge_id in to_remove {
        graph.graph.remove_edge(edge_id);
    }
}

/// Resolves outgoing Imports, Calls, and References edges for a single file.
/// Mirrors the logic in `SyntaxModule::ingest`.
fn resolve_file_edges(
    graph: &mut ProjectGraph,
    file_node: petgraph::stable_graph::NodeIndex,
    file_info: &FileInfo,
    file_stem_map: &HashMap<String, petgraph::stable_graph::NodeIndex>,
    global_sym_map: &HashMap<
        String,
        Vec<(
            petgraph::stable_graph::NodeIndex,
            petgraph::stable_graph::NodeIndex,
        )>,
    >,
) {
    let mut imported_files = HashSet::new();

    // Imports
    for import in &file_info.imports {
        if let Some(&target_node) = file_stem_map.get(&import.module)
            && target_node != file_node
        {
            graph
                .graph
                .add_edge(file_node, target_node, EdgeData::Imports);
            imported_files.insert(target_node);
        }
    }

    // References → Calls / References edges
    for reference in &file_info.references {
        // Find the smallest enclosing symbol to use as the caller.
        let caller_node = file_info
            .symbols
            .iter()
            .filter(|s| {
                reference.byte_offset >= s.byte_start && reference.byte_offset <= s.byte_end
            })
            .min_by_key(|s| s.byte_end - s.byte_start)
            .and_then(|s| {
                let sym_id = format!("symbol:{}:{}", file_info.path.display(), s.name);
                graph.node_index.get(&sym_id).copied()
            });

        let caller = caller_node.unwrap_or(file_node);

        if let Some(targets) = global_sym_map.get(&reference.name) {
            let mut resolved = Vec::new();

            // Priority 1: same file
            for &(f, s) in targets {
                if f == file_node {
                    resolved.push(s);
                }
            }
            // Priority 2: imported files
            if resolved.is_empty() {
                for &(f, s) in targets {
                    if imported_files.contains(&f) {
                        resolved.push(s);
                    }
                }
            }
            // Priority 3: global fallback
            if resolved.is_empty() {
                for &(_, s) in targets {
                    resolved.push(s);
                }
            }

            let edge_data = match reference.kind {
                ReferenceKind::Call => EdgeData::Calls,
                _ => EdgeData::References,
            };

            for target in resolved {
                if caller != target {
                    graph.graph.add_edge(caller, target, edge_data.clone());
                }
            }
        }
    }
}
