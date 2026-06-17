use globset::GlobSet;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;

pub fn count_tokens(tokenizer: Option<&Tokenizer>, source: &str) -> usize {
    tokenizer
        .and_then(|t| t.encode(source, false).ok())
        .map(|e| e.get_ids().len())
        .unwrap_or_else(|| source.len() / 4)
}

pub fn relative_path(base: &Path, file: &Path) -> PathBuf {
    if base.is_file() {
        return file
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| file.to_path_buf());
    }

    file.strip_prefix(base).unwrap_or(file).to_path_buf()
}

pub fn matches_globset(globs: &GlobSet, relative: &Path, full: &Path) -> bool {
    if globs.is_match(relative) || globs.is_match(full) {
        return true;
    }
    if let Some(filename) = relative.file_name()
        && globs.is_match(filename)
    {
        return true;
    }
    for ancestor in relative.ancestors() {
        if ancestor.as_os_str().is_empty() || ancestor == Path::new(".") {
            continue;
        }
        if globs.is_match(ancestor) {
            return true;
        }
    }
    false
}

pub fn collect_files(input: &Path, distillery_root: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if input.is_file() {
        out.push(input.to_path_buf());
        return Ok(out);
    }
    let walker = walkdir::WalkDir::new(input)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".git");

    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        if let Some(dist_root) = distillery_root
            && path.starts_with(dist_root)
        {
            continue;
        }
        if path.is_file() {
            out.push(path.to_path_buf());
        }
    }
    out.sort();
    Ok(out)
}
