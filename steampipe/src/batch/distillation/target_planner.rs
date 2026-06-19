use boil_core::canon::{FileInfo, graph::ProjectGraph};
use crate::batch::distillation::scoring;
use crate::batch::distillation::transforms::strategy::CompressionStrategy;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokenizers::Tokenizer;

pub struct TargetPlanner {
    strategies: Vec<Box<dyn CompressionStrategy>>,
    tokenizer: Option<Tokenizer>,
}

pub struct TargetResult {
    pub sources: HashMap<PathBuf, String>,
    pub target_tokens: usize,
    pub achieved: bool,
}

impl TargetPlanner {
    pub fn new(
        strategies: Vec<Box<dyn CompressionStrategy>>,
        tokenizer: Option<Tokenizer>,
    ) -> Self {
        TargetPlanner {
            strategies,
            tokenizer,
        }
    }

    pub fn achieve_target(
        &self,
        file_infos: &mut [FileInfo],
        graph: &ProjectGraph,
        target_tokens: usize,
        source_map: &HashMap<PathBuf, String>,
        raw_paths: &HashSet<PathBuf>,
        focus_paths: &HashSet<PathBuf>,
        pb: Option<&indicatif::ProgressBar>,
    ) -> TargetResult {
        let mut current_sources = source_map.clone();
        let mut total_tokens = self.calculate_total_tokens(&current_sources);

        if total_tokens <= target_tokens {
            return TargetResult {
                sources: current_sources,
                target_tokens,
                achieved: true,
            };
        }

        let mut steps = Vec::new();
        for (f_idx, file_info) in file_infos.iter().enumerate() {
            if raw_paths.contains(&file_info.path) || focus_paths.contains(&file_info.path) {
                continue;
            }
            for (s_idx, strategy) in self.strategies.iter().enumerate() {
                if strategy.is_safe_for_lang(&file_info.language) {
                    steps.push((f_idx, s_idx));
                }
            }
        }

        steps.sort_by(|&(f_a, s_a), &(f_b, s_b)| {
            let score_a = self.strategies[s_a].destructiveness_score(&file_infos[f_a].language);
            let score_b = self.strategies[s_b].destructiveness_score(&file_infos[f_b].language);
            
            if score_a != score_b {
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                let imp_a = scoring::calculate_importance(&file_infos[f_a], graph, focus_paths.contains(&file_infos[f_a].path)).total();
                let imp_b = scoring::calculate_importance(&file_infos[f_b], graph, focus_paths.contains(&file_infos[f_b].path)).total();
                imp_a.partial_cmp(&imp_b).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        let mut processed_files = HashSet::new();

        // Iterative refinement
        for (f_idx, s_idx) in steps {
            if total_tokens <= target_tokens {
                break;
            }

            if processed_files.insert(f_idx) {
                if let Some(ref bar) = pb {
                    bar.inc(1);
                }
            }

            let file_info = &file_infos[f_idx];
            let strategy = &self.strategies[s_idx];
            let source = current_sources.get(&file_info.path).unwrap();
            let lang = &file_info.language;

            let Some(mut parser) = boil_engine::adapters::input::syntax::parser::create_parser(lang) else {
                continue;
            };
            let Ok(tree) = boil_engine::adapters::input::syntax::parser::parse_source(&mut parser, source) else {
                continue;
            };

            let edits = strategy.get_edits(source, &tree, lang, file_info);
            if !edits.is_empty() {
                let new_source = boil_engine::adapters::input::syntax::parser::apply_edits(source, edits);

                if new_source != *source {
                    let old_tokens = self.count_tokens(source);
                    let new_tokens = self.count_tokens(&new_source);

                    total_tokens =
                        total_tokens.saturating_sub(old_tokens.saturating_sub(new_tokens));
                    current_sources.insert(file_info.path.clone(), new_source);
                }
            }
        }

        TargetResult {
            sources: current_sources,
            target_tokens,
            achieved: total_tokens <= target_tokens,
        }
    }

    fn count_tokens(&self, source: &str) -> usize {
        self.tokenizer
            .as_ref()
            .and_then(|t| t.encode(source, false).ok())
            .map(|e| e.get_ids().len())
            .unwrap_or_else(|| source.len() / 4)
    }

    fn calculate_total_tokens(&self, sources: &HashMap<PathBuf, String>) -> usize {
        sources.values().map(|s| self.count_tokens(s)).sum()
    }
}
