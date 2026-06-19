use tree_sitter::Tree;
use boil_core::language::Language;
use boil_engine::adapters::input::syntax::parser::edits::Edit;
use boil_core::canon::FileInfo;

pub trait CompressionStrategy {
    fn get_edits(&self, source: &str, tree: &Tree, lang: &Language, info: &FileInfo) -> Vec<Edit>;
    fn destructiveness_score(&self, lang: &Language) -> f64; // Score now depends on the language
    fn is_safe_for_lang(&self, lang: &Language) -> bool;
}

use std::sync::{OnceLock, Mutex, Arc};
use std::collections::HashMap;
use tree_sitter::Query;

pub fn get_cached_query(ts_lang: tree_sitter::Language, query_str: &'static str) -> Option<Arc<Query>> {
    static CACHE: OnceLock<Mutex<HashMap<(tree_sitter::Language, &'static str), Arc<Query>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().unwrap();
    let key = (ts_lang, query_str);
    if let Some(query) = map.get(&key) {
        return Some(Arc::clone(query));
    }
    if let Ok(query) = Query::new(ts_lang, query_str) {
        let arc_query = Arc::new(query);
        map.insert(key, Arc::clone(&arc_query));
        Some(arc_query)
    } else {
        None
    }
}

pub fn has_overlap(edits: &[Edit], start: usize, end: usize) -> bool {
    if edits.is_empty() {
        return false;
    }
    // Fast path: check the last edit first (matches are usually returned in order)
    if let Some(last) = edits.last() {
        if last.start < end && start < last.end {
            return true;
        }
    }
    // Fallback: binary search by start byte
    match edits.binary_search_by(|e| e.start.cmp(&start)) {
        Ok(_) => true,
        Err(idx) => {
            if idx > 0 {
                let prev = &edits[idx - 1];
                if prev.start < end && start < prev.end {
                    return true;
                }
            }
            if idx < edits.len() {
                let next = &edits[idx];
                if next.start < end && start < next.end {
                    return true;
                }
            }
            false
        }
    }
}
