use crate::adapters::input::syntax::extractors::SymbolExtractor;
use crate::core::canon::{FileInfo, Reference, Symbol, SymbolKind};
use crate::core::language::Language;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Tree;

pub struct RustExtractor;

impl SymbolExtractor for RustExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        let import_query_str = "(use_declaration argument: (_) @import)";
        static IMPORT_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let import_query = IMPORT_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_rust::language(), import_query_str).unwrap()
        });
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let import_matches = cursor.matches(import_query, tree.root_node(), source.as_bytes());
            for m in import_matches {
                for cap in m.captures {
                    let raw_import = source[cap.node.start_byte()..cap.node.end_byte()].to_string();
                    let parts: Vec<&str> = raw_import.split("::").collect();
                    let last_part = *parts.last().unwrap_or(&raw_import.as_str());
                    let module_name = last_part
                        .trim_matches(|c: char| {
                            c == ';' || c == '{' || c == '}' || c == ' ' || c == '\n'
                        })
                        .to_string();

                    imports.push(crate::core::canon::Import {
                        module: module_name,
                    });
                }
            }
        }

        // Query for Functions, Structs, Variables, and Constants
        let symbol_query_str = "(function_item name: (identifier) @name) @fn
                                (struct_item name: (type_identifier) @name) @struct
                                (impl_item type: (type_identifier) @name) @impl
                                (let_declaration pattern: (identifier) @name) @var
                                (const_item name: (identifier) @name) @const";

        static SYMBOL_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let query = SYMBOL_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_rust::language(), symbol_query_str).unwrap()
        });
        let mut cursor = tree_sitter::QueryCursor::new();
        let matches = cursor.matches(query, tree.root_node(), source.as_bytes());

        for m in matches {
            let mut name_node = None;
            let mut node = None;
            let mut kind = SymbolKind::Function;

            for capture in m.captures {
                let capture_name = query.capture_names()[capture.index as usize].as_str();
                match capture_name {
                    "name" => name_node = Some(capture.node),
                    "fn" => {
                        node = Some(capture.node);
                        kind = SymbolKind::Function;
                    }
                    "struct" => {
                        node = Some(capture.node);
                        kind = SymbolKind::Struct;
                    }
                    "impl" => {
                        node = Some(capture.node);
                        kind = SymbolKind::Struct;
                    }
                    "var" => {
                        node = Some(capture.node);
                        kind = SymbolKind::Variable;
                    }
                    "const" => {
                        node = Some(capture.node);
                        kind = SymbolKind::Constant;
                    }
                    _ => {}
                }
            }

            if let (Some(n_node), Some(d_node)) = (name_node, node) {
                let name = source[n_node.start_byte()..n_node.end_byte()].to_string();

                let signature = source[d_node.start_byte()..d_node.end_byte()]
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();

                symbols.push(Symbol {
                    name,
                    kind,
                    byte_start: d_node.start_byte(),
                    byte_end: d_node.end_byte(),
                    line_start: d_node.start_position().row,
                    line_end: d_node.end_position().row,
                    exported: true,
                    signature: Some(signature),
                    references: 0,
                });
            }
        }

        // Query for Calls, Types, and fallback Identifiers (References)
        let mut references = Vec::new();
        let ref_query_str = "(call_expression function: (identifier) @ref.call)
                             (call_expression function: (field_expression field: (field_identifier) @ref.call))
                             (type_identifier) @ref.type
                             (identifier) @ref.var";

        static REF_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let ref_query = REF_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_rust::language(), ref_query_str).unwrap()
        });
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let ref_matches = cursor.matches(ref_query, tree.root_node(), source.as_bytes());
            for m in ref_matches {
                for cap in m.captures {
                    let capture_name = ref_query.capture_names()[cap.index as usize].as_str();
                    let kind = match capture_name {
                        "ref.call" => crate::core::canon::ReferenceKind::Call,
                        "ref.type" => crate::core::canon::ReferenceKind::Type,
                        _ => crate::core::canon::ReferenceKind::Variable,
                    };
                    references.push(Reference {
                        name: source[cap.node.start_byte()..cap.node.end_byte()].to_string(),
                        byte_offset: cap.node.start_byte(),
                        kind,
                    });
                }
            }
        }

        FileInfo {
            path: path.to_path_buf(),
            language: Language::Rust,
            symbols,
            imports,
            references,
            original_tokens: token_count,
        }
    }
}
