// SPDX-License-Identifier: MIT OR Apache-2.0

//! Source guard: no silent `let _ = fallible();` and no panic in production code.
//!
//! A discarded `Result` hides the failure from everyone. Production code either
//! propagates it with `?`, or states that ignoring it is deliberate through the
//! helpers in `atomwrite::best_effort`, which log it or name the reason. This
//! test counts the `let _ =` statements per source file (test code excluded)
//! and compares the count with an explicit, documented allowance.
//!
//! The allowance is a ratchet: more statements than allowed fail (a new
//! silent discard). For a justified entry, fewer than allowed also fails
//! (tighten the entry, or delete it when it reaches zero). A `PENDING` entry
//! is only an upper bound while its owner is still working.
//!
//! The second half of the file holds the same line for panics: `unwrap()`,
//! `expect(`, `unreachable!`, `panic!`, `todo!` and `unimplemented!` have no
//! place outside test code, where a failure is a typed error instead.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// `(file relative to the crate root, allowed count, why)`.
///
/// Entries whose reason starts with `PENDING` are files another workstream of
/// the same delivery still owns; they must reach zero or move to a justified
/// entry before the delivery closes.
const ALLOWED: &[(&str, usize, &str)] = &[];

fn rust_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("read_dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") && !is_test_file(&path) {
            out.push(path);
        }
    }
}

fn is_test_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name == "tests.rs"
        || name.starts_with("tests")
        || name.ends_with("tests.inc.rs")
        || name.ends_with("_tests.rs")
}

fn is_test_cfg(trimmed: &str) -> bool {
    trimmed.starts_with("#[cfg(") && trimmed.contains("test") && !trimmed.contains("not(test")
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Production lines of a source file, with their 1-based numbers.
///
/// Everything from a `#[cfg(test)] mod` onward is dropped, and so is a single
/// `#[cfg(test)]` item (a helper function or a `use`) up to its closing brace.
fn production_lines(source: &str) -> Vec<(usize, &str)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if !is_test_cfg(lines[i].trim()) {
            out.push((i + 1, lines[i]));
            i += 1;
            continue;
        }
        let Some(offset) = lines[i + 1..].iter().position(|l| !l.trim().is_empty()) else {
            break;
        };
        let first = i + 1 + offset;
        let head = lines[first].trim();
        if head.starts_with("mod ") || head.starts_with("pub mod ") {
            break;
        }
        // One test-only item: a `use` ends at its `;`, anything else at the
        // closing brace found at the attribute's own indent.
        let indent = indent_of(lines[i]);
        let mut end = first;
        if !lines[first].trim_end().ends_with(';') {
            while end < lines.len()
                && !(indent_of(lines[end]) == indent && lines[end].trim_start().starts_with('}'))
            {
                end += 1;
            }
        }
        i = end + 1;
    }
    out
}

/// Count `let _ =` / `let _:` statements in production code.
fn silent_discards(source: &str) -> usize {
    production_lines(source)
        .iter()
        .filter(|(_, l)| {
            let t = l.trim();
            t.starts_with("let _ =") || t.starts_with("let _:")
        })
        .count()
}

fn collect_counts(count: impl Fn(&str) -> usize) -> BTreeMap<String, usize> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);

    let mut actual: BTreeMap<String, usize> = BTreeMap::new();
    for file in files {
        let source = fs::read_to_string(&file).expect("read source");
        let n = count(&source);
        if n > 0 {
            let rel = file
                .strip_prefix(root)
                .expect("under crate root")
                .to_string_lossy()
                .replace('\\', "/");
            actual.insert(rel, n);
        }
    }
    actual
}

#[test]
fn let_underscore_stays_within_the_documented_allowance() {
    let actual = collect_counts(silent_discards);

    let mut problems = Vec::new();
    for (file, n) in &actual {
        let allowed = ALLOWED
            .iter()
            .find(|(f, _, _)| f == file)
            .map_or(0, |(_, c, _)| *c);
        if *n > allowed {
            problems.push(format!(
                "{file}: {n} `let _ =` found, {allowed} allowed (propagate with `?` or use best_effort::best_effort)"
            ));
        }
    }
    for (file, allowed, why) in ALLOWED {
        let n = actual.get(*file).copied().unwrap_or(0);
        // A PENDING entry may shrink while its owner is still working; it is
        // only an upper bound until the delivery closes.
        if n < *allowed && !why.starts_with("PENDING") {
            problems.push(format!(
                "{file}: allowance is {allowed} but only {n} remain (tighten or remove the entry)"
            ));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn every_allowance_states_a_reason() {
    for (file, count, why) in ALLOWED {
        assert!(*count > 0, "{file}: a zero allowance is just no entry");
        assert!(why.len() > 10, "{file}: the reason is too short to be one");
    }
}

#[test]
fn counter_ignores_comments_and_test_code() {
    let src = "// let _ = a();\nfn f() { let _ = b(); }\nlet _ = c();\n#[cfg(test)]\nmod tests {\n    let _ = d();\n}\n";
    // Only the third line is a statement at line start; line 2 begins with `fn`.
    assert_eq!(silent_discards(src), 1);
    let guarded = "#[cfg(test)]\nfn only_in_tests() {\n    let _ = f();\n}\nlet _ = e();\n";
    assert_eq!(silent_discards(guarded), 1);
    let one_line = "#[cfg(test)]\nuse std::fmt;\nlet _ = g();\n";
    assert_eq!(silent_discards(one_line), 1);
}

/// Panicking constructs searched for, each as a call-site needle.
const PANIC_NEEDLES: &[&str] = &[
    ".unwrap()",
    ".expect(",
    "unreachable!(",
    "panic!(",
    "todo!(",
    "unimplemented!(",
];

/// Count panicking calls in production code.
///
/// Comment lines are skipped, and a macro needle glued to an identifier (such
/// as `setup_panic!(` from `human_panic`) is not the std macro.
fn panics_in_production(source: &str) -> usize {
    let mut found = 0;
    for (_, line) in production_lines(source) {
        if line.trim_start().starts_with("//") {
            continue;
        }
        for needle in PANIC_NEEDLES {
            let mut from = 0;
            while let Some(at) = line[from..].find(needle) {
                let start = from + at;
                let glued = !needle.starts_with('.')
                    && line[..start]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric() || c == '_');
                if !glued {
                    found += 1;
                }
                from = start + needle.len();
            }
        }
    }
    found
}

/// `(file, allowed count, why)` for a panic that a unit test proves impossible.
const ALLOWED_PANICS: &[(&str, usize, &str)] = &[];

#[test]
fn production_code_does_not_panic() {
    let actual = collect_counts(panics_in_production);
    let mut problems = Vec::new();
    for (file, n) in &actual {
        let allowed = ALLOWED_PANICS
            .iter()
            .find(|(f, _, _)| f == file)
            .map_or(0, |(_, c, _)| *c);
        if *n > allowed {
            problems.push(format!(
                "{file}: {n} panicking call(s), {allowed} allowed (return a typed error such as AtomwriteError::InternalError)"
            ));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn panic_counter_sees_calls_and_skips_tests_comments_and_glued_names() {
    let src = "fn f() { x.unwrap(); }\n// y.unwrap();\nhuman_panic::setup_panic!();\nfn g() { panic!(\"no\"); }\n#[cfg(test)]\nmod tests {\n    fn t() { z.expect(\"e\"); }\n}\n";
    assert_eq!(panics_in_production(src), 2);
    let test_only =
        "#[cfg(test)]\nfn helper() {\n    a.expect(\"b\")\n}\nfn real() { c.unwrap() }\n";
    assert_eq!(panics_in_production(test_only), 1);
}
