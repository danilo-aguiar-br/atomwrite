// SPDX-License-Identifier: MIT OR Apache-2.0

//! Every record `type` the product emits is classified exactly once (R6).
//!
//! `SUCCESS_ENVELOPE_TYPES` decides which records the writer stamps with
//! `ok: true`, and `STREAM_RECORD_TYPES` lists the ones it never stamps. A
//! `type` in neither list is a record the writer silently ignores, and a list
//! entry no subcommand emits is a verdict that can never fire. Both used to
//! drift unseen: the list named `copy`, `move`, `apply`, `delete` and `scope`
//! while the verbs emit `copied`, `moved`, `applied`, `deleted` and `scoped`.

use atomwrite::output::{STREAM_RECORD_TYPES, SUCCESS_ENVELOPE_TYPES};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

/// Every `type` literal some source file stores in an emitted record.
///
/// Test fixtures are skipped by file name, because they build records no
/// subcommand emits. `stream_diff` is the one record whose discriminator field
/// is called `kind` (serde renames it to `type` on the wire).
fn emitted_types() -> BTreeMap<String, Vec<String>> {
    let discriminator =
        Regex::new(r#"r#type:\s*"([a-z_]+)"|kind:\s*"(stream_diff)""#).expect("regex");
    let mut files = Vec::new();
    rust_files(&src_dir(), &mut files);
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.contains("test") {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        for cap in discriminator.captures_iter(&text) {
            let kind = cap.get(1).or_else(|| cap.get(2)).expect("a group").as_str();
            found
                .entry(kind.to_string())
                .or_default()
                .push(name.to_string());
        }
    }
    found
}

fn set_of(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn the_two_lists_have_no_duplicates_and_do_not_overlap() {
    let success = set_of(SUCCESS_ENVELOPE_TYPES);
    let stream = set_of(STREAM_RECORD_TYPES);
    assert_eq!(
        success.len(),
        SUCCESS_ENVELOPE_TYPES.len(),
        "SUCCESS_ENVELOPE_TYPES repeats an entry"
    );
    assert_eq!(
        stream.len(),
        STREAM_RECORD_TYPES.len(),
        "STREAM_RECORD_TYPES repeats an entry"
    );
    let both: Vec<&String> = success.intersection(&stream).collect();
    assert!(
        both.is_empty(),
        "types listed as both outcome and stream: {both:?}"
    );
}

#[test]
fn every_emitted_type_is_classified() {
    let known: BTreeSet<String> = set_of(SUCCESS_ENVELOPE_TYPES)
        .union(&set_of(STREAM_RECORD_TYPES))
        .cloned()
        .collect();
    let emitted = emitted_types();
    assert!(
        emitted.len() > 30,
        "the scan found only {} types, the pattern is stale",
        emitted.len()
    );
    let unclassified: Vec<String> = emitted
        .iter()
        .filter(|(kind, _)| !known.contains(*kind))
        .map(|(kind, files)| format!("{kind} (in {})", files.join(", ")))
        .collect();
    assert!(
        unclassified.is_empty(),
        "emitted types missing from SUCCESS_ENVELOPE_TYPES and STREAM_RECORD_TYPES: {unclassified:?}"
    );
}

#[test]
fn every_listed_type_is_emitted_by_some_verb() {
    let emitted: BTreeSet<String> = emitted_types().into_keys().collect();
    let phantom: Vec<&str> = SUCCESS_ENVELOPE_TYPES
        .iter()
        .chain(STREAM_RECORD_TYPES.iter())
        .copied()
        .filter(|kind| !emitted.contains(*kind))
        .collect();
    assert!(
        phantom.is_empty(),
        "listed types no subcommand emits (they can never match): {phantom:?}"
    );
}
