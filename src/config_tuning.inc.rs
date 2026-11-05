// SPDX-License-Identifier: MIT OR Apache-2.0
// Tuning sections, the layered config load and the process-wide effective
// tuning (v0.1.37, T6a). Included from config.rs.

/// Worker-bound tuning loaded from `[concurrency]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConcurrencySection {
    /// Ceiling of the automatic worker bound (`--threads` omitted or `0`).
    pub max_threads: usize,
    /// Percentage of free RAM eligible for concurrent file tasks (1..=100).
    pub ram_fraction_percent: u64,
}

impl Default for ConcurrencySection {
    fn default() -> Self {
        Self {
            max_threads: crate::constants::MAX_AUTO_THREADS,
            ram_fraction_percent: crate::constants::RAM_FRACTION_PERCENT,
        }
    }
}

/// Advisory lock tuning loaded from `[lock]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LockSection {
    /// Longest wait for the advisory lock of a write target, in milliseconds.
    pub acquire_timeout_ms: u64,
}

impl Default for LockSection {
    fn default() -> Self {
        Self {
            acquire_timeout_ms: crate::constants::LOCK_ACQUIRE_TIMEOUT_MS,
        }
    }
}

/// Retry backoff tuning loaded from `[retry]`.
///
/// A key left unset keeps the built-in policy of each retry site, so the lock
/// and the tempfile persist retry keep their own constants.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetrySection {
    /// Ceiling of the first backoff wait, in milliseconds.
    pub base_ms: Option<u64>,
    /// Ceiling no single backoff wait exceeds, in milliseconds.
    pub max_ms: Option<u64>,
    /// Total time budget of a retry that has no dedicated key, in milliseconds.
    ///
    /// The advisory lock budget is `lock.acquire_timeout_ms`.
    pub max_elapsed_ms: Option<u64>,
}

/// `extract-artifact` bounds loaded from `[extract]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExtractSection {
    /// Rows sampled per sheet (0 = header only).
    pub max_sample_rows: usize,
    /// Characters kept per text excerpt.
    pub max_text_chars: usize,
    /// Worksheets read per workbook.
    pub max_sheets: usize,
    /// Slides read per presentation.
    pub max_slides: usize,
}

impl Default for ExtractSection {
    fn default() -> Self {
        use crate::commands::extract_artifact as artifact;
        Self {
            max_sample_rows: artifact::DEFAULT_MAX_SAMPLE_ROWS,
            max_text_chars: artifact::DEFAULT_MAX_TEXT_CHARS,
            max_sheets: artifact::MAX_SHEETS,
            max_slides: artifact::MAX_SLIDES,
        }
    }
}
/// Reject a tuning value outside `min..=max`.
fn require_in_range(
    section: &str,
    key: &str,
    value: u64,
    min: u64,
    max: u64,
) -> Result<(), AtomwriteError> {
    if !(min..=max).contains(&value) {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "config [{section}] {key} = {value} {} {min}..={max}",
                crate::constants::CONFIG_OUT_OF_RANGE_REASON_MARKER
            ),
        });
    }
    Ok(())
}

/// Reject a tuning value above the value of the key it must not exceed.
fn require_not_above(
    section: &str,
    (key, value): (&str, u64),
    (limit_key, limit): (&str, u64),
) -> Result<(), AtomwriteError> {
    if value > limit {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "config [{section}] {key} = {value} {} {limit_key} = {limit}",
                crate::constants::CONFIG_RELATION_REASON_MARKER
            ),
        });
    }
    Ok(())
}

/// Validate the tuning sections (`concurrency`, `lock`, `retry`, `extract`,
/// `search.max_inflight_bytes`); fail closed on bad config.
///
/// Every key has a named range in `constants`, and the retry keys are checked
/// against each other with the value each one would have at run time.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when any `[concurrency]`, `[lock]`, `[retry]` or `[extract]` key falls outside its named range, or when `retry.base_ms` exceeds `retry.max_ms` or `retry.max_elapsed_ms`.
pub fn validate_tuning(cfg: &AtomwriteConfig) -> Result<(), AtomwriteError> {
    use crate::constants as k;
    let c = &cfg.concurrency;
    require_in_range(
        "concurrency",
        "max_threads",
        c.max_threads as u64,
        k::CONCURRENCY_MIN_THREADS,
        k::CONCURRENCY_MAX_THREADS_CEILING,
    )?;
    require_in_range(
        "concurrency",
        "ram_fraction_percent",
        c.ram_fraction_percent,
        k::RAM_FRACTION_PERCENT_MIN,
        k::PERCENT_MAX,
    )?;
    require_in_range(
        "lock",
        "acquire_timeout_ms",
        cfg.lock.acquire_timeout_ms,
        k::LOCK_MIN_ACQUIRE_TIMEOUT_MS,
        k::LOCK_MAX_ACQUIRE_TIMEOUT_MS,
    )?;
    let r = &cfg.retry;
    for (key, value, max) in [
        ("base_ms", r.base_ms, k::RETRY_MAX_WAIT_CEILING_MS),
        ("max_ms", r.max_ms, k::RETRY_MAX_WAIT_CEILING_MS),
        (
            "max_elapsed_ms",
            r.max_elapsed_ms,
            k::RETRY_MAX_ELAPSED_CEILING_MS,
        ),
    ] {
        if let Some(value) = value {
            require_in_range("retry", key, value, k::RETRY_MIN_MS, max)?;
        }
    }
    let base = r.base_ms.unwrap_or(k::LOCK_RETRY_BASE_MS);
    require_not_above(
        "retry",
        ("base_ms", base),
        ("max_ms", r.max_ms.unwrap_or(k::LOCK_RETRY_CAP_MS)),
    )?;
    require_not_above(
        "retry",
        ("base_ms", base),
        (
            "max_elapsed_ms",
            r.max_elapsed_ms.unwrap_or(k::PERSIST_RETRY_MAX_ELAPSED_MS),
        ),
    )?;
    let e = &cfg.extract;
    require_in_range(
        "extract",
        "max_sample_rows",
        e.max_sample_rows as u64,
        0,
        k::EXTRACT_MAX_SAMPLE_ROWS_CEILING,
    )?;
    require_in_range(
        "extract",
        "max_text_chars",
        e.max_text_chars as u64,
        1,
        k::EXTRACT_MAX_TEXT_CHARS_CEILING,
    )?;
    require_in_range(
        "extract",
        "max_sheets",
        e.max_sheets as u64,
        1,
        k::EXTRACT_MAX_PARTS_CEILING,
    )?;
    require_in_range(
        "extract",
        "max_slides",
        e.max_slides as u64,
        1,
        k::EXTRACT_MAX_PARTS_CEILING,
    )?;
    Ok(())
}
/// Validate every section of an already parsed configuration.
///
/// Runs the same checks at startup and before `config set` writes a file.
///
/// # Errors
///
/// Returns the `AtomwriteError` of the first section that fails: `AtomwriteError::InvalidInput` from [`validate_fuzzy`], [`validate_write`], [`validate_search_section`] or [`validate_tuning`], or `AtomwriteError::ConfigInvalid` from [`validate_log`].
pub fn validate_all(cfg: &AtomwriteConfig) -> Result<(), AtomwriteError> {
    validate_fuzzy(&cfg.fuzzy)?;
    validate_write(&cfg.write)?;
    validate_search_section(&cfg.search)?;
    crate::config::validate_read_output(&cfg.read, &cfg.output)?;
    crate::config::validate_summary_section(&cfg.summary)?;
    validate_log(&cfg.log)?;
    validate_tuning(cfg)
}

/// Effective values of the keys the product consults while it runs.
///
/// Built once from the layered config by [`install_tuning`]; consumers deep in
/// the call tree (lock, retry, thread pool, search budget, artifact bounds)
/// read it through [`tuning`] instead of threading the config through every
/// signature. A CLI flag, when the verb has one, still wins over these values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuning {
    /// `concurrency.max_threads`.
    pub max_threads: usize,
    /// `concurrency.ram_fraction_percent`.
    pub ram_fraction_percent: u64,
    /// `lock.acquire_timeout_ms`.
    pub lock_acquire_timeout_ms: u64,
    /// `retry.base_ms`; `None` keeps the policy of each retry site.
    pub retry_base_ms: Option<u64>,
    /// `retry.max_ms`; `None` keeps the policy of each retry site.
    pub retry_max_ms: Option<u64>,
    /// `retry.max_elapsed_ms`; `None` keeps the policy of each retry site.
    pub retry_max_elapsed_ms: Option<u64>,
    /// `search.format` says `text`, so stdout carries result lines.
    pub search_format_is_text: bool,
    /// `search.max_inflight_bytes`.
    pub search_max_inflight_bytes: u64,
    /// `extract.max_sample_rows`.
    pub extract_max_sample_rows: usize,
    /// `extract.max_text_chars`.
    pub extract_max_text_chars: usize,
    /// `extract.max_sheets`.
    pub extract_max_sheets: usize,
    /// `extract.max_slides`.
    pub extract_max_slides: usize,
    /// `summary.errors_max_entries`.
    pub summary_errors_max_entries: usize,
    /// `read.max_output_bytes`.
    pub read_max_output_bytes: usize,
    /// `read.number`.
    pub read_number: bool,
    /// `delete.backup`.
    pub delete_backup: Option<bool>,
    /// `output.profile` resolves to `lean`.
    pub output_lean: bool,
}

impl Tuning {
    /// Project a parsed configuration onto the runtime tuning.
    #[must_use]
    pub fn from_config(cfg: &AtomwriteConfig) -> Self {
        Self {
            max_threads: cfg.concurrency.max_threads,
            ram_fraction_percent: cfg.concurrency.ram_fraction_percent,
            lock_acquire_timeout_ms: cfg.lock.acquire_timeout_ms,
            retry_base_ms: cfg.retry.base_ms,
            retry_max_ms: cfg.retry.max_ms,
            retry_max_elapsed_ms: cfg.retry.max_elapsed_ms,
            search_format_is_text: cfg.search.format == "text",
            search_max_inflight_bytes: cfg.search.max_inflight_bytes,
            extract_max_sample_rows: cfg.extract.max_sample_rows,
            extract_max_text_chars: cfg.extract.max_text_chars,
            extract_max_sheets: cfg.extract.max_sheets,
            extract_max_slides: cfg.extract.max_slides,
            summary_errors_max_entries: cfg.summary.errors_max_entries,
            read_max_output_bytes: cfg.read.max_output_bytes,
            read_number: cfg.read.number,
            delete_backup: cfg.delete.backup,
            output_lean: cfg.output.profile.eq_ignore_ascii_case("lean"),
        }
    }
}

impl Default for Tuning {
    fn default() -> Self {
        Self::from_config(&AtomwriteConfig::default())
    }
}

static TUNING: std::sync::OnceLock<Tuning> = std::sync::OnceLock::new();

/// Record the effective tuning of this process; the first call wins.
pub fn install_tuning(cfg: &AtomwriteConfig) {
    if TUNING.set(Tuning::from_config(cfg)).is_err() {
        tracing::debug!("tuning already installed; keeping the first value");
    }
}

/// Byte ceiling of a whole-file `read`, in bytes.
#[must_use]
pub fn read_max_output_bytes() -> usize {
    tuning().read_max_output_bytes
}

/// Whether raw reads number their lines by default.
#[must_use]
pub fn read_number_default() -> bool {
    tuning().read_number
}

/// Whether `delete` keeps a backup when neither flag nor `defaults.backup`
/// says otherwise.
#[must_use]
pub fn delete_backup() -> Option<bool> {
    tuning().delete_backup
}

/// Effective tuning of this process (built-in defaults until installed).
#[must_use]
pub fn tuning() -> Tuning {
    TUNING.get().copied().unwrap_or_default()
}

/// Most `errors[]` entries a terminal summary lists.
///
/// Every failure recorded asks for it, so the whole cost is one `OnceLock`
/// load of a `Copy` value: no allocation and no lock.
#[must_use]
pub fn summary_errors_max_entries() -> usize {
    tuning().summary_errors_max_entries
}

/// Raw text of the global config, read at most once per process.
struct GlobalText {
    path: std::path::PathBuf,
    text: Result<String, String>,
}

static GLOBAL_TEXT: std::sync::OnceLock<Option<GlobalText>> = std::sync::OnceLock::new();

/// The global config file, when it exists. Startup logging and the layered
/// load both go through here, so the file is read once.
fn global_text() -> Option<&'static GlobalText> {
    GLOBAL_TEXT
        .get_or_init(|| {
            let path = crate::storage::global_config_path().filter(|p| p.is_file())?;
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string());
            Some(GlobalText { path, text })
        })
        .as_ref()
}

/// One parsed config file: the strict typed view and the raw table.
struct Layer {
    config: AtomwriteConfig,
    table: toml::Table,
}

/// Parse one config file's text; every failure names the file in the reason.
fn parse_layer(path: &Path, content: &str) -> Result<Layer, AtomwriteError> {
    let config: AtomwriteConfig =
        toml::from_str(content).map_err(|e| AtomwriteError::ConfigInvalid {
            reason: format!("malformed config {}: {e}", path.display()),
        })?;
    if config.version > crate::constants::CONFIG_SCHEMA_VERSION {
        return Err(AtomwriteError::ConfigInvalid {
            reason: format!(
                "config {} declares version {}; this build supports up to {}",
                path.display(),
                config.version,
                crate::constants::CONFIG_SCHEMA_VERSION
            ),
        });
    }
    let table: toml::Table =
        toml::from_str(content).map_err(|e| AtomwriteError::ConfigInvalid {
            reason: format!("malformed config {}: {e}", path.display()),
        })?;
    tracing::debug!(path = %path.display(), "loaded config");
    Ok(Layer { config, table })
}

/// Read and parse one config file.
fn read_layer(path: &Path) -> Result<Layer, AtomwriteError> {
    let content = std::fs::read_to_string(path).map_err(|e| AtomwriteError::ConfigInvalid {
        reason: format!("cannot read config {}: {e}", path.display()),
    })?;
    parse_layer(path, &content)
}

/// Overlay `over` on `base`: tables merge key by key, any other value replaces.
fn merge_tables(mut base: toml::Table, over: toml::Table) -> toml::Table {
    for (key, value) in over {
        match (base.remove(&key), value) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => {
                base.insert(key, toml::Value::Table(merge_tables(b, o)));
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
    base
}
