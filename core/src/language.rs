use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Language {
    Rust,
    C,
    Cpp,
    Go,
    Java,
    JavaScript,
    Python,
    TypeScript,
    Tsx,
    CSharp,
    Ruby,
    Kotlin,
    Swift,
    Unknown,
}

pub fn detect_language(path: &Path) -> Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Language::Rust,
        Some("c") => Language::C,
        Some("cpp" | "cc" | "cxx") => Language::Cpp,
        Some("h") => {
            if let Ok(content) = std::fs::read_to_string(path) {
                // Search for traces of C++ and, if none found, resort to C.
                if content.contains("class ")
                    || content.contains("template ")
                    || content.contains("namespace ")
                    || content.contains("using ")
                {
                    Language::Cpp
                } else {
                    Language::C
                }
            } else {
                Language::C // Default to C if read fails
            }
        }
        Some("js") => Language::JavaScript,
        Some("ts") => Language::TypeScript,
        Some("tsx") => Language::Tsx,
        Some("py") => Language::Python,
        Some("java") => Language::Java,
        Some("go") => Language::Go,
        Some("cs") => Language::CSharp,
        Some("rb") => Language::Ruby,
        Some("kt" | "kts") => Language::Kotlin,
        Some("swift") => Language::Swift,
        _ => Language::Unknown,
    }
}
