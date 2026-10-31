// SPDX-License-Identifier: MIT OR Apache-2.0
// Global flags shared across all subcommands.
// Included from cli.rs.

#[derive(clap::Args, Debug)]
/// Global flags shared across all subcommands.
pub struct GlobalArgs {
    /// Verbosity level (repeat for more: -v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true, help = "Increase verbosity (-v info, -vv debug, -vvv trace)")]
    pub verbose: u8,

    /// Quiet level (repeat for less: -q, -qq).
    #[arg(short, long, action = clap::ArgAction::Count, global = true, help = "Decrease verbosity (-q error, -qq off)")]
    pub quiet: u8,

    /// Workspace root for path jail validation.
    #[arg(
        long,
        global = true,
        value_hint = ValueHint::DirPath,
        help = "Workspace root for path jail validation"
    )]
    pub workspace: Option<PathBuf>,

    /// Explicit config file path (overrides workspace `.atomwrite.toml` and XDG config).
    #[arg(
        long,
        global = true,
        value_hint = ValueHint::FilePath,
        help = "Path to .atomwrite.toml (explicit override; fails if missing/malformed)"
    )]
    pub config: Option<PathBuf>,

    /// Color output mode.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto, help = "Control colored output")]
    pub color: ColorChoice,

    /// Disable colored output (equivalent to --color never).
    #[arg(long, global = true, help = "Disable colored output", action = clap::ArgAction::SetTrue)]
    pub no_color: bool,

    /// Write the process ID to this path once the signal handlers are installed.
    ///
    /// Meant for test harnesses that must know when the process is ready to
    /// receive signals. One-shot agents normally omit it.
    #[arg(
        long = "ready-file",
        global = true,
        value_hint = ValueHint::FilePath,
        help = "Write the PID after signal handlers are ready (test harness use)"
    )]
    pub ready_file: Option<PathBuf>,

    /// Disable .gitignore filtering.
    #[arg(long, global = true, help = "Do not respect .gitignore files", action = clap::ArgAction::SetTrue)]
    pub no_gitignore: bool,

    /// Include hidden files and directories.
    #[arg(long, global = true, help = "Include hidden files and directories", action = clap::ArgAction::SetTrue)]
    pub hidden: bool,

    /// Follow symbolic links during traversal.
    #[arg(long, global = true, help = "Follow symbolic links", action = clap::ArgAction::SetTrue)]
    pub follow_symlinks: bool,

    /// Maximum number of parallel workers for multi-file commands.
    ///
    /// The bound covers the directory walk and the shared worker pool used by
    /// hashing, backup, recursive delete and copy, semantic-search scoring and
    /// non-transactional batch. Omitted or `0` means automatic: all logical
    /// CPUs up to a built-in ceiling of 16 workers, capped by available RAM.
    /// `N` means exactly N workers, also above that ceiling.
    ///
    /// Alias: `--max-concurrency`. The bound comes from this flag only, never
    /// from a process environment variable.
    #[arg(
        short = 'j',
        long = "threads",
        visible_alias = "max-concurrency",
        global = true,
        value_name = "N",
        help = "Maximum parallel workers (0 or omitted = automatic: all cores up to 16, capped by RAM; N = exactly N)"
    )]
    pub threads: Option<usize>,

    /// Maximum allowed file size in bytes.
    #[arg(
        long,
        global = true,
        help = "Maximum file size in bytes (default: 1 GiB, larger files are rejected)"
    )]
    pub max_filesize: Option<u64>,

    /// Global operation timeout in seconds.
    ///
    /// When non-zero, a watchdog cancels the operation after N seconds and the
    /// command exits with code 124. Pass `0` to disable the timeout, which is
    /// not recommended for agents. Alias: `--timeout`.
    #[arg(
        long = "timeout-secs",
        visible_alias = "timeout",
        global = true,
        default_value_t = crate::constants::DEFAULT_TIMEOUT_SECS,
        help = "Global operation timeout in seconds (0 disables, exit 124 on deadline)"
    )]
    pub timeout_secs: u64,

    /// Suppress NDJSON `progress` heartbeats (batch/replace).
    #[arg(
        long,
        global = true,
        help = "Disable NDJSON progress events on long operations",
        action = clap::ArgAction::SetTrue
    )]
    pub no_progress: bool,

    /// Emit JSON Schema for subcommand output and exit.
    #[arg(
        long,
        global = true,
        help = "Emit JSON Schema for the subcommand output and exit",
        action = clap::ArgAction::SetTrue
    )]
    pub json_schema: bool,

    /// Discover the NDJSON schema surface, or emit one schema.
    ///
    /// Without a value it lists every schema name as one NDJSON line per name.
    /// With `NAME` it emits that schema on the data channel: `read`, `edit`,
    /// `stream_diff`, `error`, and so on. An unknown name fails closed with a
    /// typed error instead of emitting an empty document.
    #[arg(
        long = "print-schema",
        global = true,
        value_name = "NAME",
        num_args = 0..=1,
        default_missing_value = "",
        help = "List schema names, or emit the schema for NAME, on stdout"
    )]
    pub print_schema: Option<String>,

    /// Declare agent-hosted (non-interactive) execution.
    ///
    /// The process never reads a confirmation from stdin and never tries to
    /// open a terminal prompt; a command that would need one fails with a
    /// typed error instead of blocking. Not required for ordinary one-shot
    /// runs — `/dev/null` stdin already behaves this way.
    #[arg(
        long = "non-interactive",
        visible_alias = "no-input",
        global = true,
        help = "Declare non-interactive execution: never prompt, never read confirmation from stdin",
        action = clap::ArgAction::SetTrue
    )]
    pub non_interactive: bool,

    /// Acknowledge the side effect of the command before it runs.
    ///
    /// Only the commands whose contract requires an acknowledgement (see
    /// `commands --include-globals`) accept it. Any other command rejects the
    /// flag with a typed error rather than treating it as a no-op.
    #[arg(
        long = "confirm-side-effect",
        global = true,
        help = "Acknowledge the command side effect (required by contract-gated commands)",
        action = clap::ArgAction::SetTrue
    )]
    pub confirm_side_effect: bool,

    /// Opt in to the value window used by `--max-output-bytes`.
    ///
    /// Truncation is opt-in: the flag is rejected when
    /// `--max-output-bytes` is absent, so a truncated payload is never the
    /// default.
    #[arg(
        long = "allow-truncate",
        global = true,
        requires = "max_output_bytes",
        help = "Opt in to value truncation; required by --max-output-bytes",
        action = clap::ArgAction::SetTrue
    )]
    pub allow_truncate: bool,

    /// Accepted for compatibility but ignored — output is always NDJSON.
    #[arg(long, global = true, hide = true, action = clap::ArgAction::SetTrue)]
    pub json: bool,

    /// Override the UI locale for translated suggestions and human stderr (`en`, `pt-BR`).
    ///
    /// Precedence: `--locale`, then the saved preference (`atomwrite locale
    /// --set`), then the operating system locale, then `en`. Error codes and
    /// messages always stay in English, only the suggestions follow the locale.
    #[arg(
        long = "locale",
        global = true,
        value_parser = crate::locale::parse_cli_locale,
        help = "Override locale (en, pt-BR)"
    )]
    pub lang: Option<String>,

    /// Skip the on-startup `wal-heal` pass.
    ///
    /// By default every command that modifies files first removes stale
    /// completed write-ahead journals older than 3600 seconds from the
    /// directories named on the command line, within a 100 ms wall-clock
    /// budget. Set this flag in tight loops or in benchmarks that measure the
    /// cost of one command in isolation.
    #[arg(
        long,
        global = true,
        help = "Skip the startup wal-heal pass (default: remove journals older than 3600 s within a 100 ms budget)",
        action = clap::ArgAction::SetTrue
    )]
    pub no_auto_heal: bool,

    /// Project NDJSON events to top-level keys only (comma-separated).
    ///
    /// Applied by the stdout writer for every subcommand. Missing keys are
    /// omitted (no null filler). Alias: `--fields`.
    #[arg(
        long = "select",
        visible_alias = "fields",
        global = true,
        value_name = "KEYS",
        help = "Project NDJSON to top-level keys (CSV); alias --fields"
    )]
    pub select: Option<String>,

    /// Keep only NDJSON events matching top-level key equality (repeatable AND).
    ///
    /// Expressions: `key=value`, `key==value`, or `key!=value`. Values may be
    /// `true`/`false`/`null`/number/string. Applies to all events including errors.
    #[arg(
        long = "filter",
        global = true,
        value_name = "EXPR",
        action = clap::ArgAction::Append,
        help = "Filter NDJSON events by top-level key=value (repeatable AND)"
    )]
    pub filter: Vec<String>,

    /// Emit at most N NDJSON events after filtering.
    ///
    /// Long-only: the short `-n` belongs to `replace --max-replacements`.
    #[arg(
        long = "limit",
        global = true,
        value_name = "N",
        help = "Max NDJSON events after --filter (long-only, -n belongs to replace)"
    )]
    pub limit: Option<u64>,

    /// Order buffered NDJSON events before they reach the data channel.
    ///
    /// `KEY` sorts by that top-level key, `path-line` by `path` then `line`,
    /// `line-path` by `line` then `path`. Missing keys sort last, and every
    /// comparison falls back to arrival order, so the ordering is total.
    /// Implies buffering: events are emitted on flush, not as they arrive.
    ///
    /// Not to be confused with the local `search --sort`.
    // The flag is `--sort-by`, not `--sort`: `search` already owns a local
    // `--sort`, and clap derives the argument id from the field name, so
    // sharing `sort` with `SearchArgs` would collide.
    #[arg(
        long = "sort-by",
        global = true,
        value_name = "KEY",
        help = "Sort NDJSON events by key, path-line, or line-path (buffers until flush)"
    )]
    pub sort_by: Option<String>,

    /// Keep the first event of each duplicate group and drop the rest.
    ///
    /// `KEYS` is a CSV of top-level keys (or repeat the flag). Distinct from
    /// `--sort-by`: dedupe keeps arrival order of the surviving events unless
    /// a sort flag is also set.
    #[arg(
        long = "dedupe",
        global = true,
        value_name = "KEYS",
        action = clap::ArgAction::Append,
        help = "Drop events whose CSV key tuple was already seen (repeatable)"
    )]
    pub dedupe: Vec<String>,

    /// Collapse buffered events into one summary line per group.
    ///
    /// Expression: `KEY[:OP[:FIELD]]` with `OP` one of `count` (default),
    /// `sum`, `min`, `max`. `FIELD` is the numeric key the reduction reads.
    /// Use with `--sort-by` to also order the surviving groups.
    #[arg(
        long = "aggregate",
        global = true,
        value_name = "KEY:OP[:FIELD]",
        action = clap::ArgAction::Append,
        conflicts_with = "diff_against",
        help = "Group buffered events by key and reduce (repeatable): count, sum, min, max"
    )]
    pub aggregate: Vec<String>,

    /// Attach a table read from stdin to every event, by matching key.
    ///
    /// Expression: `KEY:SUFFIX`. The whole NDJSON stdin is read as the table
    /// (records grouped by `KEY`) and each event gains a top-level `SUFFIX`
    /// array with its matching records. Events with no match pass through
    /// unchanged. Contract: stdin carries the table, stdout carries results.
    #[arg(
        long = "join",
        global = true,
        value_name = "KEY:SUFFIX",
        action = clap::ArgAction::Append,
        help = "Join the NDJSON table on stdin into each event (repeatable): key:suffix"
    )]
    pub join: Vec<String>,

    /// Truncate string values of the given key to N characters.
    ///
    /// Expression: `KEY:LEN`, counted in characters so multibyte text is cut
    /// on a boundary. Non-string keys and shorter values pass through.
    #[arg(
        long = "truncate-content",
        global = true,
        value_name = "KEY:LEN",
        action = clap::ArgAction::Append,
        help = "Truncate a string key to N characters (repeatable): key:len"
    )]
    pub truncate_content: Vec<String>,

    /// Replace secret-shaped values with `[redacted]`.
    ///
    /// Two rules: a top-level key whose name marks a credential always has
    /// its string value replaced, and any string carrying a known credential
    /// prefix is replaced whole. Keeps the agent from echoing tokens onward.
    #[arg(
        long = "redact",
        global = true,
        help = "Replace secret-shaped values with [redacted] before emitting",
        action = clap::ArgAction::SetTrue
    )]
    pub redact: bool,

    /// Normalize every string value to Unicode NFC.
    ///
    /// Use when two sources disagree on the byte form of the same text
    /// (composed vs decomposed accents) and the agent needs one form.
    #[arg(
        long = "normalize-encoding",
        global = true,
        help = "Normalize all string values to Unicode NFC",
        action = clap::ArgAction::SetTrue
    )]
    pub normalize_encoding: bool,

    /// Compare the emitted stream against a baseline NDJSON file.
    ///
    /// On flush, one `stream_diff` line reports `added`, `removed` and
    /// `unchanged` counts by event identity (duplicates counted as a
    /// multiset) instead of the live events. The path must resolve inside the
    /// workspace jail.
    #[arg(
        long = "diff-against",
        global = true,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        conflicts_with = "truncate_content",
        help = "Report added/removed/unchanged against a baseline NDJSON file"
    )]
    pub diff_against: Option<PathBuf>,

    /// Cap the byte length of every string value in the emitted events.
    ///
    /// Cuts on a UTF-8 boundary, never mid-character. It is a value window,
    /// not a line window: numbers and structure stay intact, so an agent still
    /// reads the counts. Opt-in by contract — see `--allow-truncate`.
    #[arg(
        long = "max-output-bytes",
        global = true,
        value_name = "BYTES",
        requires = "allow_truncate",
        help = "Cap the byte length of every string value (needs --allow-truncate)"
    )]
    pub max_output_bytes: Option<usize>,
}
