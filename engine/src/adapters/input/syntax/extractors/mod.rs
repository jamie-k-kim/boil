pub mod c;
pub mod cpp;
pub mod csharp;
pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod swift;
pub mod typescript;

use crate::adapters::input::syntax::extractors::c::CExtractor;
use crate::adapters::input::syntax::extractors::cpp::CppExtractor;
use crate::adapters::input::syntax::extractors::csharp::CSharpExtractor;
use crate::adapters::input::syntax::extractors::go::GoExtractor;
use crate::adapters::input::syntax::extractors::java::JavaExtractor;
use crate::adapters::input::syntax::extractors::javascript::JSExtractor;
use crate::adapters::input::syntax::extractors::kotlin::KotlinExtractor;
use crate::adapters::input::syntax::extractors::python::PythonExtractor;
use crate::adapters::input::syntax::extractors::ruby::RubyExtractor;
use crate::adapters::input::syntax::extractors::rust::RustExtractor;
use crate::adapters::input::syntax::extractors::swift::SwiftExtractor;
use crate::adapters::input::syntax::extractors::typescript::TSExtractor;
use boil_core::canon::FileInfo;
use std::path::Path;
use tree_sitter::Tree;

pub trait SymbolExtractor {
    fn analyze(&self, path: &Path, source: &str, tree: &Tree, token_count: usize) -> FileInfo;
}

pub fn get_extractor(language: boil_core::language::Language) -> Box<dyn SymbolExtractor> {
    match language {
        boil_core::language::Language::Rust => Box::new(RustExtractor),
        boil_core::language::Language::Python => Box::new(PythonExtractor),
        boil_core::language::Language::C => Box::new(CExtractor),
        boil_core::language::Language::Cpp => Box::new(CppExtractor),
        boil_core::language::Language::Java => Box::new(JavaExtractor),
        boil_core::language::Language::JavaScript => Box::new(JSExtractor),
        boil_core::language::Language::TypeScript | boil_core::language::Language::Tsx => {
            Box::new(TSExtractor)
        }
        boil_core::language::Language::Go => Box::new(GoExtractor),
        boil_core::language::Language::CSharp => Box::new(CSharpExtractor),
        boil_core::language::Language::Ruby => Box::new(RubyExtractor),
        boil_core::language::Language::Kotlin => Box::new(KotlinExtractor),
        boil_core::language::Language::Swift => Box::new(SwiftExtractor),
        boil_core::language::Language::Unknown => {
            panic!("No extractor for this unknown language")
        }
    }
}
