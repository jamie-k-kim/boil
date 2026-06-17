#[derive(Debug, Clone)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

// Apply edits safely (must be reverse sorted and non-overlapping)
pub fn apply_edits(source: &str, edits: Vec<Edit>) -> String {
    if edits.is_empty() {
        return source.to_string();
    }

    let mut sorted_edits = edits;
    // Sort by start position ascending, then end position descending (larger ranges first)
    sorted_edits.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

    let mut merged: Vec<Edit> = Vec::new();
    for edit in sorted_edits {
        if let Some(last) = merged.last_mut()
            && edit.start < last.end
        {
            // Overlap detected.
            // Since we sorted by start ascending and then end descending,
            // this 'edit' is entirely contained within or starts after 'last'.
            // If it's contained, we just ignore it (last already covers it).
            if edit.end > last.end {
                // This case shouldn't happen with our sorting unless they partially overlap
                // but start at the same place (impossible due to then(b.end.cmp(&a.end)))
                // or start later and end later.
                // For now, we just extend the last edit.
                last.end = edit.end;
            }
            continue;
        }
        merged.push(edit);
    }

    // Apply in reverse to keep offsets valid
    merged.sort_by_key(|b| std::cmp::Reverse(b.start));

    let mut result = source.to_string();
    for e in merged {
        if e.end <= result.len() && e.start <= e.end {
            result.replace_range(e.start..e.end, &e.replacement);
        }
    }

    result
}
