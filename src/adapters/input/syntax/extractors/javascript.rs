use crate::adapters::input::syntax::extractors::SymbolExtractor;
use crate::core::canon::{FileInfo, Import, Reference, Symbol, SymbolKind};
use crate::core::language::Language;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::Tree;

pub struct JSExtractor;

impl SymbolExtractor for JSExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo {
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        let symbol_query_str = "(function_declaration name: (identifier) @name) @fn
                                (class_declaration name: (identifier) @name) @class
                                (variable_declarator name: (identifier) @name) @var";

        static SYMBOL_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let query = SYMBOL_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_javascript::language(), symbol_query_str).unwrap()
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
                    "class" => {
                        node = Some(capture.node);
                        kind = SymbolKind::Class;
                    }
                    "var" => {
                        node = Some(capture.node);
                        kind = SymbolKind::Variable;
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

        let import_query_str = "(import_statement source: (string) @source)
                                (call_expression function: (identifier) @req arguments: (arguments (string) @source) (#eq? @req \"require\"))";
        static IMPORT_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let import_query = IMPORT_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_javascript::language(), import_query_str).unwrap()
        });
        let mut cursor = tree_sitter::QueryCursor::new();
        let import_matches = cursor.matches(import_query, tree.root_node(), source.as_bytes());

        for m in import_matches {
            for capture in m.captures {
                let capture_name = import_query.capture_names()[capture.index as usize].as_str();
                if capture_name == "source" {
                    let node = capture.node;
                    let module_name = source[node.start_byte()..node.end_byte()]
                        .trim_matches(|c| c == '\'' || c == '"')
                        .to_string();
                    imports.push(Import {
                        module: module_name,
                    });
                }
            }
        }

        // Query for Calls, Types, and fallback Identifiers (References)
        let mut references = Vec::new();
        let ref_query_str = "(call_expression function: (identifier) @ref.call)
                             (call_expression function: (member_expression property: (property_identifier) @ref.call))
                             (identifier) @ref.var";

        static REF_QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();
        let ref_query = REF_QUERY.get_or_init(|| {
            tree_sitter::Query::new(tree_sitter_javascript::language(), ref_query_str).unwrap()
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
            language: Language::JavaScript,
            symbols,
            imports,
            references,
            original_tokens: token_count,
        }
    }
}
