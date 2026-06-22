// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regex pattern generation from example strings via grex.
//! Workload: CPU-bound (regex synthesis from examples).

use std::io::{BufRead, Read, Write};
use std::time::Instant;

use anyhow::Result;

use crate::cli::RegexArgs;
use crate::error::AtomwriteError;
use crate::ndjson_types::RegexOutput;
use crate::output::NdjsonWriter;

/// Generate a regular expression from example strings via the grex engine.
///
/// # Errors
///
/// Returns `AtomwriteError::InvalidInput` if no example strings are provided.
#[tracing::instrument(skip_all, fields(command = "regex"))]
pub fn cmd_regex(
    args: &RegexArgs,
    stdin: impl Read,
    writer: &mut NdjsonWriter<impl Write>,
) -> Result<()> {
    let start = Instant::now();

    // GAP-2026-025: warn when examples look like flags (allow_hyphen_values quirk)
    const KNOWN_FLAGS: &[&str] = &[
        "--digits", "--words", "--spaces", "--repetitions",
        "--case-insensitive", "--no-anchors", "--stdin",
    ];
    for ex in &args.examples {
        if KNOWN_FLAGS.contains(&ex.as_str())
            || matches!(ex.as_str(), "-d" | "-w" | "-s" | "-r" | "-i")
        {
            eprintln!(
                "\x1b[33mwarning:\x1b[0m example {:?} looks like a flag; \
                 place flags BEFORE examples: `atomwrite regex {} \"ex1\" \"ex2\"`",
                ex, ex
            );
        }
    }

    let examples = if args.stdin || args.examples.is_empty() {
        let reader = std::io::BufReader::with_capacity(crate::constants::BUF_CAPACITY, stdin);
        let lines: Vec<String> = reader
            .lines()
            .map_while(|l| l.ok())
            .filter(|l| !l.trim().is_empty())
            .collect();
        lines
    } else {
        args.examples.clone()
    };

    if examples.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "at least one example is required".into(),
        }
        .into());
    }

    let mut builder = grex::RegExpBuilder::from(&examples);

    if args.digits {
        builder.with_conversion_of_digits();
    }
    if args.words {
        builder.with_conversion_of_words();
    }
    if args.spaces {
        builder.with_conversion_of_whitespace();
    }
    if args.repetitions {
        builder.with_conversion_of_repetitions();
    }
    if args.case_insensitive {
        builder.with_case_insensitive_matching();
    }
    if args.no_anchors {
        builder.without_anchors();
    }

    let regex_str = builder.build();

    regex::Regex::new(&regex_str).map_err(|e| AtomwriteError::InvalidInput {
        reason: format!("generated regex is invalid: {e}"),
    })?;

    let output = RegexOutput {
        r#type: "regex",
        regex: regex_str,
        examples: examples.len() as u64,
        anchored: !args.no_anchors,
        elapsed_ms: start.elapsed().as_millis() as u64,
    };

    writer.write_event(&output)?;
    Ok(())
}
