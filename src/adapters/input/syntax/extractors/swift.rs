use crate::adapters::input::syntax::extractors::SymbolExtractor;
use crate::core::canon::{FileInfo, Reference, Symbol, SymbolKind};
use crate::core::language::Language;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Tree;

pub struct SwiftExtractor;

impl SymbolExtractor for SwiftExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        let import_query_str = "(import_declaration) @import";
        static IMPORT_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let import_query = IMPORT_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_swift::language(), import_query_str).unwrap()
        });
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let import_matches = cursor.matches(import_query, tree.root_node(), source.as_bytes());
            for m in import_matches {
                for cap in m.captures {
                    let raw_import = source[cap.node.start_byte()..cap.node.end_byte()].to_string();
                    let module_name = raw_import.replace("import ", "").trim().to_string();
                    imports.push(crate::core::canon::Import {
                        module: module_name,
                    });
                }
            }
        }

        let symbol_query_str = "
            (class_declaration type: (type_identifier) @name) @class
            (struct_declaration type: (type_identifier) @name) @struct
            (protocol_declaration type: (type_identifier) @name) @interface
            (function_declaration name: (simple_identifier) @name) @fn
        ";

        static SYMBOL_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let query = SYMBOL_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_swift::language(), symbol_query_str).unwrap()
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
                        "class" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Class;
                        }
                        "struct" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Struct;
                        }
                        "interface" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Interface;
                        }
                        "fn" => {
                            node = Some(capture.node);
                            kind = SymbolKind::Function;
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
        }

        let mut references = Vec::new();
        let ref_query_str = "
            (call_expression (simple_identifier) @ref.call)
            (simple_identifier) @ref.var
        ";

        static REF_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let ref_query = REF_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_swift::language(), ref_query_str).unwrap()
        });
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let ref_matches = cursor.matches(ref_query, tree.root_node(), source.as_bytes());
            for m in ref_matches {
                for cap in m.captures {
                    let capture_name = ref_query.capture_names()[cap.index as usize].as_str();
                    let kind = match capture_name {
                        "ref.call" => crate::core::canon::ReferenceKind::Call,
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
            language: Language::Swift,
            symbols,
            imports,
            references,
            original_tokens: token_count,
        }
    }
}
