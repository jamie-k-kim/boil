use boil_core::canon::{FileInfo, Reference, Symbol, SymbolKind};
use std::sync::OnceLock;
use tree_sitter::Tree;

use crate::adapters::input::syntax::extractors::SymbolExtractor;
use std::path::Path;

pub struct TSExtractor;

impl SymbolExtractor for TSExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        let import_query_str = "(import_statement source: (string) @source)
                                (call_expression function: (identifier) @req arguments: (arguments (string) @source) (#eq? @req \"require\"))";
        static IMPORT_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let import_query = IMPORT_QUERY
            .get_or_init(|| tree_sitter::Query::new(tree.language(), import_query_str).unwrap());
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let import_matches = cursor.matches(import_query, tree.root_node(), source.as_bytes());
            for m in import_matches {
                for capture in m.captures {
                    let capture_name =
                        import_query.capture_names()[capture.index as usize].as_str();
                    if capture_name == "source" {
                        let node = capture.node;
                        let module_name = source[node.start_byte()..node.end_byte()]
                            .trim_matches(|c| c == '\'' || c == '"')
                            .to_string();
                        imports.push(boil_core::canon::Import {
                            module: module_name,
                        });
                    }
                }
            }
        }

        // Robust query for TS/TSX symbols
        let symbol_query_str = "
            (function_declaration name: (identifier) @name) @fn
            (method_definition name: (property_identifier) @name) @method
            (class_declaration name: (type_identifier) @name) @class
            (interface_declaration name: (type_identifier) @name) @interface
            (variable_declarator name: (identifier) @name) @var
        ";

        static SYMBOL_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let query = SYMBOL_QUERY
            .get_or_init(|| tree_sitter::Query::new(tree.language(), symbol_query_str).unwrap());

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
                        "class" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Class;
                        }
                        "interface" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Interface;
                        }
                        "var" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Variable;
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

        // Query for Calls, Types, and fallback Identifiers (References)
        let mut references = Vec::new();
        let ref_query_str = "(call_expression function: (identifier) @ref.call)
                             (call_expression function: (member_expression property: (property_identifier) @ref.call))
                             (type_identifier) @ref.type
                             (identifier) @ref.var";

        static REF_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let ref_query = REF_QUERY
            .get_or_init(|| tree_sitter::Query::new(tree.language(), ref_query_str).unwrap());
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let ref_matches = cursor.matches(ref_query, tree.root_node(), source.as_bytes());
            for m in ref_matches {
                for cap in m.captures {
                    let capture_name = ref_query.capture_names()[cap.index as usize].as_str();
                    let kind = match capture_name {
                        "ref.call" => boil_core::canon::ReferenceKind::Call,
                        "ref.type" => boil_core::canon::ReferenceKind::Type,
                        _ => boil_core::canon::ReferenceKind::Variable,
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
            language: boil_core::language::Language::TypeScript,
            symbols,
            imports,
            references,
            original_tokens: token_count,
        }
    }
}
