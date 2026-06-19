use crate::adapters::input::syntax::extractors::SymbolExtractor;
use boil_core::canon::{FileInfo, Import, Reference, Symbol, SymbolKind};
use boil_core::language::Language;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Tree;

pub struct CExtractor;

impl SymbolExtractor for CExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        // Query for Functions, Structs, Variables, and Constants
        let symbol_query_str = "(function_definition declarator: (function_declarator declarator: (identifier) @name)) @fn
                                (struct_specifier name: (type_identifier) @name) @struct
                                (init_declarator declarator: (identifier) @name) @var
                                (enumerator name: (identifier) @name) @const";

        static SYMBOL_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let query = SYMBOL_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_c::language(), symbol_query_str).unwrap()
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

            if let (Some(name_node), Some(node)) = (name_node, node) {
                let name = source[name_node.start_byte()..name_node.end_byte()].to_string();
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

        // Query for Imports
        let import_query_str = "(preproc_include) @import";
        static IMPORT_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let import_query = IMPORT_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_c::language(), import_query_str).unwrap()
        });
        let mut cursor = tree_sitter::QueryCursor::new();
        let import_matches = cursor.matches(import_query, tree.root_node(), source.as_bytes());

        for m in import_matches {
            for capture in m.captures {
                let node = capture.node;
                let raw = source[node.start_byte()..node.end_byte()].to_string();
                imports.push(Import { module: raw });
            }
        }

        // Query for Identifiers (References)
        let mut references = Vec::new();
        let ref_query_str = "(identifier) @ref";
        static REF_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let ref_query = REF_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_c::language(), ref_query_str).unwrap()
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
            language: Language::C,
            symbols,
            imports,
            references,
            original_tokens: token_count,
        }
    }
}
