use crate::adapters::input::syntax::extractors::SymbolExtractor;
use boil_core::canon::{FileInfo, Reference, Symbol, SymbolKind};
use boil_core::language::Language;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Tree;

pub struct GoExtractor;

impl SymbolExtractor for GoExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo {
        let mut symbols = Vec::new();
        let imports = Vec::new();

        // Robust query for Go symbols
        let symbol_query_str = "
            (function_declaration name: (identifier) @name) @fn
            (type_spec name: (type_identifier) @name) @struct
            (var_spec name: (identifier) @name) @var
            (const_spec name: (identifier) @name) @const
        ";

        static SYMBOL_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let query = SYMBOL_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_go::language(), symbol_query_str).unwrap()
        });

        if true {
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
                        "method" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Method;
                        }
                        "struct" => {
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

                if let Some(node) = node {
                    let name = name_node
                        .map(|n| source[n.start_byte()..n.end_byte()].to_string())
                        .unwrap_or_else(|| "anonymous".to_string());

                    symbols.push(Symbol {
                        name,
                        kind,
                        byte_start: node.start_byte(),
                        byte_end: node.end_byte(),
                        line_start: node.start_position().row,
                        line_end: node.end_position().row,
                        exported: true,
                        signature: None,
                        references: 0,
                    });
                }
            }
        }

        // Query for Identifiers (References)
        let mut references = Vec::new();
        let ref_query_str = "(identifier) @ref";
        static REF_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let ref_query = REF_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_go::language(), ref_query_str).unwrap()
        });
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let ref_matches = cursor.matches(ref_query, tree.root_node(), source.as_bytes());
            for m in ref_matches {
                for cap in m.captures {
                    references.push(Reference {
                        name: source[cap.node.start_byte()..cap.node.end_byte()].to_string(),
                        byte_offset: cap.node.start_byte(),
                        kind: boil_core::canon::ReferenceKind::Variable,
                    });
                }
            }
        }

        FileInfo {
            path: path.to_path_buf(),
            language: Language::Go,
            symbols,
            imports,
            references,
            original_tokens: token_count,
        }
    }
}
