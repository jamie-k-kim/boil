use boil_core::language::Language as BoilLanguage;
use anyhow::Result;
use tree_sitter::{Language as TsLanguage, Parser, Tree};

fn language_rust() -> TsLanguage {
    tree_sitter_rust::language()
}

fn language_c() -> TsLanguage {
    tree_sitter_c::language()
}

fn language_javascript() -> TsLanguage {
    tree_sitter_javascript::language()
}

fn language_python() -> TsLanguage {
    tree_sitter_python::language()
}

fn language_typescript() -> TsLanguage {
    tree_sitter_typescript::language_typescript()
}

fn language_tsx() -> TsLanguage {
    tree_sitter_typescript::language_tsx()
}

fn language_java() -> TsLanguage {
    tree_sitter_java::language()
}

fn language_go() -> TsLanguage {
    tree_sitter_go::language()
}

fn language_cpp() -> TsLanguage {
    tree_sitter_cpp::language()
}

pub fn create_parser(lang: &BoilLanguage) -> Option<Parser> {
    let mut parser = Parser::new();

    let language = match lang {
        BoilLanguage::Rust => language_rust(),
        BoilLanguage::C => language_c(),
        BoilLanguage::Cpp => language_cpp(),
        BoilLanguage::JavaScript => language_javascript(),
        BoilLanguage::TypeScript => language_typescript(),
        BoilLanguage::Tsx => language_tsx(),
        BoilLanguage::Python => language_python(),
        BoilLanguage::Java => language_java(),
        BoilLanguage::Go => language_go(),
        BoilLanguage::CSharp => tree_sitter_c_sharp::language(),
        BoilLanguage::Ruby => tree_sitter_ruby::language(),
        BoilLanguage::Kotlin => tree_sitter_kotlin::language(),
        BoilLanguage::Swift => tree_sitter_swift::language(),
        BoilLanguage::Unknown => return None,
    };

    parser.set_language(language).ok()?;
    Some(parser)
}

pub fn parse_source(parser: &mut Parser, source: &str) -> Result<Tree> {
    parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse source"))
}
