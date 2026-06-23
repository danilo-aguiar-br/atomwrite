// SPDX-License-Identifier: MIT OR Apache-2.0

//! Grammatical scoping: AST-based code selection and transformation.
//! Workload: mixed I/O-bound + CPU-bound (file reading + AST traversal via ast-grep + atomic write).

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::{Context, Result};
use ast_grep_core::AstGrep;
use ast_grep_core::matcher::Pattern;
use ast_grep_language::SupportLang;
use unicode_normalization::UnicodeNormalization;

use crate::atomic::{AtomicWriteOptions, atomic_write};
use crate::checksum;
use crate::cli::GlobalArgs;
use crate::error::AtomwriteError;
use crate::ndjson_types::{ScopeResult, Summary};
use crate::output::NdjsonWriter;
use crate::signal::ShutdownSignal;

/// Arguments for the scope subcommand.
#[derive(clap::Args, Debug)]
pub struct ScopeArgs {
    /// Paths to search within.
    #[arg(default_value = ".")]
    pub paths: Vec<std::path::PathBuf>,

    /// Source language for AST parsing.
    /// GAP-2026-003 — fixed in v0.1.20 via ADR-0037: the global locale
    /// flag was renamed from `--lang` to `--locale`, freeing the
    /// `--lang` namespace. `--lang` is now a working alias for
    /// `--language`. Both `--lang rust` and `--language rust` are
    /// accepted; use the short form `-l rust` for terse scripts.
    #[arg(
        short = 'l',
        long = "language",
        alias = "lang",
        required = true,
        help = "Language (rust, py, ts, go, c, etc); accepts --lang as alias"
    )]
    pub language: String,

    /// Prepared query name (e.g. comments, strings, fn, pub-fn).
    #[arg(
        long,
        help = "Prepared query name (comments, strings, fn, pub-fn, etc)"
    )]
    pub query: Option<String>,

    /// Custom AST pattern to match (same syntax as transform).
    #[arg(long, help = "Custom AST pattern to match")]
    pub pattern: Option<String>,

    /// Delete matched content.
    #[arg(long, help = "Delete all matched content")]
    pub delete: bool,

    /// Action to apply on matched content.
    #[arg(
        long,
        value_enum,
        help = "Transform action: upper, lower, titlecase, squeeze, symbols, normalize"
    )]
    pub action: Option<ScopeAction>,

    /// Replacement text for matched content.
    #[arg(long, help = "Replace matched content with this text")]
    pub replace_with: Option<String>,

    /// Glob patterns for file inclusion.
    #[arg(short = 'g', long, action = clap::ArgAction::Append, help = "Include files matching glob")]
    pub include: Vec<String>,

    /// Glob patterns for file exclusion.
    #[arg(long, action = clap::ArgAction::Append, help = "Exclude files matching glob")]
    pub exclude: Vec<String>,

    /// Preview without writing.
    #[arg(long, help = "Show what would be done without writing")]
    pub dry_run: bool,

    /// Create backup before modifying.
    #[arg(long, help = "Create backup before modifying")]
    pub backup: bool,
}

/// Available actions for the scope subcommand.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ScopeAction {
    /// Convert to uppercase.
    Upper,
    /// Convert to lowercase.
    Lower,
    /// Convert to title case.
    Titlecase,
    /// Collapse consecutive repeated whitespace.
    Squeeze,
    /// Convert ASCII symbols to Unicode equivalents.
    Symbols,
    /// NFC Unicode normalization.
    Normalize,
}

/// Execute the scope subcommand.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` for unknown language, query, or pattern.
#[tracing::instrument(skip_all, fields(command = "scope"))]
pub fn cmd_scope(
    args: &ScopeArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();

    let lang = parse_language(&args.language)?;
    let pattern_strs = resolve_patterns(&args.query, &args.pattern, &args.language)?;
    let patterns: Vec<Pattern> = pattern_strs
        .iter()
        .map(|ps| {
            Pattern::try_new(ps, lang).map_err(|e| {
                anyhow::anyhow!(AtomwriteError::InvalidInput {
                    reason: format!("invalid scope pattern: {e}"),
                })
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let query_name = args.query.clone().unwrap_or_else(|| "custom".to_owned());

    let workspace = global.resolve_workspace()?;

    let canonical_paths =
        crate::commands::path_resolution::resolve_paths_against_workspace(&args.paths, &workspace)?;
    let mut walker = ignore::WalkBuilder::new(&canonical_paths[0]);
    for p in canonical_paths.iter().skip(1) {
        walker.add(p);
    }
    walker
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore)
        .follow_links(global.follow_symlinks);

    let extensions = crate::lang_utils::lang_extensions(&args.language);
    if !extensions.is_empty() {
        let mut types_builder = ignore::types::TypesBuilder::new();
        for ext in &extensions {
            types_builder
                .add_def(&format!("lang:*.{ext}"))
                .context("invalid extension")?;
        }
        types_builder.select("lang");
        walker.types(types_builder.build().context("build types")?);
    }

    if !args.include.is_empty() || !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&canonical_paths[0]);
        for pat in &args.include {
            overrides.add(pat)?;
        }
        for pat in &args.exclude {
            overrides.add(&format!("!{pat}"))?;
        }
        walker.overrides(overrides.build()?);
    }

    let (tx, rx) = crossbeam_channel::bounded::<ScopeEvent>(1024);

    let files_visited = Arc::new(AtomicU64::new(0));
    let files_modified = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_modified);
    let fs = Arc::clone(&files_skipped);
    let delete = args.delete;
    let action = args.action;
    let replace_with: Option<Arc<str>> = args.replace_with.clone().map(Into::into);
    let dry_run = args.dry_run;
    let backup = args.backup;
    let ws: Arc<std::path::Path> = Arc::from(workspace.as_path());
    let qn: Arc<str> = query_name.into();
    let lang_name: Arc<str> = args.language.clone().into();

    let max_size = global.effective_max_filesize();
    let shutdown_flag = shutdown.flag();
    let patterns = Arc::new(patterns);
    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let patterns = Arc::clone(&patterns);
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let fs = Arc::clone(&fs);
            let replace_with = replace_with.clone();
            let ws = Arc::clone(&ws);
            let qn = Arc::clone(&qn);
            let lang_name = Arc::clone(&lang_name);
            let shutdown_flag = Arc::clone(&shutdown_flag);

            Box::new(move |entry| {
                if shutdown_flag.load(Ordering::Acquire) {
                    return ignore::WalkState::Quit;
                }

                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    return ignore::WalkState::Continue;
                }

                fv.fetch_add(1, Ordering::Relaxed);
                let path = entry.path().to_path_buf();
                let _span = tracing::debug_span!("process_file", path = %path.display()).entered();
                let file_start = Instant::now();

                let content = match crate::file_io::read_file_string(&path, max_size) {
                    Ok(c) => c,
                    Err(_) => {
                        fs.fetch_add(1, Ordering::Relaxed);
                        return ignore::WalkState::Continue;
                    }
                };

                let grep = AstGrep::new(&content, lang);
                let root = grep.root();
                let matches: Vec<_> = patterns.iter().flat_map(|p| root.find_all(p)).collect();

                if matches.is_empty() {
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                let scopes_matched = matches.len() as u64;

                let is_read_only = !delete && action.is_none() && replace_with.is_none();
                if is_read_only {
                    fm.fetch_add(1, Ordering::Relaxed);
                    let checksum = checksum::hash_bytes(content.as_bytes());
                    let _ = tx.send(ScopeEvent::Result(ScopeResult {
                        r#type: "scoped",
                        path: path.display().to_string(),
                        language: lang_name.to_string(),
                        query: qn.to_string(),
                        action: "none".to_owned(),
                        scopes_matched,
                        bytes_before: content.len() as u64,
                        bytes_after: content.len() as u64,
                        checksum_before: checksum.clone(),
                        checksum_after: checksum,
                        elapsed_ms: file_start.elapsed().as_millis() as u64,
                    }));
                    return ignore::WalkState::Continue;
                }

                let mut edits: Vec<(usize, usize, String)> = Vec::with_capacity(matches.len());

                for m in &matches {
                    let range = m.range();
                    let (effective_start, effective_end) = if delete {
                        expand_to_full_line(&content, range.start, range.end)
                    } else {
                        (range.start, range.end)
                    };
                    let matched_text = &content[effective_start..effective_end];
                    let replacement =
                        apply_scope_action(matched_text, delete, action, replace_with.as_deref());
                    edits.push((effective_start, effective_end, replacement.into_owned()));
                }

                edits.sort_by_key(|e| std::cmp::Reverse(e.0));

                let checksum_before = checksum::hash_bytes(content.as_bytes());
                let bytes_before = content.len() as u64;
                let mut content = content; // rebind as mut — no clone
                for (s, e, replacement) in &edits {
                    content.replace_range(*s..*e, replacement);
                }
                let checksum_after = checksum::hash_bytes(content.as_bytes());

                if checksum_before == checksum_after {
                    // O(1) hash comparison instead of O(n) string comparison
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                fm.fetch_add(1, Ordering::Relaxed);

                if !dry_run {
                    let opts = AtomicWriteOptions {
                        backup,
                        ..Default::default()
                    };
                    if let Err(e) = atomic_write(&path, content.as_bytes(), &opts, &ws) {
                        tracing::warn!(path = %path.display(), error = %e, "scope write failed");
                        return ignore::WalkState::Continue;
                    }
                }

                let action_name = if delete {
                    "delete"
                } else if replace_with.is_some() {
                    "replace"
                } else {
                    match action {
                        Some(ScopeAction::Upper) => "upper",
                        Some(ScopeAction::Lower) => "lower",
                        Some(ScopeAction::Titlecase) => "titlecase",
                        Some(ScopeAction::Squeeze) => "squeeze",
                        Some(ScopeAction::Symbols) => "symbols",
                        Some(ScopeAction::Normalize) => "normalize",
                        None => "none",
                    }
                };

                // Receiver may have dropped during shutdown — send failure is expected
                let _ = tx.send(ScopeEvent::Result(ScopeResult {
                    r#type: "scoped",
                    path: path.display().to_string(),
                    language: lang_name.to_string(),
                    query: qn.to_string(),
                    action: action_name.to_owned(),
                    scopes_matched,
                    bytes_before,
                    bytes_after: content.len() as u64,
                    checksum_before,
                    checksum_after,
                    elapsed_ms: file_start.elapsed().as_millis() as u64,
                }));

                ignore::WalkState::Continue
            })
        });
    });

    for event in rx {
        if shutdown.is_shutdown() {
            break;
        }
        match event {
            ScopeEvent::Result(r) => writer.write_event(&r)?,
        }
    }

    if let Err(panic_payload) = walker_thread.join() {
        std::panic::resume_unwind(panic_payload);
    }

    let summary = Summary {
        r#type: "summary",
        files_visited: files_visited.load(Ordering::Relaxed),
        files_matched: files_modified.load(Ordering::Relaxed),
        files_modified: {
            let is_read_only = !args.delete && args.action.is_none() && args.replace_with.is_none();
            if !args.dry_run && !is_read_only {
                Some(files_modified.load(Ordering::Relaxed))
            } else {
                None
            }
        },
        files_skipped: Some(files_skipped.load(Ordering::Relaxed)),
        total_matches: None,
        total_replacements: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };

    writer.write_event(&summary)?;
    Ok(())
}

fn expand_to_full_line(content: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = content.as_bytes();
    let line_start = bytes[..start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |pos| pos + 1);
    let line_end = bytes[end..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(content.len(), |pos| end + pos + 1);
    (line_start, line_end)
}

fn apply_scope_action<'a>(
    text: &'a str,
    delete: bool,
    action: Option<ScopeAction>,
    replace_with: Option<&str>,
) -> std::borrow::Cow<'a, str> {
    if delete {
        return std::borrow::Cow::Owned(String::new());
    }
    if let Some(replacement) = replace_with {
        return std::borrow::Cow::Owned(replacement.to_owned());
    }
    match action {
        Some(ScopeAction::Upper) => std::borrow::Cow::Owned(text.to_uppercase()),
        Some(ScopeAction::Lower) => std::borrow::Cow::Owned(text.to_lowercase()),
        Some(ScopeAction::Titlecase) => std::borrow::Cow::Owned(titlecase(text)),
        Some(ScopeAction::Squeeze) => std::borrow::Cow::Owned(squeeze(text)),
        Some(ScopeAction::Symbols) => std::borrow::Cow::Owned(symbolize(text)),
        Some(ScopeAction::Normalize) => std::borrow::Cow::Owned(text.nfc().collect::<String>()),
        None => std::borrow::Cow::Borrowed(text),
    }
}

fn titlecase(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for c in s.chars() {
        if capitalize_next && c.is_alphabetic() {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
            if c.is_whitespace() || c == '_' || c == '-' {
                capitalize_next = true;
            }
        }
    }
    result
}

fn squeeze(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if Some(c) != prev || !c.is_whitespace() {
            result.push(c);
        }
        prev = Some(c);
    }
    result
}

fn symbolize(s: &str) -> String {
    s.replace("=>", "⇒")
        .replace("->", "→")
        .replace("<-", "←")
        .replace("!=", "≠")
        .replace(">=", "≥")
        .replace("<=", "≤")
        .replace("...", "…")
        .replace("--", "—")
}

fn resolve_patterns(
    query_name: &Option<String>,
    custom_pattern: &Option<String>,
    lang_str: &str,
) -> Result<Vec<String>> {
    if let Some(p) = custom_pattern {
        return Ok(vec![p.clone()]);
    }

    let name = query_name
        .as_deref()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "either --query or --pattern is required".into(),
        })?;

    lookup_prepared_queries(name, lang_str)
}

fn parse_language(lang_str: &str) -> Result<SupportLang> {
    lang_str.parse().map_err(|_| {
        AtomwriteError::InvalidInput {
            reason: format!("unsupported language: {lang_str}"),
        }
        .into()
    })
}

fn lookup_prepared_queries(name: &str, lang: &str) -> Result<Vec<String>> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => lookup_rust_queries(name),
        "python" | "py" => lookup_python_query(name).map(|s| vec![s]),
        "javascript" | "js" | "typescript" | "ts" | "tsx" | "jsx" => {
            lookup_js_query(name).map(|s| vec![s])
        }
        "go" | "golang" => lookup_go_query(name).map(|s| vec![s]),
        _ => Err(AtomwriteError::InvalidInput {
            reason: format!(
                "no prepared queries for language: {lang}. \
                 Supported: rust, python, javascript, typescript, go"
            ),
        }
        .into()),
    }
}

fn lookup_rust_queries(name: &str) -> Result<Vec<String>> {
    let qs: Vec<&str> = match name {
        "comments" => vec!["// $$BODY\\s*", "/* $$$BODY */"],
        "strings" => vec!["\"$$$BODY\""],
        "fn" => vec!["fn $NAME($$$ARGS) { $$$BODY }"],
        "pub-fn" => vec!["pub fn $NAME($$$ARGS) { $$$BODY }"],
        "async-fn" => vec!["async fn $NAME($$$ARGS) { $$$BODY }"],
        "unsafe-fn" => vec!["unsafe fn $NAME($$$ARGS) { $$$BODY }"],
        "struct" => vec!["struct $NAME { $$$FIELDS }"],
        "pub-struct" => vec!["pub struct $NAME { $$$FIELDS }"],
        "enum" => vec!["enum $NAME { $$$VARIANTS }"],
        "pub-enum" => vec!["pub enum $NAME { $$$VARIANTS }"],
        "trait" => vec!["trait $NAME { $$$BODY }"],
        "impl" => vec!["impl $TYPE { $$$BODY }"],
        "mod" => vec!["mod $NAME { $$$BODY }"],
        "closure" => vec!["|$$$ARGS| $$$BODY"],
        "unsafe" => vec!["unsafe { $$$BODY }"],
        "use" => vec!["use $$$PATH;"],
        // GAP-134: test-fn pattern is multi-node (#[test] + fn) which
        // ast-grep rejects. Disabled until ast-grep supports composite patterns.
        "test-fn" => {
            return Err(AtomwriteError::InvalidInput {
                reason: "query 'test-fn' is currently unavailable: the pattern '#[test] fn ...' \
                         spans two AST nodes (attribute + function_item) which ast-grep does not \
                         support as a single pattern. Use 'atomwrite scope --pattern \"#[test]\"' \
                         to match test attributes, or 'atomwrite query -Q \
                         \"(function_item (attribute_item) @attr)\"' for tree-sitter queries."
                    .into(),
            }
            .into());
        }
        "attribute" => vec!["#[$$$ATTR]"],
        "return" => vec!["return $$$EXPR"],
        "match" => vec!["match $EXPR { $$$ARMS }"],
        "if-let" => vec!["if let $PAT = $EXPR { $$$BODY }"],
        "while-let" => vec!["while let $PAT = $EXPR { $$$BODY }"],
        "for" => vec!["for $PAT in $ITER { $$$BODY }"],
        "loop" => vec!["loop { $$$BODY }"],
        "const" => vec!["const $NAME: $TYPE = $$$EXPR;"],
        "static" => vec!["static $NAME: $TYPE = $$$EXPR;"],
        "type-alias" => vec!["type $NAME = $$$TYPE;"],
        "macro-rules" => vec!["macro_rules! $NAME { $$$BODY }"],
        "derive" => vec!["#[derive($$$TRAITS)]"],
        "doc-comment" => vec!["/// $$$BODY"],
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown Rust query: {name}. Available: comments, strings, fn, pub-fn, \
                     async-fn, unsafe-fn, struct, pub-struct, enum, pub-enum, trait, impl, \
                     mod, closure, unsafe, use, attribute, return, match, if-let, \
                     while-let, for, loop, const, static, type-alias, macro-rules, derive, \
                     doc-comment. Note: test-fn is disabled (ast-grep multi-node limitation)"
                ),
            }
            .into());
        }
    };
    Ok(qs.into_iter().map(String::from).collect())
}

fn lookup_python_query(name: &str) -> Result<String> {
    let q = match name {
        "comments" => "# $$$BODY",
        "strings" => "\"$$$BODY\"",
        "class" => "class $NAME: $$$BODY",
        "def" => "def $NAME($$$ARGS): $$$BODY",
        "async-def" => "async def $NAME($$$ARGS): $$$BODY",
        "lambda" => "lambda $$$ARGS: $BODY",
        "import" => "import $$$NAMES",
        "from-import" => "from $MODULE import $$$NAMES",
        "with" => "with $EXPR as $NAME: $$$BODY",
        "for" => "for $VAR in $ITER: $$$BODY",
        "while" => "while $COND: $$$BODY",
        "decorator" => "@$NAME($$$ARGS)",
        "try-except" => "try: $$$BODY",
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown Python query: {name}. Available: comments, strings, class, def, \
                     async-def, lambda, import, from-import, with, for, while, decorator, \
                     try-except"
                ),
            }
            .into());
        }
    };
    Ok(q.to_owned())
}

fn lookup_js_query(name: &str) -> Result<String> {
    let q = match name {
        "comments" => "// $$BODY\\s*",
        "strings" => "\"$$$BODY\"",
        "fn" => "function $NAME($$$ARGS) { $$$BODY }",
        "arrow-fn" => "const $NAME = ($$$ARGS) => $$$BODY",
        "class" => "class $NAME { $$$BODY }",
        "import" => "import $$$IMPORTS from \"$MODULE\"",
        "export" => "export $$$DECL",
        "async-fn" => "async function $NAME($$$ARGS) { $$$BODY }",
        "try-catch" => "try { $$$BODY } catch ($ERR) { $$$HANDLER }",
        "const" => "const $NAME = $$$EXPR",
        "let" => "let $NAME = $$$EXPR",
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown JS/TS query: {name}. Available: comments, strings, fn, arrow-fn, \
                     class, import, export, async-fn, try-catch, const, let"
                ),
            }
            .into());
        }
    };
    Ok(q.to_owned())
}

fn lookup_go_query(name: &str) -> Result<String> {
    let q = match name {
        "fn" => "func $NAME($$$ARGS) $$$RET { $$$BODY }",
        "struct" => "type $NAME struct { $$$FIELDS }",
        "interface" => "type $NAME interface { $$$METHODS }",
        "goroutine" => "go $$$EXPR",
        "defer" => "defer $$$EXPR",
        "import" => "import $$$IMPORTS",
        "const" => "const $NAME = $$$EXPR",
        "var" => "var $NAME $TYPE = $$$EXPR",
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown Go query: {name}. Available: fn, struct, interface, goroutine, \
                     defer, import, const, var"
                ),
            }
            .into());
        }
    };
    Ok(q.to_owned())
}

enum ScopeEvent {
    Result(ScopeResult),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titlecase_basic() {
        assert_eq!(titlecase("hello world"), "Hello World");
    }

    #[test]
    fn titlecase_underscore() {
        assert_eq!(titlecase("foo_bar"), "Foo_Bar");
    }

    #[test]
    fn squeeze_whitespace() {
        assert_eq!(squeeze("a  b   c"), "a b c");
    }

    #[test]
    fn squeeze_preserves_non_whitespace() {
        assert_eq!(squeeze("aabbcc"), "aabbcc");
    }

    #[test]
    fn lookup_rust_queries_known() {
        let result = lookup_rust_queries("fn");
        assert!(result.is_ok());
        let qs = result.unwrap();
        assert_eq!(qs.len(), 1);
        assert!(qs[0].contains("fn $NAME"));
    }

    #[test]
    fn lookup_rust_queries_comments_multi() {
        let result = lookup_rust_queries("comments");
        assert!(result.is_ok());
        let qs = result.unwrap();
        assert_eq!(
            qs.len(),
            2,
            "comments should produce 2 patterns (line + block)"
        );
    }

    #[test]
    fn lookup_rust_queries_unknown() {
        let result = lookup_rust_queries("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn lookup_js_query_known() {
        let result = lookup_js_query("class");
        assert!(result.is_ok());
    }

    #[test]
    fn lookup_go_query_known() {
        let result = lookup_go_query("struct");
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_patterns_custom() {
        let result = resolve_patterns(&None, &Some("custom_pattern".into()), "rust");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["custom_pattern"]);
    }

    #[test]
    fn resolve_patterns_requires_query_or_pattern() {
        let result = resolve_patterns(&None, &None, "rust");
        assert!(result.is_err());
    }

    #[test]
    fn apply_scope_action_delete() {
        assert_eq!(apply_scope_action("hello", true, None, None), "");
    }

    #[test]
    fn apply_scope_action_upper() {
        assert_eq!(
            apply_scope_action("hello", false, Some(ScopeAction::Upper), None),
            "HELLO"
        );
    }

    #[test]
    fn apply_scope_action_replace() {
        assert_eq!(apply_scope_action("old", false, None, Some("new")), "new");
    }
}
