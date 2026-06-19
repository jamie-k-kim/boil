use tree_sitter::{QueryCursor, Tree};
use boil_engine::adapters::input::syntax::parser::edits::Edit;
use boil_core::language::Language;
use crate::batch::distillation::transforms::strategy::CompressionStrategy;
use boil_core::canon::FileInfo;

pub struct CommentRemoval;

impl CompressionStrategy for CommentRemoval {
    fn destructiveness_score(&self, _lang: &Language) -> f64 { 0.2 }
    fn is_safe_for_lang(&self, _lang: &Language) -> bool { true }

    fn get_edits(&self, source: &str, tree: &Tree, _lang: &Language, _info: &FileInfo) -> Vec<Edit> {
        let mut edits = Vec::new();
        let possible_queries = [
            "(comment) @c", 
            "(line_comment) @c", 
            "(block_comment) @c", 
            "(doc_comment) @c",
            "(expression_statement (string) @c)",
        ];

        for query_str in possible_queries {
            if let Some(query) = crate::batch::distillation::transforms::get_cached_query(tree.language(), query_str) {
                let mut cursor = QueryCursor::new();
                let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

                for m in matches {
                    for cap in m.captures {
                        let node = cap.node;
                        let start = node.start_byte();
                        let mut end = node.end_byte();

                        if end < source.len() {
                            let tail = &source[end..];
                            if let Some(next_char) = tail.chars().next() {
                                if next_char == '\n' {
                                    end += 1;
                                } else if next_char == '\r' {
                                    if tail.get(1..2) == Some("\n") {
                                        end += 2;
                                    } else {
                                        end += 1;
                                    }
                                }
                            }
                        }

                        if !crate::batch::distillation::transforms::has_overlap(&edits, start, end) {
                            edits.push(Edit {
                                start,
                                end,
                                replacement: String::new(),
                            });
                        }
                    }
                }
            }
        }
        edits
    }
}
