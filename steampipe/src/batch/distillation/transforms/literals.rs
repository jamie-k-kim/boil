use tree_sitter::{QueryCursor, Tree};
use boil_engine::adapters::input::syntax::parser::edits::Edit;
use boil_core::language::Language;
use crate::batch::distillation::transforms::strategy::CompressionStrategy;
use boil_core::canon::FileInfo;

pub struct LiteralShrinker {
    pub threshold_bytes: usize,
}

impl LiteralShrinker {
    pub fn new(threshold_bytes: usize) -> Self {
        LiteralShrinker { threshold_bytes }
    }
}

impl CompressionStrategy for LiteralShrinker {
    fn destructiveness_score(&self, _lang: &Language) -> f64 { 0.4 }
    fn is_safe_for_lang(&self, _lang: &Language) -> bool { true }

    fn get_edits(&self, source: &str, tree: &Tree, lang: &Language, _info: &FileInfo) -> Vec<Edit> {
        let mut edits = Vec::new();

        let queries = match lang {
            Language::Rust => vec![
                "(string_literal) @lit",
                "(raw_string_literal) @lit",
                "(array_expression) @lit",
            ],
            Language::Python => vec![
                "(string) @lit",
                "(list) @lit",
                "(dictionary) @lit",
            ],
            Language::JavaScript | Language::TypeScript | Language::Tsx => vec![
                "(string) @lit",
                "(template_string) @lit",
                "(array) @lit",
                "(object) @lit",
            ],
            Language::Java => vec![
                "(string_literal) @lit",
                "(array_initializer) @lit",
            ],
            Language::Go => vec![
                "(string_literal) @lit",
                "(raw_string_literal) @lit",
                "(composite_literal) @lit",
            ],
            Language::C | Language::Cpp => vec![
                "(string_literal) @lit",
                "(initializer_list) @lit",
            ],
            Language::CSharp => vec![
                "(string_literal) @lit",
                "(character_literal) @lit",
            ],
            Language::Kotlin => vec![
                "(string_literal) @lit",
                "(character_literal) @lit",
            ],
            Language::Swift => vec![
                "(line_string_literal) @lit",
                "(array_literal) @lit",
                "(dictionary_literal) @lit",
            ],
            Language::Ruby => vec![
                "(string) @lit",
                "(array) @lit",
                "(hash) @lit",
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
                        let len = node.end_byte() - node.start_byte();
                        
                        if len > self.threshold_bytes && !crate::batch::distillation::transforms::has_overlap(&edits, node.start_byte(), node.end_byte()) {
                            let replacement = format!(" \"[OMITTED: {} bytes]\" ", len);
                            edits.push(Edit {
                                start: node.start_byte(),
                                end: node.end_byte(),
                                replacement,
                            });
                        }
                    }
                }
            }
        }

        edits
    }
}
