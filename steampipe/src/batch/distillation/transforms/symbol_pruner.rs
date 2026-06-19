use tree_sitter::{QueryCursor, Tree};
use boil_engine::adapters::input::syntax::parser::edits::Edit;
use boil_core::language::Language;
use crate::batch::distillation::transforms::strategy::CompressionStrategy;
use boil_core::canon::ProjectGraph;
use boil_core::canon::FileInfo;
use boil_core::canon::graph::NodeData;
use std::collections::HashMap;

pub struct SymbolPruner {
    pub symbol_refs: HashMap<String, usize>,
}

impl SymbolPruner {
    pub fn new(graph: &ProjectGraph) -> Self {
        let mut symbol_refs = HashMap::new();
        for node_idx in graph.graph.node_indices() {
            if let NodeData::Symbol { name, references, .. } = &graph.graph[node_idx] {
                symbol_refs.insert(name.clone(), *references);
            }
        }
        SymbolPruner { symbol_refs }
    }
}

impl CompressionStrategy for SymbolPruner {
    fn destructiveness_score(&self, _lang: &Language) -> f64 { 0.9 }
    fn is_safe_for_lang(&self, _lang: &Language) -> bool { true }

    fn get_edits(&self, source: &str, tree: &Tree, lang: &Language, _info: &FileInfo) -> Vec<Edit> {
        let mut edits = Vec::new();

        let query_str = match lang {
            Language::Rust => "(function_item name: (identifier) @name) @def",
            Language::Python => "(function_definition name: (identifier) @name) @def",
            Language::JavaScript | Language::TypeScript | Language::Tsx => "(function_declaration name: (identifier) @name) @def",
            Language::Java => "(method_declaration name: (identifier) @name) @def",
            Language::Go => "(function_declaration name: (identifier) @name) @def",
            Language::C | Language::Cpp => "(function_definition declarator: (function_declarator declarator: (identifier) @name)) @def",
            Language::CSharp => "(method_declaration name: (identifier) @name) @def",
            Language::Kotlin => "(function_declaration (simple_identifier) @name) @def",
            Language::Swift => "(function_declaration name: (simple_identifier) @name) @def",
            Language::Ruby => "(method name: (identifier) @name) @def",
            _ => "",
        };

        if query_str.is_empty() { return edits; }

        if let Some(query) = crate::batch::distillation::transforms::get_cached_query(tree.language(), query_str) {
            let mut cursor = QueryCursor::new();
            let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

            for m in matches {
                let mut name_node = None;
                let mut def_node = None;

                for cap in m.captures {
                    let capture_name = query.capture_names()[cap.index as usize].as_str();
                    if capture_name == "name" { name_node = Some(cap.node); }
                    if capture_name == "def" { def_node = Some(cap.node); }
                }

                if let (Some(n_node), Some(d_node)) = (name_node, def_node) {
                    let name = &source[n_node.start_byte()..n_node.end_byte()];
                    
                    if let Some(count) = self.symbol_refs.get(name) {
                        if *count <= 1 && !crate::batch::distillation::transforms::has_overlap(&edits, d_node.start_byte(), d_node.end_byte()) {
                            edits.push(Edit {
                                start: d_node.start_byte(),
                                end: d_node.end_byte(),
                                replacement: format!("// Symbol {} pruned\n", name),
                            });
                        }
                    }
                }
            }
        }
        edits
    }
}
