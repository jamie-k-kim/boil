use tree_sitter::{QueryCursor, Tree};
use boil_engine::adapters::input::syntax::parser::edits::Edit;
use boil_core::language::Language;
use crate::batch::distillation::transforms::strategy::CompressionStrategy;
use boil_core::canon::FileInfo;

pub struct DebugRemover;

impl CompressionStrategy for DebugRemover {
    fn destructiveness_score(&self, _lang: &Language) -> f64 { 0.1 }
    fn is_safe_for_lang(&self, _lang: &Language) -> bool { true }

    fn get_edits(&self, source: &str, tree: &Tree, lang: &Language, _info: &FileInfo) -> Vec<Edit> {
        let mut edits = Vec::new();

        let queries = match lang {
            Language::Rust => vec![
                "(macro_invocation) @debug",
            ],
            Language::Python => vec![
                "(call function: (identifier) @debug) @debug",
                "(call function: (attribute object: (identifier) @obj) @debug) @debug",
            ],
            Language::JavaScript | Language::TypeScript | Language::Tsx => vec![
                "(call_expression function: (member_expression) @debug) @debug",
                "(call_expression function: (identifier) @debug) @debug",
            ],
            Language::Java => vec![
                "(method_invocation) @debug",
            ],
            Language::Go => vec![
                "(call_expression) @debug",
            ],
            Language::C | Language::Cpp => vec![
                "(call_expression) @debug",
            ],
            Language::CSharp => vec![
                "(invocation_expression) @debug",
            ],
            Language::Kotlin => vec![
                "(call_expression) @debug",
            ],
            Language::Swift => vec![
                "(call_expression) @debug",
            ],
            Language::Ruby => vec![
                "(call) @debug",
            ],
            _ => vec![],
        };

        for query_str in queries {
            if let Some(query) = crate::batch::distillation::transforms::get_cached_query(tree.language(), query_str) {
                let mut cursor = QueryCursor::new();
                let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
                
                for m in matches {
                    for cap in m.captures {
                        let node = cap.node;
                        let text = &source[node.start_byte()..node.end_byte()];
                        
                        if text.contains("print") || text.contains("log") || text.contains("dbg") || text.contains("printf") || text.contains("cout") {
                            let mut end = node.end_byte();
                            if source.get(end..end+1) == Some(";") {
                                end += 1;
                            }
                            
                            if !crate::batch::distillation::transforms::has_overlap(&edits, node.start_byte(), end) {
                                edits.push(Edit {
                                    start: node.start_byte(),
                                    end,
                                    replacement: String::new(),
                                });
                            }
                        }
                    }
                }
            }
        }
        edits
    }
}
