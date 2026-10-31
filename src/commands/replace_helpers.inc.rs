fn compile_pattern(args: &ReplaceArgs) -> Result<Regex> {
    if args.pattern.is_empty() {
        return Err(crate::error::AtomwriteError::InvalidInput {
            reason: "pattern must not be empty".to_string(),
        }
        .into());
    }

    let pattern_str = if args.literal || !args.regex {
        regex::escape(&args.pattern)
    } else {
        args.pattern.clone()
    };

    let pattern_str = if args.word {
        format!(r"\b{pattern_str}\b")
    } else {
        pattern_str
    };

    let pattern_str = if args.preserve_case {
        format!("(?i){pattern_str}")
    } else {
        pattern_str
    };

    Regex::new(&pattern_str).map_err(|e| {
        crate::error::AtomwriteError::invalid(&format!("invalid pattern '{}'", args.pattern), e)
            .into()
    })
}

fn adapt_case(original: &str, replacement: &str) -> String {
    if original
        .chars()
        .all(|c| !c.is_alphabetic() || c.is_uppercase())
        && original.chars().any(|c| c.is_alphabetic())
    {
        replacement.to_uppercase()
    } else if original
        .chars()
        .all(|c| !c.is_alphabetic() || c.is_lowercase())
    {
        replacement.to_lowercase()
    } else if original.starts_with(|c: char| c.is_uppercase()) {
        let mut chars = replacement.chars();
        match chars.next() {
            Some(first) => {
                let mut s = first.to_uppercase().to_string();
                s.push_str(chars.as_str());
                s
            }
            None => String::new(),
        }
    } else {
        replacement.to_owned()
    }
}

/// Apply `replacement` to the matches of `pattern`, with an optional
/// block-granular cancellation check point (v0.1.36).
///
/// Both the counting scan and the rewriting scan poll `cancel` once per work
/// block, so a run that blows the `--timeout-secs` deadline stops inside the
/// file instead of finishing the whole buffer first.
///
/// `replacement` is expanded with [`regex::Captures::expand`], which is what
/// the upstream `replace_all` / `replacen` calls do for a `&str` replacer —
/// capture references such as `$1` keep working exactly as before.
///
/// Passing `cancel == None` reproduces the pre-v0.1.36 behavior byte for byte,
/// including delegating to the upstream `regex` calls.
///
/// # Errors
///
/// Returns [`crate::error::AtomwriteError::Cancelled`] when `cancel` observes
/// a cancelled run.
fn apply_replacement_cancellable<'a>(
    pattern: &Regex,
    content: &'a str,
    replacement: &str,
    max_replacements: Option<usize>,
    preserve_case: bool,
    mut cancel: Option<&mut crate::signal::CancelCheck>,
) -> std::result::Result<(Cow<'a, str>, u64, u64), crate::error::AtomwriteError> {
    let count = match cancel.as_mut() {
        Some(check) => {
            let mut seen = 0u64;
            for _ in pattern.find_iter(content) {
                check.advance(1)?;
                seen += 1;
            }
            seen
        }
        None => pattern.find_iter(content).count() as u64,
    };

    if count == 0 {
        return Ok((Cow::Borrowed(content), 0, 0));
    }

    // Upstream `replacen` treats a limit of 0 as "replace everything". Apply
    // that rule once here so both branches below agree on `-n 0` instead of one
    // of them reading it as "replace nothing".
    let allowed = match max_replacements.filter(|n| *n > 0) {
        Some(n) => count.min(n as u64),
        None => count,
    };

    let Some(check) = cancel.as_mut() else {
        // Legacy path: callers without a check point keep the upstream calls,
        // so their behavior cannot drift from pre-v0.1.36.
        if preserve_case {
            let mut result = String::with_capacity(content.len());
            let mut last_end = 0;
            let mut applied = 0u64;
            for m in pattern.find_iter(content) {
                if applied >= allowed {
                    break;
                }
                result.push_str(&content[last_end..m.start()]);
                result.push_str(&adapt_case(m.as_str(), replacement));
                last_end = m.end();
                applied += 1;
            }
            result.push_str(&content[last_end..]);
            return Ok((Cow::Owned(result), applied, count.saturating_sub(applied)));
        }

        let replaced = match max_replacements {
            Some(n) if n > 0 => {
                let result = pattern.replacen(content, n, replacement);
                return Ok((
                    Cow::Owned(result.into_owned()),
                    allowed,
                    count.saturating_sub(allowed),
                ));
            }
            // `None` and `Some(0)` both mean "replace everything" upstream.
            _ => pattern.replace_all(content, replacement),
        };
        return Ok(match replaced {
            Cow::Borrowed(_) => (Cow::Borrowed(content), 0, count),
            Cow::Owned(s) => (Cow::Owned(s), count, 0),
        });
    };
    let mut result = String::with_capacity(content.len());
    let mut last_end = 0;
    let mut applied = 0u64;

    if preserve_case {
        for m in pattern.find_iter(content) {
            if applied >= allowed {
                break;
            }
            check.advance(1)?;
            result.push_str(&content[last_end..m.start()]);
            result.push_str(&adapt_case(m.as_str(), replacement));
            last_end = m.end();
            applied += 1;
        }
        result.push_str(&content[last_end..]);
        return Ok((Cow::Owned(result), applied, count.saturating_sub(applied)));
    }

    for caps in pattern.captures_iter(content) {
        if applied >= allowed {
            break;
        }
        check.advance(1)?;
        let Some(m) = caps.get(0) else {
            continue;
        };
        result.push_str(&content[last_end..m.start()]);
        caps.expand(replacement, &mut result);
        last_end = m.end();
        applied += 1;
    }
    result.push_str(&content[last_end..]);
    Ok((Cow::Owned(result), applied, count.saturating_sub(applied)))
}

/// Apply `replacement` to the matches of `pattern` without a check point.
///
/// Kept so callers written against the pre-v0.1.36 internal signature still
/// compile. The `replace` command no longer calls it: it passes a
/// [`crate::signal::CancelCheck`] so the loop stops mid-file when the global
/// deadline fires. It remains the reference implementation the cancellation
/// path is verified against in the module tests.
#[cfg(test)]
fn apply_replacement<'a>(
    pattern: &Regex,
    content: &'a str,
    replacement: &str,
    max_replacements: Option<usize>,
    preserve_case: bool,
) -> (Cow<'a, str>, u64) {
    apply_replacement_cancellable(
        pattern,
        content,
        replacement,
        max_replacements,
        preserve_case,
        None,
    )
    .map(|(text, applied, _suppressed)| (text, applied))
    .expect("a replacement run without a check point cannot observe a cancel")
}

fn build_walker(
    args: &ReplaceArgs,
    canonical_paths: &[std::path::PathBuf],
    global: &GlobalArgs,
) -> Result<ignore::WalkBuilder> {
    crate::commands::build_scoped_walker(canonical_paths, global, &args.include, &args.exclude)
}

#[cfg(test)]
mod apply_replacement_tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Neutral check point: the flag never flips, so the cancel path must
    /// produce exactly what the legacy path produces.
    fn armed_but_clear() -> crate::signal::CancelCheck {
        crate::signal::CancelCheck::for_bytes(Arc::new(AtomicBool::new(false)))
    }

    /// Run both paths over the same input and assert same text and same count.
    fn assert_paths_agree(
        pattern: &str,
        content: &str,
        replacement: &str,
        max_replacements: Option<usize>,
        preserve_case: bool,
    ) {
        let regex = Regex::new(pattern).expect("valid test regex");
        let (legacy_text, legacy_count) = apply_replacement(
            &regex,
            content,
            replacement,
            max_replacements,
            preserve_case,
        );
        let mut check = armed_but_clear();
        let (cancel_text, cancel_count, _suppressed) = apply_replacement_cancellable(
            &regex,
            content,
            replacement,
            max_replacements,
            preserve_case,
            Some(&mut check),
        )
        .expect("clear check point must not cancel");
        assert_eq!(
            cancel_text.as_ref(),
            legacy_text.as_ref(),
            "text diverged for pattern {pattern:?}"
        );
        assert_eq!(
            cancel_count, legacy_count,
            "count diverged for pattern {pattern:?}"
        );
    }

    #[test]
    fn cancel_path_matches_legacy_for_literal_patterns() {
        assert_paths_agree("foo", "foo bar foo baz foo", "X", None, false);
        assert_paths_agree("foo", "foo bar foo baz foo", "X", Some(2), false);
        assert_paths_agree("foo", "foo bar foo baz foo", "X", Some(1), false);
        assert_paths_agree("foo", "nothing here", "X", None, false);
    }

    /// Capture expansion must survive the switch from `replacen` to
    /// `captures_iter` + `Captures::expand`.
    #[test]
    fn cancel_path_expands_capture_references() {
        assert_paths_agree(r"(\w+),(\w+)", "a,b c,d e,f", "$2 $1", None, false);
        assert_paths_agree(r"(\w+),(\w+)", "a,b c,d e,f", "$2 $1", Some(2), false);
        assert_paths_agree(r"(\w+),(\w+)", "a,b c,d e,f", "${1}-${2}", None, false);
    }

    #[test]
    fn cancel_path_matches_legacy_with_preserve_case() {
        assert_paths_agree("foo", "FOO Foo foo", "bar", None, true);
        assert_paths_agree("foo", "FOO Foo foo", "bar", Some(2), true);
        assert_paths_agree("foo", "FOO Foo foo", "Bar Baz", None, true);
    }

    /// `-n 0` is documented as "all matches" by the upstream `replacen`. The
    /// cancellation path must agree so the limit semantics do not drift.
    #[test]
    fn cancel_path_matches_legacy_for_zero_limit() {
        assert_paths_agree("foo", "foo foo foo", "X", Some(0), false);
    }

    /// An empty match pattern is where `find_iter` and `captures_iter` could
    /// disagree silently. The counts and text must still line up.
    #[test]
    fn cancel_path_matches_legacy_for_empty_matches() {
        assert_paths_agree("x*", "axbxc", "-", None, false);
        assert_paths_agree("x*", "axbxc", "-", Some(3), false);
    }

    #[test]
    fn cancelled_flag_reports_cancelled_error() {
        let flag = Arc::new(AtomicBool::new(false));
        let check = crate::signal::CancelCheck::for_bytes(Arc::clone(&flag));
        assert!(check.check().is_ok());
        flag.store(true, Ordering::Release);
        let err = check.check().expect_err("flag set must cancel");
        assert!(
            matches!(err, crate::error::AtomwriteError::Cancelled { .. }),
            "expected Cancelled, got {err:?}"
        );
    }

    /// A flag already set must surface as a cancel before any rewrite happens.
    /// A block of 1 makes every match a check point.
    #[test]
    fn cancel_path_stops_before_writing_when_flag_is_set() {
        let regex = Regex::new("foo").expect("valid test regex");
        let flag = Arc::new(AtomicBool::new(true));
        let mut check = crate::signal::CancelCheck::new(flag, 1);
        let result = apply_replacement_cancellable(
            &regex,
            "foo foo foo",
            "X",
            None,
            false,
            Some(&mut check),
        );
        assert!(
            matches!(result, Err(crate::error::AtomwriteError::Cancelled { .. })),
            "expected Cancelled, got {result:?}"
        );
    }

    /// The flag is only loaded once a whole block of work has been consumed,
    /// which is what keeps the steady-state cost near zero.
    #[test]
    fn block_budget_polls_once_after_the_block() {
        let flag = Arc::new(AtomicBool::new(false));
        let mut check = crate::signal::CancelCheck::new(Arc::clone(&flag), 2);
        // First block closes on the second unit, but the flag is still clear.
        assert!(check.advance(1).is_ok());
        assert!(check.advance(1).is_ok());
        flag.store(true, Ordering::Release);
        // A single unit is inside the fresh block: no poll, so no cancel yet.
        assert!(check.advance(1).is_ok());
        // Closing the second block polls and observes the flag.
        assert!(check.advance(1).is_err());
    }

    /// A blocked run must not be reported as a partial rewrite: the error is
    /// the only signal, so the cancelled buffer is never handed to the caller.
    #[test]
    fn block_size_one_polls_on_every_unit() {
        let flag = Arc::new(AtomicBool::new(false));
        let mut check = crate::signal::CancelCheck::new(Arc::clone(&flag), 1);
        assert!(check.advance(1).is_ok());
        flag.store(true, Ordering::Release);
        assert!(check.advance(1).is_err());
    }
}
