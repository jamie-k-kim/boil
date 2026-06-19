use tree_sitter::{QueryCursor, Tree};
use boil_engine::adapters::input::syntax::parser::edits::Edit;
use boil_core::language::Language;
use crate::batch::distillation::transforms::strategy::CompressionStrategy;
use boil_core::canon::FileInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonMode {
    ArchitecturalOnly,
    TypesAndSignatures,
    SignaturesOnly,
}

pub struct Skeletonizer {
    pub mode: SkeletonMode,
}

impl Skeletonizer {
    pub fn new(mode: SkeletonMode) -> Self {
        Skeletonizer { mode }
    }
}

impl CompressionStrategy for Skeletonizer {

    fn destructiveness_score(&self, _lang: &Language) -> f64 {
        match self.mode {
            SkeletonMode::ArchitecturalOnly => 0.95,
            SkeletonMode::TypesAndSignatures => 0.7,
            SkeletonMode::SignaturesOnly => 0.9,
        }
    }
    
    fn is_safe_for_lang(&self, _lang: &Language) -> bool { true }

    fn get_edits(&self, source: &str, tree: &Tree, lang: &Language, _info: &FileInfo) -> Vec<Edit> {
        let mut edits = Vec::new();

        if self.mode == SkeletonMode::ArchitecturalOnly {
            let func_queries = match lang {
                Language::Rust => vec!["(function_item) @func", "(impl_item) @func"],
                Language::Python => vec!["(function_definition) @func"],
                Language::JavaScript | Language::TypeScript | Language::Tsx => vec![
                    "(function_declaration) @func",
                    "(method_definition) @func",
                    "(arrow_function) @func",
                ],
                Language::Java => vec!["(method_declaration) @func", "(constructor_declaration) @func"],
                Language::Go => vec!["(function_declaration) @func", "(method_declaration) @func"],
                Language::C | Language::Cpp => vec!["(function_definition) @func"],
                Language::CSharp => vec![
                    "(method_declaration) @func",
                    "(constructor_declaration) @func",
                ],
                Language::Kotlin => vec![
                    "(function_declaration) @func",
                ],
                Language::Swift => vec![
                    "(function_declaration) @func",
                    "(initializer_declaration) @func",
                    "(deinitializer_declaration) @func",
                ],
                Language::Ruby => vec![
                    "(method) @func",
                ],
                _ => vec![],
            };

            for query_str in func_queries {
                if let Some(query) = crate::batch::distillation::transforms::get_cached_query(tree.language(), query_str) {
                    let mut cursor = QueryCursor::new();
                    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
                    for m in matches {
                        for cap in m.captures {
                            let node = cap.node;
                            if crate::batch::distillation::transforms::has_overlap(&edits, node.start_byte(), node.end_byte()) { continue; }
                            edits.push(Edit {
                                start: node.start_byte(),
                                end: node.end_byte(),
                                replacement: String::new(),
                            });
                        }
                    }
                }
            }
            return edits;
        }

        let body_queries = match lang {
            Language::Rust => vec![
                "(function_item (block) @body)",
                "(function_item body: (block) @body)",
            ],
            Language::Python => vec![
                "(function_definition (block) @body)",
                "(class_definition (block) @body)",
            ],
            Language::JavaScript | Language::TypeScript | Language::Tsx => vec![
                "(statement_block) @body",
                "(function_declaration body: (statement_block) @body)",
                "(method_definition body: (statement_block) @body)",
                "(arrow_function body: (statement_block) @body)",
            ],
            Language::Java => vec![
                "(method_declaration (block) @body)",
                "(constructor_declaration (block) @body)",
            ],
            Language::Go => vec![
                "(function_declaration (block) @body)",
                "(method_declaration (block) @body)",
            ],
            Language::C | Language::Cpp => vec![
                "(function_definition (compound_statement) @body)",
            ],
            Language::CSharp => vec![
                "(method_declaration (block) @body)",
                "(constructor_declaration (block) @body)",
            ],
            Language::Kotlin => vec![
                "(function_declaration (function_body) @body)",
            ],
            Language::Swift => vec![
                "(function_declaration (code_block) @body)",
            ],
            Language::Ruby => vec![
                "(method (body_statement) @body)",
            ],
            _ => vec![],
        };

        for query_str in body_queries {
            if let Some(query) = crate::batch::distillation::transforms::get_cached_query(tree.language(), query_str) {
                let mut cursor = QueryCursor::new();
                let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
                for m in matches {
                    for cap in m.captures {
                        let node = cap.node;
                        if node.end_byte() - node.start_byte() < 5 { continue; }
                        if crate::batch::distillation::transforms::has_overlap(&edits, node.start_byte(), node.end_byte()) { continue; }
                        let replacement = if lang == &Language::Ruby { "\n  # ...\n".to_string() } else { " { ... } ".to_string() };
                        edits.push(Edit {
                            start: node.start_byte(),
                            end: node.end_byte(),
                            replacement,
                        });
                    }
                }
            }
        }
        // ... field queries for SignaturesOnly ...
        if self.mode == SkeletonMode::SignaturesOnly {
            let field_queries = match lang {
                Language::Rust => vec!["(struct_item (field_declaration_list) @fields)"],
                Language::TypeScript | Language::Tsx | Language::JavaScript => vec!["(class_body (public_field_definition) @field)"],
                Language::Java => vec!["(field_declaration) @field"],
                Language::CSharp => vec!["(class_declaration (class_body (field_declaration) @field))"],
                Language::Kotlin => vec!["(class_body (property_declaration) @field)"],
                Language::Swift => vec!["(member_decl_block (variable_declaration) @field)"],
                _ => vec![],
            };
            for query_str in field_queries {
                if let Some(query) = crate::batch::distillation::transforms::get_cached_query(tree.language(), query_str) {
                    let mut cursor = QueryCursor::new();
                    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
                    for m in matches {
                        for cap in m.captures {
                            let node = cap.node;
                            let replacement = if lang == &Language::Rust { ";".to_string() } else { String::new() };
                            if crate::batch::distillation::transforms::has_overlap(&edits, node.start_byte(), node.end_byte()) { continue; }
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
