use tree_sitter::{QueryCursor, Tree};
use boil_engine::adapters::input::syntax::parser::edits::Edit;
use boil_core::language::Language;
use crate::batch::distillation::transforms::strategy::CompressionStrategy;
use boil_core::canon::FileInfo;

pub struct ImportSimplifier;


impl CompressionStrategy for ImportSimplifier {

    fn destructiveness_score(&self, lang: &Language) -> f64 {
        match lang {
            // ImportSimplifier is very low risk for most languages.
            // However, it is high risk for C and C++, so treat it as a last resort.
            Language::C | Language::Cpp => 0.8,
            _ => 0.05,
        }
    }
    
    fn is_safe_for_lang(&self, lang: &Language) -> bool {
        match lang {
            Language::Rust |
            Language::JavaScript |
            Language::TypeScript |
            Language::Tsx |
            Language::Python |
            Language::Java |
            Language::Go |
            Language::Kotlin => true,
            _ => false,
        }
    }

    fn get_edits(&self, source: &str, tree: &Tree, lang: &Language, info: &FileInfo) -> Vec<Edit> {
        let mut edits = Vec::new();
        
        let (query_str, symbol_capture) = match lang {
            Language::Rust => ("(use_declaration (use_list (scoped_identifier) @name)) @import", "name"),
            Language::JavaScript | Language::TypeScript | Language::Tsx => ("(import_clause (named_imports (import_specifier name: (identifier) @name))) @import", "name"),
            Language::Python => ("(import_from_statement name: (dotted_name) @name) @import", "name"),
            Language::Java => ("(import_declaration (scoped_identifier) @name) @import", "name"),
            Language::Go => ("(import_spec (identifier) @name) @import", "name"),
            Language::Kotlin => ("(import_header (identifier) @name) @import", "name"),
            _ => ("", ""),
        };

        if query_str.is_empty() { return edits; }

        if let Some(query) = crate::batch::distillation::transforms::get_cached_query(tree.language(), query_str) {
            let mut cursor = QueryCursor::new();
            let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

            // Group imports by their parent node range
            let mut import_groups: std::collections::HashMap<(usize, usize), (tree_sitter::Node, Vec<String>)> = std::collections::HashMap::new();

            for m in matches {
                let mut name_node = None;
                let mut import_node = None;

                for cap in m.captures {
                    let capture_name = query.capture_names()[cap.index as usize].as_str();
                    if capture_name == symbol_capture { name_node = Some(cap.node); }
                    if capture_name == "import" { import_node = Some(cap.node); }
                }

                if let Some(i_node) = import_node {
                    let entry = import_groups.entry((i_node.start_byte(), i_node.end_byte()))
                        .or_insert_with(|| (i_node, Vec::new()));
                    
                    if let Some(n_node) = name_node {
                        if let Ok(name_str) = std::str::from_utf8(&source.as_bytes()[n_node.start_byte()..n_node.end_byte()]) {
                            entry.1.push(name_str.to_string());
                        }
                    }
                }
            }

            for (_, (i_node, names)) in import_groups {
                if names.is_empty() {
                    continue;
                }
                // Only prune if ALL symbols in the import are unused
                let all_unused = names.iter().all(|name| {
                    let last_segment = name.split('.').last().unwrap_or(name)
                        .split("::").last().unwrap_or(name);
                    !info.references.iter().any(|r| r.name == last_segment)
                });

                if all_unused {
                    edits.push(Edit {
                        start: i_node.start_byte(),
                        end: i_node.end_byte(),
                        replacement: format!("// Unused import {} pruned\n", names.join(", ")),
                    });
                }
            }
        }
        edits
    }
}
