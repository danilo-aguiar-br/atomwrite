// SPDX-License-Identifier: MIT OR Apache-2.0
// Shape types and expression parsing.
// Included from output_shape.inc.rs.

/// A single top-level filter predicate (`key=value`, `key==value`, or `key!=value`).
#[derive(Debug, Clone, PartialEq)]
pub struct FilterPred {
    /// Top-level object key.
    pub key: String,
    /// Expected JSON-ish value after parse.
    pub value: Value,
    /// When true, require inequality.
    pub not_equal: bool,
}

/// Sort criterion for buffered event ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// Order by the requested key only, mixed types last.
    Key,
    /// Order by the requested key, then by a stable line index.
    KeyStable,
    /// Order by top-level `path`, then `line`, then the stable line index.
    PathLine,
    /// Order by top-level `line`, then `path`, then the stable line index.
    LinePath,
}

/// Reduction applied to a buffered event group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggOp {
    /// Count the events in the group.
    Count,
    /// Sum a numeric key of the group.
    Sum(String),
    /// Minimum numeric key of the group.
    Min(String),
    /// Maximum numeric key of the group.
    Max(String),
}

/// A group-by plus reduction pair (`--aggregate key:op[:field]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggSpec {
    /// Top-level key whose value groups the events.
    pub group: String,
    /// Reduction applied inside each group.
    pub op: AggOp,
}

/// A grouping table built from positioned NDJSON stdin (`--join`).
#[derive(Debug, Clone)]
pub struct JoinSpec {
    /// Grouping key inside the external NDJSON records.
    pub key: String,
    /// Top-level suffix written on the event envelope.
    pub suffix: String,
}

/// How much of the envelope the consumer wants back.
///
/// `Full` is the published contract. `Lean` drops fields an agent never reads,
/// and it is opt-in through `[output] profile = "lean"`, so a consumer that
/// already parses the full record is never surprised by a missing key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputProfile {
    /// Every field the verb produces.
    #[default]
    Full,
    /// Only the fields that change the next decision.
    Lean,
}

impl OutputProfile {
    /// Read the profile from its configuration spelling.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        if name.eq_ignore_ascii_case("lean") {
            Self::Lean
        } else {
            Self::Full
        }
    }
}

/// Optional projection / filter / limit applied by [`NdjsonWriter`].
#[derive(Debug, Clone, Default)]
pub struct OutputShape {
    /// Requested output profile.
    pub profile: OutputProfile,
    /// Keep the diagnostic keys on a failure envelope (`-v` and above).
    ///
    /// Without it an error carries its core only: `code`, `exit`, `path`,
    /// `suggestion` and the conditional signal fields. `message`,
    /// `error_class`, `retryable` and `suggestion_lang` are diagnostics the
    /// caller reads while debugging, and a one-shot agent pays for them on
    /// every failure without branching on any of them.
    pub verbose: bool,
    /// Top-level keys to keep (order preserved). `None` = no projection.
    pub select: Option<Vec<String>>,
    /// AND-combined equality filters. Empty = no filter.
    pub filters: Vec<FilterPred>,
    /// Max events after filter. `None` = unlimited.
    pub limit: Option<u64>,
    /// Sort key selector (`--sort`, `--sort path-line`, `--sort line-path`).
    pub sort_key: Option<String>,
    /// Sort criterion derived from `sort_key`.
    pub sort: Option<SortKey>,
    /// Dedupe keys (`--dedupe`). Empty = no dedupe.
    pub dedupe_keys: Vec<String>,
    /// Aggregate specs (`--aggregate`). Empty = no aggregation.
    pub aggregates: Vec<AggSpec>,
    /// Grouping tables from `--join` stdin. Empty = no join.
    pub joins: Vec<JoinSpec>,
    /// Prefix lengths for `--truncate-content`.
    pub truncate_specs: Vec<(String, usize)>,
    /// Redact secret-shaped values (`--redact`).
    pub redact: bool,
    /// NFC-normalize string values (`--normalize-encoding`).
    pub normalize_encoding: bool,
    /// Compare the stream against a baseline NDJSON file (`--diff-against`).
    pub diff_against: Option<std::path::PathBuf>,
    /// Bytes above which streamed string values are truncated (`--max-output-bytes`).
    pub max_output_bytes: Option<usize>,
}

impl OutputShape {
    /// True when writer may serialize directly without intermediate `Value`.
    #[inline]
    pub fn is_identity(&self) -> bool {
        self.profile == OutputProfile::Full
            && self.select.is_none()
            && self.filters.is_empty()
            && self.limit.is_none()
            && self.sort.is_none()
            && self.dedupe_keys.is_empty()
            && self.aggregates.is_empty()
            && self.joins.is_empty()
            && self.truncate_specs.is_empty()
            && !self.redact
            && !self.normalize_encoding
            && self.diff_against.is_none()
            && self.max_output_bytes.is_none()
    }

    /// True when events must be buffered for sort, dedupe, aggregate or diff.
    #[inline]
    pub fn is_buffering(&self) -> bool {
        self.sort.is_some()
            || !self.dedupe_keys.is_empty()
            || !self.aggregates.is_empty()
            || self.diff_against.is_some()
    }

    /// True when `flush` has work that depends on the whole event stream.
    #[inline]
    pub fn is_tail_flush(&self) -> bool {
        self.is_buffering()
    }

    /// Build a shape from the flat CLI values (v0.1.36 signature, kept for
    /// callers that only surface `--select` / `--filter` / `--limit`).
    ///
    /// # Errors
    ///
    /// Returns `AtomwriteError::InvalidInput` when a `--filter` value has no
    /// `=`, `==` or `!=` separator, when the key before the separator is
    /// empty, or when the expression is empty.
    pub fn from_cli(
        select: Option<&str>,
        filters: &[String],
        limit: Option<u64>,
    ) -> Result<Self, crate::error::AtomwriteError> {
        Ok(Self {
            select: parse_select(select),
            filters: parse_filters(filters)?,
            limit,
            ..Self::default()
        })
    }

    /// Build the shape from the parsed global CLI arguments.
    ///
    /// Thin adapter: it only translates the CLI type into [`ShapeOptions`], so
    /// the caller keeps one construction point while the engine still reads a
    /// named description. Adding a flag means adding one field to
    /// [`ShapeOptions::from_global_args`], never editing this signature.
    ///
    /// # Errors
    ///
    /// Returns an error when an operation expression is malformed.
    pub fn from_global_args(
        global: &crate::cli::GlobalArgs,
    ) -> Result<Self, crate::error::AtomwriteError> {
        Self::from_options(ShapeOptions::from_global_args(global))
    }

    /// Build the shape from a named description of the requested operations.
    ///
    /// Every field says what it does, so the caller reads the wire without
    /// counting positional arguments and the engine reads each operation from
    /// its own name. Adding an operation means adding a field here and one line
    /// in the caller, not editing a signature that every call site shares.
    ///
    /// # Errors
    ///
    /// Returns an error when an expression is malformed: bad `--filter`,
    /// empty `--sort-by` key, unknown `--aggregate` op, malformed `--join`
    /// spec, or non-numeric `--truncate-content` length.
    pub fn from_options(
        options: ShapeOptions,
    ) -> Result<Self, crate::error::AtomwriteError> {
        let mut dedupe_keys: Vec<String> = Vec::new();
        for raw in &options.dedupe_keys {
            for key in split_keys(raw) {
                if !dedupe_keys.contains(&key) {
                    dedupe_keys.push(key);
                }
            }
        }
        let mut aggregates = Vec::new();
        for raw in &options.aggregate_exprs {
            aggregates.push(parse_aggregate_expr(raw)?);
        }
        let mut joins = Vec::new();
        for raw in &options.join_exprs {
            joins.push(parse_join_expr(raw)?);
        }
        let mut truncate_specs = Vec::new();
        for raw in &options.truncate_exprs {
            truncate_specs.push(parse_truncate_expr(raw)?);
        }
        let sort_key = options
            .sort_key
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let sort = match sort_key.as_deref() {
            None => None,
            Some("path") | Some("path-line") => Some(SortKey::PathLine),
            Some("line") | Some("line-path") => Some(SortKey::LinePath),
            Some(_) => Some(SortKey::Key),
        };
        Ok(Self {
            select: parse_select(options.select.as_deref()),
            filters: parse_filters(&options.filters)?,
            limit: options.limit,
            sort_key,
            sort,
            dedupe_keys,
            aggregates,
            joins,
            truncate_specs,
            redact: options.redact_secrets,
            normalize_encoding: options.normalize_encoding,
            diff_against: options.diff_against.clone(),
            max_output_bytes: options.max_output_bytes,
            profile: options.profile,
            verbose: options.verbose,
        })
    }

    /// Replace the output profile resolved from configuration.
    #[must_use]
    pub fn with_profile(mut self, profile: OutputProfile) -> Self {
        self.profile = profile;
        self
    }
}

/// Named description of the universal data operations to apply.
///
/// This is the wire between the command line and the engine. Fields are named
/// for the operation, not for the flag spelling, so the engine never depends on
/// how a flag is called. Every field defaults to "operation not requested".
#[derive(Debug, Clone, Default)]
pub struct ShapeOptions {
    /// Requested output profile.
    pub profile: OutputProfile,
    /// Keep the diagnostic keys on a failure envelope.
    pub verbose: bool,
    /// Top-level keys to keep. `None` = no projection.
    pub select: Option<String>,
    /// `key=value` expressions, AND-combined.
    pub filters: Vec<String>,
    /// Max events after filtering.
    pub limit: Option<u64>,
    /// Sort key selector: a top-level key, `path-line`, or `line-path`.
    pub sort_key: Option<String>,
    /// CSV dedupe key tuples, one entry per `--dedupe` occurrence.
    pub dedupe_keys: Vec<String>,
    /// `KEY:OP[:FIELD]` reductions.
    pub aggregate_exprs: Vec<String>,
    /// `KEY:SUFFIX` table joins read from stdin.
    pub join_exprs: Vec<String>,
    /// `KEY:LEN` character-prefix windows.
    pub truncate_exprs: Vec<String>,
    /// Apply the content-shaped secret rules on top of the always-on key floor.
    pub redact_secrets: bool,
    /// Normalize every string value to NFC.
    pub normalize_encoding: bool,
    /// Baseline NDJSON path for the stream comparison.
    pub diff_against: Option<std::path::PathBuf>,
    /// Byte cap for every string value.
    pub max_output_bytes: Option<usize>,
}

impl ShapeOptions {
    /// Describe the operations from the parsed global CLI arguments.
    ///
    /// Kept here — not in the engine — so the engine stays free of the CLI
    /// type. A new global flag is wired in this one place.
    pub fn from_global_args(global: &crate::cli::GlobalArgs) -> Self {
        Self {
            select: global.select.clone(),
            filters: global.filter.clone(),
            limit: global.limit,
            sort_key: global.sort_by.clone(),
            dedupe_keys: global.dedupe.clone(),
            aggregate_exprs: global.aggregate.clone(),
            join_exprs: global.join.clone(),
            truncate_exprs: global.truncate_content.clone(),
            redact_secrets: global.redact,
            normalize_encoding: global.normalize_encoding,
            diff_against: global.diff_against.clone(),
            max_output_bytes: global.max_output_bytes,
            profile: OutputProfile::Full,
            verbose: global.verbose > 0,
        }
    }

    /// Build the legacy `--select` / `--filter` / `--limit` trio only.
    ///
    /// Used by the paths that surface no universal operation. Kept as a named
    /// description for the same reason as the full constructor: the caller
    /// should not have to count positional arguments.
    pub fn from_trio(select: Option<&str>, filters: &[String], limit: Option<u64>) -> Self {
        Self {
            select: select.map(str::to_string),
            filters: filters.to_vec(),
            limit,
            ..Self::default()
        }
    }
}

fn parse_select(select: Option<&str>) -> Option<Vec<String>> {
    let csv = select.map(str::trim).filter(|s| !s.is_empty())?;
    let keys: Vec<String> = split_keys(csv);
    if keys.is_empty() { None } else { Some(keys) }
}

fn split_keys(csv: &str) -> Vec<String> {
    csv.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_filters(filters: &[String]) -> Result<Vec<FilterPred>, crate::error::AtomwriteError> {
    let mut preds = Vec::with_capacity(filters.len());
    for f in filters {
        preds.push(parse_filter_expr(f)?);
    }
    Ok(preds)
}

/// Parse `KEY:OP[:FIELD]` into an [`AggSpec`].
///
/// # Errors
///
/// Returns an error when the group key is empty or the op is unknown.
pub fn parse_aggregate_expr(raw: &str) -> Result<AggSpec, crate::error::AtomwriteError> {
    let parts: Vec<&str> = raw.split(':').collect();
    let group = parts.first().map(|s| s.trim()).unwrap_or("");
    if group.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid --aggregate {raw:?}: expected key:op[:field] with non-empty key"),
        });
    }
    let op = parts.get(1).map(|s| s.trim()).unwrap_or("count");
    let field = parts.get(2).map(|s| s.trim()).filter(|s| !s.is_empty());
    let op = match op {
        "" | "count" => AggOp::Count,
        "sum" | "min" | "max" => {
            let Some(field) = field else {
                return Err(crate::error::AtomwriteError::InvalidInput {
                    reason: format!("invalid --aggregate {raw:?}: {op} requires a field"),
                });
            };
            match op {
                "sum" => AggOp::Sum(field.to_string()),
                "min" => AggOp::Min(field.to_string()),
                _ => AggOp::Max(field.to_string()),
            }
        }
        other => {
            return Err(crate::error::AtomwriteError::InvalidInput {
                reason: format!(
                    "invalid --aggregate {raw:?}: unknown op {other:?} (expected count, sum, min, max)"
                ),
            });
        }
    };
    Ok(AggSpec {
        group: group.to_string(),
        op,
    })
}

/// Parse `KEY:SUFFIX` into a [`JoinSpec`].
///
/// # Errors
///
/// Returns an error when the key or the suffix is empty.
pub fn parse_join_expr(raw: &str) -> Result<JoinSpec, crate::error::AtomwriteError> {
    let Some((key, suffix)) = raw.split_once(':') else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid --join {raw:?}: expected key:suffix"),
        });
    };
    let (key, suffix) = (key.trim(), suffix.trim());
    if key.is_empty() || suffix.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid --join {raw:?}: key and suffix must not be empty"),
        });
    }
    Ok(JoinSpec {
        key: key.to_string(),
        suffix: suffix.to_string(),
    })
}

/// Parse `KEY:LEN` into a truncate pair.
///
/// # Errors
///
/// Returns an error when the key is empty or `LEN` is not a positive integer.
pub fn parse_truncate_expr(raw: &str) -> Result<(String, usize), crate::error::AtomwriteError> {
    let Some((key, len)) = raw.split_once(':') else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid --truncate-content {raw:?}: expected key:len"),
        });
    };
    let key = key.trim();
    if key.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid --truncate-content {raw:?}: empty key"),
        });
    }
    let len: usize = len.trim().parse().map_err(|_| crate::error::AtomwriteError::InvalidInput {
        reason: format!("invalid --truncate-content {raw:?}: len must be a positive integer"),
    })?;
    if len == 0 {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid --truncate-content {raw:?}: len must be greater than zero"),
        });
    }
    Ok((key.to_string(), len))
}

/// Parse `key=value`, `key==value`, or `key!=value` (top-level only).
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` when the expression is empty, when
/// it has no `=`, `==` or `!=` separator, or when the key before the
/// separator is empty.
pub fn parse_filter_expr(raw: &str) -> Result<FilterPred, crate::error::AtomwriteError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "empty --filter expression".into(),
        });
    }
    let (key, val_raw, not_equal) = if let Some(rest) = s.split_once("!=") {
        (rest.0, rest.1, true)
    } else if let Some(rest) = s.split_once("==") {
        (rest.0, rest.1, false)
    } else if let Some(rest) = s.split_once('=') {
        (rest.0, rest.1, false)
    } else {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!(
                "invalid --filter {raw:?}: expected key=value, key==value, or key!=value"
            ),
        });
    };
    let key = key.trim();
    if key.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: format!("invalid --filter {raw:?}: empty key"),
        });
    }
    let value = parse_filter_value(val_raw.trim());
    Ok(FilterPred {
        key: key.to_string(),
        value,
        not_equal,
    })
}

fn parse_filter_value(raw: &str) -> Value {
    match raw {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        s if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 => {
            Value::String(s[1..s.len() - 1].to_string())
        }
        s => {
            if let Ok(n) = s.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(n) = s.parse::<u64>() {
                Value::Number(n.into())
            } else if let Ok(n) = s.parse::<f64>() {
                serde_json::Number::from_f64(n)
                    .map(Value::Number)
                    .unwrap_or_else(|| Value::String(s.to_string()))
            } else {
                Value::String(s.to_string())
            }
        }
    }
}
