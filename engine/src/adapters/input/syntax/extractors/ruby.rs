use crate::adapters::input::syntax::extractors::SymbolExtractor;
use boil_core::canon::{FileInfo, Reference, Symbol, SymbolKind};
use boil_core::language::Language;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Tree;

pub struct RubyExtractor;

impl SymbolExtractor for RubyExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        let import_query_str = "(call method: (identifier) @method arguments: (argument_list (string (string_content) @import)))";
        static IMPORT_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let import_query = IMPORT_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_ruby::language(), import_query_str).unwrap()
        });
        if true {
            let mut cursor = tree_sitter::QueryCursor::new();
            let import_matches = cursor.matches(import_query, tree.root_node(), source.as_bytes());
            for m in import_matches {
                let mut method = "";
                let mut import = "";
                for cap in m.captures {
                    let capture_name = import_query.capture_names()[cap.index as usize].as_str();
                    let text = &source[cap.node.start_byte()..cap.node.end_byte()];
                    if capture_name == "method" {
                        method = text;
                    }
                    if capture_name == "import" {
                        import = text;
                    }
                }
                if method == "require" || method == "require_relative" {
                    imports.push(boil_core::canon::Import {
                        module: import.to_string(),
                    });
                }
            }
        }

        let symbol_query_str = "
            (class name: (constant) @name) @class
            (module name: (constant) @name) @module
            (method name: (identifier) @name) @fn
        ";

        static SYMBOL_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let query = SYMBOL_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_ruby::language(), symbol_query_str).unwrap()
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
                        "module" => {
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
            (call method: (identifier) @ref.call)
            (identifier) @ref.var
            (constant) @ref.type
        ";
        static REF_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let ref_query = REF_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_ruby::language(), ref_query_str).unwrap()
        });
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
            language: Language::Ruby,
            symbols,
            imports,
            references,
            original_tokens: token_count,
        }
    }
}
