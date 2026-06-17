use crate::adapters::input::syntax::extractors::get_extractor;
use crate::adapters::input::syntax::parser::{create_parser, parse_source};
use crate::core::canon::FileInfo;
use crate::core::language::detect_language;
use crate::core::utils::count_tokens;
use crate::ports::syntax::SyntaxProvider;
use anyhow::Result;
use std::path::Path;
use tokenizers::Tokenizer;

/// A polyglot syntax provider leveraging Tree-sitter.
///
/// Utilizes native Tree-sitter grammar bindings for dozens of languages (Rust, Python, Go, etc.)
/// to robustly extract symbols, scopes, and dependencies from the syntax tree.
pub struct TreeSitterProvider {
    tokenizer: Option<Tokenizer>,
}

impl Default for TreeSitterProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSitterProvider {
    pub fn new() -> Self {
        Self {
            tokenizer: Tokenizer::from_pretrained("gpt2", None).ok(),
        }
    }
}

impl SyntaxProvider for TreeSitterProvider {
    fn parse_file(&self, path: &Path, source: &str) -> Result<FileInfo> {
        let lang = detect_language(path);
        let mut parser =
            create_parser(&lang).ok_or_else(|| anyhow::anyhow!("Unsupported language"))?;
        let tree = parse_source(&mut parser, source)?;
        let extractor = get_extractor(lang);

        let token_count = count_tokens(self.tokenizer.as_ref(), source);
        Ok(extractor.analyze(path, source, &tree, token_count))
    }
}
