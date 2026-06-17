use crate::adapters::input::syntax::extractors::SymbolExtractor;
use crate::core::canon::{FileInfo, Import, Reference, Symbol, SymbolKind};
use crate::core::language::Language;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Tree;

pub struct CppExtractor;

impl SymbolExtractor for CppExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        // Robust query for C++ symbols
        let symbol_query_str = "
            (function_definition declarator: (function_declarator declarator: (identifier) @name) ) @fn
            (class_specifier name: (type_identifier) @name) @class
            (struct_specifier name: (type_identifier) @name) @struct
            (init_declarator declarator: (identifier) @name) @var
            (enumerator name: (identifier) @name) @const
        ";

        // Attempt to build the query, fallback to even simpler if it fails
        static SYMBOL_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let query = SYMBOL_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_cpp::language(), symbol_query_str).unwrap()
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
                        "class" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Class;
                        }
                        "struct" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Struct;
                        }
                        "ns" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Module;
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

                    let signature = source[node.start_byte()..node.end_byte()]
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    symbols.push(Symbol {
                        name,
                        kind,
                        byte_start: node.start_byte(),
                        byte_end: node.end_byte(),
                        line_start: node.start_position().row,
                        line_end: node.end_position().row,
                        exported: true,
                        signature: Some(signature),
                        references: 0,
                    });
                }
            }
        }

        // Simple include query
        let import_query_str = "(preproc_include) @import";
        static IMPORT_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let import_query = IMPORT_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_cpp::language(), import_query_str).unwrap()
        });
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let import_matches = cursor.matches(import_query, tree.root_node(), source.as_bytes());

            for m in import_matches {
                for capture in m.captures {
                    let node = capture.node;
                    let raw = source[node.start_byte()..node.end_byte()].to_string();
                    imports.push(Import { module: raw });
                }
            }
        }

        // Query for Identifiers (References)
        let mut references = Vec::new();
        let ref_query_str = "(identifier) @ref";
        static REF_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let ref_query = REF_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_cpp::language(), ref_query_str).unwrap()
        });
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let ref_matches = cursor.matches(ref_query, tree.root_node(), source.as_bytes());
            for m in ref_matches {
                for cap in m.captures {
                    references.push(Reference {
                        name: source[cap.node.start_byte()..cap.node.end_byte()].to_string(),
                        byte_offset: cap.node.start_byte(),
                        kind: crate::core::canon::ReferenceKind::Variable,
                    });
                }
            }
        }

        FileInfo {
            path: path.to_path_buf(),
            language: Language::Cpp,
            symbols,
            imports,
            references,
            original_tokens: token_count,
        }
    }
}
