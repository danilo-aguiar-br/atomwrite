// SPDX-License-Identifier: MIT OR Apache-2.0

//! Grammatical scoping: AST-based code selection and transformation.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use anyhow::Result;
use ast_grep_core::AstGrep;
use ast_grep_core::matcher::Pattern;
use ast_grep_language::SupportLang;

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
    #[arg(
        short = 'l',
        long,
        required = true,
        help = "Language (rust, py, ts, go, c, etc)"
    )]
    pub lang: String,

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
        help = "Transform action: upper, lower, titlecase, squeeze"
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
}

/// Execute the scope subcommand.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` for unknown language, query, or pattern.
pub fn cmd_scope(
    args: &ScopeArgs,
    global: &GlobalArgs,
    writer: &mut NdjsonWriter<impl Write>,
    _shutdown: &ShutdownSignal,
) -> Result<()> {
    let start = Instant::now();

    let lang = parse_language(&args.lang)?;
    let pattern_str = resolve_pattern(&args.query, &args.pattern, &args.lang)?;
    let pattern =
        Pattern::try_new(&pattern_str, lang).map_err(|e| AtomwriteError::InvalidInput {
            reason: format!("invalid scope pattern: {e}"),
        })?;
    let query_name = args.query.clone().unwrap_or_else(|| "custom".to_owned());

    let workspace = global.resolve_workspace()?;

    let mut walker = ignore::WalkBuilder::new(&args.paths[0]);
    for p in args.paths.iter().skip(1) {
        walker.add(p);
    }
    walker
        .hidden(!global.hidden)
        .git_ignore(!global.no_gitignore)
        .follow_links(global.follow_symlinks);

    if !args.include.is_empty() || !args.exclude.is_empty() {
        let mut overrides = ignore::overrides::OverrideBuilder::new(&args.paths[0]);
        for pat in &args.include {
            overrides.add(pat)?;
        }
        for pat in &args.exclude {
            overrides.add(&format!("!{pat}"))?;
        }
        walker.overrides(overrides.build()?);
    }

    let (tx, rx) = crossbeam_channel::unbounded::<ScopeEvent>();

    let files_visited = Arc::new(AtomicU64::new(0));
    let files_modified = Arc::new(AtomicU64::new(0));
    let files_skipped = Arc::new(AtomicU64::new(0));

    let fv = Arc::clone(&files_visited);
    let fm = Arc::clone(&files_modified);
    let fs = Arc::clone(&files_skipped);
    let delete = args.delete;
    let action = args.action;
    let replace_with = args.replace_with.clone();
    let dry_run = args.dry_run;
    let backup = args.backup;
    let ws = workspace.clone();
    let qn = query_name.clone();
    let lang_name = args.lang.clone();

    let walker_thread = std::thread::spawn(move || {
        walker.build_parallel().run(|| {
            let pattern = pattern.clone();
            let tx = tx.clone();
            let fv = Arc::clone(&fv);
            let fm = Arc::clone(&fm);
            let fs = Arc::clone(&fs);
            let replace_with = replace_with.clone();
            let ws = ws.clone();
            let qn = qn.clone();
            let lang_name = lang_name.clone();

            Box::new(move |entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return ignore::WalkState::Continue,
                };

                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    return ignore::WalkState::Continue;
                }

                fv.fetch_add(1, Ordering::Relaxed);
                let path = entry.path().to_path_buf();
                let file_start = Instant::now();

                let content = match crate::file_io::read_file_string(&path) {
                    Ok(c) => c,
                    Err(_) => {
                        fs.fetch_add(1, Ordering::Relaxed);
                        return ignore::WalkState::Continue;
                    }
                };

                let grep = AstGrep::new(&content, lang);
                let root = grep.root();
                let matches: Vec<_> = root.find_all(&pattern).collect();

                if matches.is_empty() {
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                let scopes_matched = matches.len() as u64;
                let mut edits: Vec<(usize, usize, String)> = Vec::with_capacity(matches.len());

                for m in &matches {
                    let range = m.range();
                    let matched_text = &content[range.start..range.end];
                    let replacement =
                        apply_scope_action(matched_text, delete, action, replace_with.as_deref());
                    edits.push((range.start, range.end, replacement));
                }

                edits.sort_by_key(|e| std::cmp::Reverse(e.0));

                let mut result_content = content.clone();
                for (s, e, replacement) in &edits {
                    result_content.replace_range(*s..*e, replacement);
                }

                if result_content == content {
                    fs.fetch_add(1, Ordering::Relaxed);
                    return ignore::WalkState::Continue;
                }

                fm.fetch_add(1, Ordering::Relaxed);

                let checksum_before = checksum::hash_bytes(content.as_bytes());
                let checksum_after = checksum::hash_bytes(result_content.as_bytes());

                if !dry_run {
                    let opts = AtomicWriteOptions {
                        backup,
                        ..Default::default()
                    };
                    if let Err(e) = atomic_write(&path, result_content.as_bytes(), &opts, &ws) {
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
                        None => "none",
                    }
                };

                let _ = tx.send(ScopeEvent::Result(ScopeResult {
                    r#type: "scoped",
                    path: path.display().to_string(),
                    language: lang_name.clone(),
                    query: qn.clone(),
                    action: action_name.to_owned(),
                    scopes_matched,
                    bytes_before: content.len() as u64,
                    bytes_after: result_content.len() as u64,
                    checksum_before,
                    checksum_after,
                    elapsed_ms: file_start.elapsed().as_millis() as u64,
                }));

                ignore::WalkState::Continue
            })
        });
    });

    for event in rx {
        match event {
            ScopeEvent::Result(r) => writer.write_event(&r)?,
        }
    }

    let _ = walker_thread.join();

    let summary = Summary {
        r#type: "summary",
        files_visited: files_visited.load(Ordering::Relaxed),
        files_matched: files_modified.load(Ordering::Relaxed),
        files_modified: if !args.dry_run {
            Some(files_modified.load(Ordering::Relaxed))
        } else {
            None
        },
        files_skipped: Some(files_skipped.load(Ordering::Relaxed)),
        total_matches: None,
        total_replacements: None,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };

    writer.write_event(&summary)?;
    Ok(())
}

fn apply_scope_action(
    text: &str,
    delete: bool,
    action: Option<ScopeAction>,
    replace_with: Option<&str>,
) -> String {
    if delete {
        return String::new();
    }
    if let Some(replacement) = replace_with {
        return replacement.to_owned();
    }
    match action {
        Some(ScopeAction::Upper) => text.to_uppercase(),
        Some(ScopeAction::Lower) => text.to_lowercase(),
        Some(ScopeAction::Titlecase) => titlecase(text),
        Some(ScopeAction::Squeeze) => squeeze(text),
        None => text.to_owned(),
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

fn resolve_pattern(
    query_name: &Option<String>,
    custom_pattern: &Option<String>,
    lang_str: &str,
) -> Result<String> {
    if let Some(p) = custom_pattern {
        return Ok(p.clone());
    }

    let name = query_name
        .as_deref()
        .ok_or_else(|| AtomwriteError::InvalidInput {
            reason: "either --query or --pattern is required".into(),
        })?;

    lookup_prepared_query(name, lang_str)
}

fn parse_language(lang_str: &str) -> Result<SupportLang> {
    lang_str.parse().map_err(|_| {
        AtomwriteError::InvalidInput {
            reason: format!("unsupported language: {lang_str}"),
        }
        .into()
    })
}

fn lookup_prepared_query(name: &str, lang: &str) -> Result<String> {
    match lang.to_lowercase().as_str() {
        "rust" | "rs" => lookup_rust_query(name),
        "python" | "py" => lookup_python_query(name),
        "javascript" | "js" | "typescript" | "ts" | "tsx" | "jsx" => lookup_js_query(name),
        "go" | "golang" => lookup_go_query(name),
        _ => Err(AtomwriteError::InvalidInput {
            reason: format!(
                "no prepared queries for language: {lang}. \
                 Supported: rust, python, javascript, typescript, go"
            ),
        }
        .into()),
    }
}

fn lookup_rust_query(name: &str) -> Result<String> {
    let q = match name {
        "comments" => "// $$$BODY",
        "strings" => "\"$$$BODY\"",
        "fn" => "fn $NAME($$$ARGS) { $$$BODY }",
        "pub-fn" => "pub fn $NAME($$$ARGS) { $$$BODY }",
        "async-fn" => "async fn $NAME($$$ARGS) { $$$BODY }",
        "unsafe-fn" => "unsafe fn $NAME($$$ARGS) { $$$BODY }",
        "struct" => "struct $NAME { $$$FIELDS }",
        "pub-struct" => "pub struct $NAME { $$$FIELDS }",
        "enum" => "enum $NAME { $$$VARIANTS }",
        "pub-enum" => "pub enum $NAME { $$$VARIANTS }",
        "trait" => "trait $NAME { $$$BODY }",
        "impl" => "impl $TYPE { $$$BODY }",
        "mod" => "mod $NAME { $$$BODY }",
        "closure" => "|$$$ARGS| $$$BODY",
        "unsafe" => "unsafe { $$$BODY }",
        "use" => "use $$$PATH;",
        "test-fn" => "#[test] fn $NAME() { $$$BODY }",
        "attribute" => "#[$$$ATTR]",
        "return" => "return $$$EXPR",
        "match" => "match $EXPR { $$$ARMS }",
        "if-let" => "if let $PAT = $EXPR { $$$BODY }",
        "while-let" => "while let $PAT = $EXPR { $$$BODY }",
        "for" => "for $PAT in $ITER { $$$BODY }",
        "loop" => "loop { $$$BODY }",
        "const" => "const $NAME: $TYPE = $$$EXPR;",
        "static" => "static $NAME: $TYPE = $$$EXPR;",
        "type-alias" => "type $NAME = $$$TYPE;",
        "macro-rules" => "macro_rules! $NAME { $$$BODY }",
        "derive" => "#[derive($$$TRAITS)]",
        "doc-comment" => "/// $$$BODY",
        _ => {
            return Err(AtomwriteError::InvalidInput {
                reason: format!(
                    "unknown Rust query: {name}. Available: comments, strings, fn, pub-fn, \
                     async-fn, unsafe-fn, struct, pub-struct, enum, pub-enum, trait, impl, \
                     mod, closure, unsafe, use, test-fn, attribute, return, match, if-let, \
                     while-let, for, loop, const, static, type-alias, macro-rules, derive, \
                     doc-comment"
                ),
            }
            .into());
        }
    };
    Ok(q.to_owned())
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
        "comments" => "// $$$BODY",
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
    fn lookup_rust_query_known() {
        let result = lookup_rust_query("fn");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("fn $NAME"));
    }

    #[test]
    fn lookup_rust_query_unknown() {
        let result = lookup_rust_query("nonexistent");
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
    fn resolve_pattern_custom() {
        let result = resolve_pattern(&None, &Some("custom_pattern".into()), "rust");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "custom_pattern");
    }

    #[test]
    fn resolve_pattern_requires_query_or_pattern() {
        let result = resolve_pattern(&None, &None, "rust");
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
