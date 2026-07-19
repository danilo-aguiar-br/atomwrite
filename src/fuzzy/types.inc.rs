/// Diagnostic detail about which fuzzy-matching strategy resolved a pair.
#[derive(Debug, Clone)]
pub struct FuzzyInfo {
    /// Whether a fuzzy (non-exact) strategy was used.
    pub fuzzy: bool,
    /// Name of the strategy that produced the match.
    pub strategy: String,
    /// Number of strategies attempted before finding a match.
    pub strategies_tried: u64,
    /// Similarity score of the match (0.0–1.0), if applicable.
    pub similarity: Option<f64>,
    /// Mini unified diff showing what matched vs what was expected.
    pub diff_preview: Option<String>,
    /// Number of occurrences replaced (1 unless `replace_all`).
    pub match_count: u64,
    /// True when `apply_indent_delta*` changed the replacement text.
    pub indent_adjusted: bool,
}

/// Options for [`match_pair_with`].
#[derive(Debug, Clone, Copy)]
pub struct MatchOpts {
    /// Fuzzy cascade mode (Auto, Aggressive, or Off=exact-only).
    pub mode: FuzzyMode,
    /// Optional similarity threshold override (CLI wins over section).
    pub threshold: Option<f64>,
    /// XDG `[fuzzy].threshold_aggressive` default.
    pub thr_aggressive: f64,
    /// XDG `[fuzzy].threshold_context` default.
    pub thr_context: f64,
    /// XDG `[fuzzy].threshold_jw` default.
    pub thr_jw: f64,
    /// XDG `[fuzzy].threshold` auto dual-gate floor.
    pub thr_auto: f64,
    /// When true, replace every occurrence; when false, require uniqueness.
    pub replace_all: bool,
}

impl Default for MatchOpts {
    fn default() -> Self {
        Self {
            mode: FuzzyMode::Auto,
            threshold: None,
            thr_aggressive: crate::constants::FUZZY_THRESHOLD_AGGRESSIVE,
            thr_context: crate::constants::FUZZY_THRESHOLD_CONTEXT,
            thr_jw: crate::constants::FUZZY_THRESHOLD_JW,
            thr_auto: crate::constants::FUZZY_THRESHOLD_AUTO,
            replace_all: false,
        }
    }
}

/// Build [`MatchOpts`] from CLI mode/threshold and XDG `FuzzySection` (G-FZZ thr wire).
pub fn match_opts_from_section(
    mode: FuzzyMode,
    cli_threshold: Option<f64>,
    section: &crate::config::FuzzySection,
    replace_all: bool,
) -> MatchOpts {
    MatchOpts {
        mode,
        threshold: cli_threshold,
        thr_aggressive: section.threshold_aggressive,
        thr_context: section.threshold_context,
        thr_jw: section.threshold_jw,
        thr_auto: section.threshold,
        replace_all,
    }
}

/// Adaptive floor: short patterns need higher thr to avoid FP (G-FZZ-118).
pub fn adaptive_threshold(base: f64, pattern_len: usize) -> f64 {
    if pattern_len > 0 && pattern_len < crate::constants::FUZZY_ADAPTIVE_SHORT_PATTERN_CHARS {
        (base + crate::constants::FUZZY_ADAPTIVE_SHORT_BOOST).min(1.0)
    } else {
        base
    }
}

/// Minimum similarity to serialize a best-candidate in error envelopes.
const BEST_CANDIDATE_MIN: f64 = crate::constants::FUZZY_BEST_CANDIDATE_MIN;
/// Max multi `did_you_mean` candidates.
const MAX_CANDIDATES: usize = crate::constants::FUZZY_MAX_CANDIDATES;

