// SPDX-License-Identifier: MIT OR Apache-2.0
// The single typed key table of the `config` verb (v0.1.37, T6a).
// Included from config_cmd.rs.
//
// `list`, `get`, `set` and `unset` all read this table; no key is inferred
// from a section struct or guessed from a value.

/// Wire type of a key, with the range it accepts.
#[derive(Clone, Copy)]
enum Kind {
    /// Whole number in `min..=max` (never above `i64::MAX`, the TOML limit).
    Integer { min: u64, max: u64 },
    /// Finite number in `min..=max`.
    Float { min: f64, max: f64 },
    /// `true` or `false`.
    Boolean,
    /// One of a closed set of words.
    Choice(&'static [&'static str]),
    /// Absolute filesystem path.
    AbsolutePath,
    /// UI locale tag, stored in the `locale` preference file.
    Locale,
    /// Ordered list of non-empty strings, stored as a TOML array.
    List,
}

/// Where the value of a key lives.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Store {
    /// `[section] field` of the scope's config file.
    Toml,
    /// `[section] field` of the bootstrap file at the `ProjectDirs` location.
    ///
    /// `storage.home` relocates the config directory, so it cannot live in the
    /// file that the relocation moves.
    Bootstrap,
    /// The `locale` preference file next to the global config.
    LocaleFile,
}

/// Scopes a key may be written in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Scopes {
    /// Global and workspace.
    Both,
    /// Global only: read before any workspace file is known.
    GlobalOnly,
}

/// One row of the key table.
struct KeySpec {
    /// Config section (`""` for a top-level key).
    section: &'static str,
    /// Field inside the section.
    field: &'static str,
    kind: Kind,
    store: Store,
    scopes: Scopes,
    /// Built-in value, taken from the same defaults the product runs with.
    default: fn(&AtomwriteConfig) -> serde_json::Value,
}

impl KeySpec {
    /// Dotted key as typed on the command line.
    fn name(&self) -> String {
        if self.section.is_empty() {
            self.field.to_string()
        } else {
            format!("{}.{}", self.section, self.field)
        }
    }
}

include!("config_help.inc.rs");

/// Upper bound of an integer key: the largest TOML integer.
const INT_MAX: u64 = i64::MAX as u64;
/// Unit interval for similarity thresholds.
const UNIT: Kind = Kind::Float { min: 0.0, max: 1.0 };
/// Any positive integer.
const POSITIVE: Kind = Kind::Integer {
    min: 1,
    max: INT_MAX,
};

/// Integer key bounded by `min..=max`.
const fn between(min: u64, max: u64) -> Kind {
    Kind::Integer { min, max }
}

/// A key stored in the TOML file of its scope.
const fn toml_key(
    section: &'static str,
    field: &'static str,
    kind: Kind,
    scopes: Scopes,
    default: fn(&AtomwriteConfig) -> serde_json::Value,
) -> KeySpec {
    KeySpec {
        section,
        field,
        kind,
        store: Store::Toml,
        scopes,
        default,
    }
}

/// Every key the `config` verb exposes, in listing order.
///
/// No key is listed here unless a verb reads it: `defaults.line_ending` is the
/// fallback of `write --line-ending`, and `defaults.max_filesize` the fallback
/// of `--max-filesize` there. A listed key nothing consumes is a promise the
/// product would not keep.
static KEYS: &[KeySpec] = &[
    toml_key(
        "defaults",
        "line_ending",
        Kind::Choice(&["auto", "lf", "cr-lf", "cr"]),
        Scopes::Both,
        |c| serde_json::json!(c.defaults.line_ending),
    ),
    toml_key("defaults", "backup", Kind::Boolean, Scopes::Both, |c| {
        serde_json::json!(c.defaults.backup)
    }),
    toml_key("delete", "backup", Kind::Boolean, Scopes::Both, |c| {
        serde_json::json!(c.delete.backup)
    }),
    toml_key(
        "defaults",
        "retention",
        // The stored type is `u8`, so the ceiling is the type's own maximum.
        between(0, u8::MAX as u64),
        Scopes::Both,
        |c| serde_json::json!(c.defaults.retention),
    ),
    toml_key("defaults", "max_filesize", POSITIVE, Scopes::Both, |c| {
        serde_json::json!(c.defaults.max_filesize)
    }),
    toml_key(
        "fuzzy",
        "mode",
        Kind::Choice(&["auto", "aggressive", "off"]),
        Scopes::Both,
        |c| serde_json::json!(c.fuzzy.mode),
    ),
    toml_key("fuzzy", "threshold", UNIT, Scopes::Both, |c| {
        serde_json::json!(c.fuzzy.threshold)
    }),
    toml_key("fuzzy", "threshold_aggressive", UNIT, Scopes::Both, |c| {
        serde_json::json!(c.fuzzy.threshold_aggressive)
    }),
    toml_key("fuzzy", "threshold_context", UNIT, Scopes::Both, |c| {
        serde_json::json!(c.fuzzy.threshold_context)
    }),
    toml_key("fuzzy", "threshold_jw", UNIT, Scopes::Both, |c| {
        serde_json::json!(c.fuzzy.threshold_jw)
    }),
    toml_key("fuzzy", "soft_floor_delta", UNIT, Scopes::Both, |c| {
        serde_json::json!(c.fuzzy.soft_floor_delta)
    }),
    toml_key("fuzzy", "max_content_bytes", POSITIVE, Scopes::Both, |c| {
        serde_json::json!(c.fuzzy.max_content_bytes)
    }),
    toml_key(
        "apply",
        "max_ellipsis_lines",
        POSITIVE,
        Scopes::Both,
        |c| serde_json::json!(c.apply.max_ellipsis_lines),
    ),
    toml_key(
        "write",
        "confirm_large_bytes",
        POSITIVE,
        Scopes::Both,
        |c| serde_json::json!(c.write.confirm_large_bytes),
    ),
    toml_key(
        "write",
        "shrink_block_percent",
        between(
            crate::constants::SHRINK_BLOCK_PERCENT_MIN as u64,
            crate::constants::SHRINK_BLOCK_PERCENT_MAX as u64,
        ),
        Scopes::Both,
        |c| serde_json::json!(c.write.shrink_block_percent),
    ),
    toml_key(
        "write",
        "auto_rotate_max_age_secs",
        POSITIVE,
        Scopes::Both,
        |c| serde_json::json!(c.write.auto_rotate_max_age_secs),
    ),
    // The empty list is not a value of its own: `assess_content_risk` reads it
    // as "use the built-in patterns", so the default published here is the
    // built-in set, which is what the run actually applies.
    toml_key(
        "write",
        "content_risk_patterns",
        Kind::List,
        Scopes::Both,
        |c| {
            let configured = &c.write.content_risk_patterns;
            if configured.is_empty() {
                serde_json::json!(crate::constants::WRITE_CONTENT_RISK_PATTERNS)
            } else {
                serde_json::json!(configured)
            }
        },
    ),
    toml_key("watch", "idle_exit_ms", between(0, INT_MAX), Scopes::Both, |c| {
        serde_json::json!(c.watch.idle_exit_ms)
    }),
    toml_key(
        "watch",
        "debounce_ms",
        // `watch` clamps the effective value with `.max(1)`, so 1 is the floor
        // it enforces; 0 would be raised to 1 on use and would not round-trip.
        between(crate::constants::WATCH_DEBOUNCE_FLOOR_MS, INT_MAX),
        Scopes::Both,
        |c| serde_json::json!(c.watch.debounce_ms),
    ),
    // Both scopes: the workspace `.atomwrite.toml` now reaches the log sink,
    // because `startup_log_section` merges both layers. Rejecting the write
    // here while honouring the key there would be the same silent split this
    // change removed.
    toml_key("log", "file", Kind::Boolean, Scopes::Both, |c| {
        serde_json::json!(c.log.file)
    }),
    toml_key(
        "log",
        "rotation",
        Kind::Choice(&["daily", "hourly"]),
        Scopes::Both,
        |c| serde_json::json!(c.log.rotation),
    ),
    KeySpec {
        section: "storage",
        field: "home",
        kind: Kind::AbsolutePath,
        store: Store::Bootstrap,
        scopes: Scopes::GlobalOnly,
        default: |c| serde_json::json!(c.storage.home),
    },
    KeySpec {
        section: "",
        field: "locale",
        kind: Kind::Locale,
        store: Store::LocaleFile,
        scopes: Scopes::GlobalOnly,
        default: |_| serde_json::Value::Null,
    },
    toml_key(
        "concurrency",
        "max_threads",
        between(
            crate::constants::CONCURRENCY_MIN_THREADS,
            crate::constants::CONCURRENCY_MAX_THREADS_CEILING,
        ),
        Scopes::Both,
        |c| serde_json::json!(c.concurrency.max_threads),
    ),
    toml_key(
        "concurrency",
        "ram_fraction_percent",
        between(
            crate::constants::RAM_FRACTION_PERCENT_MIN,
            crate::constants::PERCENT_MAX,
        ),
        Scopes::Both,
        |c| serde_json::json!(c.concurrency.ram_fraction_percent),
    ),
    toml_key("search", "context", between(0, INT_MAX), Scopes::Both, |c| {
        serde_json::json!(c.search.context)
    }),
    toml_key(
        "search",
        "smart_case",
        Kind::Boolean,
        Scopes::Both,
        |c| serde_json::json!(c.search.smart_case),
    ),
    toml_key("search", "max_filesize", POSITIVE, Scopes::Both, |c| {
        serde_json::json!(c.search.max_filesize)
    }),
    toml_key("search", "max_columns", POSITIVE, Scopes::Both, |c| {
        serde_json::json!(c.search.max_columns)
    }),
    toml_key(
        "search",
        "max_inflight_bytes",
        between(
            crate::constants::SEARCH_MIN_INFLIGHT_BYTES,
            crate::constants::SEARCH_MAX_INFLIGHT_CEILING_BYTES,
        ),
        Scopes::Both,
        |c| serde_json::json!(c.search.max_inflight_bytes),
    ),
    toml_key(
        "search",
        "format",
        Kind::Choice(&["ndjson", "text"]),
        Scopes::Both,
        |c| serde_json::json!(c.search.format),
    ),
    toml_key(
        "read",
        "max_output_bytes",
        POSITIVE,
        Scopes::Both,
        |c| serde_json::json!(c.read.max_output_bytes),
    ),
    toml_key("read", "number", Kind::Boolean, Scopes::Both, |c| {
        serde_json::json!(c.read.number)
    }),
    toml_key(
        "output",
        "profile",
        Kind::Choice(&["full", "lean"]),
        Scopes::Both,
        |c| serde_json::json!(c.output.profile),
    ),
    toml_key(
        "summary",
        "errors_max_entries",
        between(
            crate::constants::SUMMARY_ERRORS_MIN_ENTRIES as u64,
            crate::constants::SUMMARY_ERRORS_CEILING as u64,
        ),
        Scopes::Both,
        |c| serde_json::json!(c.summary.errors_max_entries),
    ),
    toml_key(
        "lock",
        "acquire_timeout_ms",
        between(
            crate::constants::LOCK_MIN_ACQUIRE_TIMEOUT_MS,
            crate::constants::LOCK_MAX_ACQUIRE_TIMEOUT_MS,
        ),
        Scopes::Both,
        |c| serde_json::json!(c.lock.acquire_timeout_ms),
    ),
    toml_key(
        "retry",
        "base_ms",
        between(
            crate::constants::RETRY_MIN_MS,
            crate::constants::RETRY_MAX_WAIT_CEILING_MS,
        ),
        Scopes::Both,
        |_| serde_json::json!(crate::constants::LOCK_RETRY_BASE_MS),
    ),
    toml_key(
        "retry",
        "max_ms",
        between(
            crate::constants::RETRY_MIN_MS,
            crate::constants::RETRY_MAX_WAIT_CEILING_MS,
        ),
        Scopes::Both,
        |_| serde_json::json!(crate::constants::LOCK_RETRY_CAP_MS),
    ),
    toml_key(
        "retry",
        "max_elapsed_ms",
        between(
            crate::constants::RETRY_MIN_MS,
            crate::constants::RETRY_MAX_ELAPSED_CEILING_MS,
        ),
        Scopes::Both,
        |_| serde_json::json!(crate::constants::PERSIST_RETRY_MAX_ELAPSED_MS),
    ),
    toml_key(
        "extract",
        "max_sample_rows",
        between(0, crate::constants::EXTRACT_MAX_SAMPLE_ROWS_CEILING),
        Scopes::Both,
        |c| serde_json::json!(c.extract.max_sample_rows),
    ),
    toml_key(
        "extract",
        "max_text_chars",
        between(1, crate::constants::EXTRACT_MAX_TEXT_CHARS_CEILING),
        Scopes::Both,
        |c| serde_json::json!(c.extract.max_text_chars),
    ),
    toml_key(
        "extract",
        "max_sheets",
        between(1, crate::constants::EXTRACT_MAX_PARTS_CEILING),
        Scopes::Both,
        |c| serde_json::json!(c.extract.max_sheets),
    ),
    toml_key(
        "extract",
        "max_slides",
        between(1, crate::constants::EXTRACT_MAX_PARTS_CEILING),
        Scopes::Both,
        |c| serde_json::json!(c.extract.max_slides),
    ),
];

/// Row of the table for a dotted key, or the exit-65 error listing every key.
fn find_key(key: &str) -> Result<&'static KeySpec, AtomwriteError> {
    KEYS.iter()
        .find(|spec| spec.name() == key)
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: format!(
                "unknown config key '{key}'; valid keys: {}",
                KEYS.iter()
                    .map(KeySpec::name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
}

/// A value accepted for a key: the text TOML stores and its JSON form.
struct Parsed {
    toml_text: String,
    json: serde_json::Value,
}

/// A number outside the range of its key; the reason carries the marker that
/// selects the localized hint.
fn out_of_range(key: &str, raw: &str, range: &str) -> AtomwriteError {
    AtomwriteError::InvalidInput {
        reason: format!(
            "config key '{key}' = {raw} {} {range}",
            crate::constants::CONFIG_OUT_OF_RANGE_REASON_MARKER
        ),
    }
}

fn bad_value(key: &str, expected: &str, raw: &str) -> AtomwriteError {
    AtomwriteError::InvalidInput {
        reason: format!("config key '{key}' expects {expected}, got '{raw}'"),
    }
}

impl Kind {
    /// Wire name of the type, as `config list` reports it.
    fn label(&self) -> &'static str {
        match self {
            Self::Integer { .. } => "integer",
            Self::Float { .. } => "float",
            Self::Boolean => "boolean",
            Self::Choice(_) | Self::AbsolutePath | Self::Locale => "string",
            Self::List => "list",
        }
    }

    /// Check `raw` against the type and range, and normalise it for TOML.
    fn parse(&self, key: &str, raw: &str) -> Result<Parsed, AtomwriteError> {
        let raw = raw.trim();
        match *self {
            Self::Integer { min, max } => {
                let n: i128 = raw
                    .parse()
                    .map_err(|_| bad_value(key, "a whole number", raw))?;
                let n = u64::try_from(n)
                    .ok()
                    .filter(|n| (min..=max).contains(n))
                    .ok_or_else(|| out_of_range(key, raw, &format!("{min}..={max}")))?;
                Ok(Parsed {
                    toml_text: n.to_string(),
                    json: serde_json::json!(n),
                })
            }
            Self::Float { min, max } => {
                let n: f64 = raw
                    .parse()
                    .ok()
                    .filter(|n: &f64| n.is_finite())
                    .ok_or_else(|| bad_value(key, "a finite number", raw))?;
                if !(min..=max).contains(&n) {
                    return Err(out_of_range(key, raw, &format!("{min}..={max}")));
                }
                Ok(Parsed {
                    toml_text: format!("{n:?}"),
                    json: serde_json::json!(n),
                })
            }
            Self::Boolean => match raw {
                "true" | "false" => Ok(Parsed {
                    toml_text: raw.to_string(),
                    json: serde_json::json!(raw == "true"),
                }),
                _ => Err(bad_value(key, "true or false", raw)),
            },
            Self::Choice(words) => {
                if words.contains(&raw) {
                    Ok(Parsed {
                        toml_text: raw.to_string(),
                        json: serde_json::json!(raw),
                    })
                } else {
                    Err(bad_value(key, &format!("one of {}", words.join(", ")), raw))
                }
            }
            Self::AbsolutePath => {
                if Path::new(raw).is_absolute() {
                    Ok(Parsed {
                        toml_text: raw.to_string(),
                        json: serde_json::json!(raw),
                    })
                } else {
                    Err(bad_value(key, "an absolute path", raw))
                }
            }
            Self::Locale => crate::locale::Language::from_tag(raw)
                .map(|language| Parsed {
                    toml_text: language.as_str().to_string(),
                    json: serde_json::json!(language.as_str()),
                })
                .ok_or_else(|| bad_value(key, "a supported locale (en, pt-BR)", raw)),
            Self::List => parse_list(key, raw),
        }
    }
}

include!("config_list_parse.inc.rs");
