use boil_core::canon::{FileInfo, graph::ProjectGraph};

#[derive(Debug, Clone)]
pub struct Importance {
    pub structural: f64,
    pub user_priority: f64,
}

impl Importance {
    pub fn total(&self) -> f64 {
        self.structural + self.user_priority
    }
}

pub fn calculate_importance(
    file_info: &FileInfo,
    _graph: &ProjectGraph,
    is_focused: bool,
) -> Importance {
    // 1. Structural score: sum of symbol popularity
    // A file is important if it defines symbols that are frequently referenced elsewhere.
    let symbol_importance: f64 = file_info.symbols.iter()
        .map(|s| s.references as f64 * 2.0) // Each reference adds 2 points to the file's importance
        .sum();

    // Still include baseline importance so files aren't automatically dropped if they define no symbols
    let structural = 5.0 + symbol_importance;

    // 2. User priority (Focus bonus)
    let user_priority = if is_focused { 100.0 } else { 0.0 };

    Importance {
        structural,
        user_priority,
    }
}
