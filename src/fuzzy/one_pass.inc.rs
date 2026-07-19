// match_block_anchor / match_context_aware → windows.rs

/// Result of [`apply_fuzzy_one_pass`] (v0.1.33).
#[derive(Debug, Clone)]
pub struct FuzzyOnePassResult {
    /// Edited buffer (or original when `applied == 0`).
    pub edited: String,
    /// Number of successful applies (never re-scans inserted text).
    pub applied: u64,
    /// Last successful match diagnostics.
    pub info: Option<FuzzyInfo>,
    /// True when `replacement` contains `pattern` (forces single apply).
    pub replacement_embeds_pattern: bool,
}

#[inline]
fn check_cancel() -> std::result::Result<(), AtomwriteError> {
    if crate::signal::is_global_shutdown() {
        return Err(crate::signal::cancelled_error(
            "fuzzy match cancelled by signal or --timeout-secs",
        ));
    }
    Ok(())
}

fn max_edited_len(input_len: usize) -> usize {
    let by_factor = input_len.saturating_mul(crate::constants::FUZZY_MAX_BUFFER_GROWTH_FACTOR);
    let by_bytes = input_len.saturating_add(crate::constants::FUZZY_MAX_BUFFER_GROWTH_BYTES);
    by_factor.max(by_bytes)
}

/// One-pass fuzzy apply for multi-file `replace` (v0.1.33 one-shot).
///
/// Never re-scans text that was just inserted (sed / `str::replacen` semantics).
/// Default when `max_replacements` is `None`: **1** apply.
/// When `replacement` contains `pattern`, force **1** apply even if a higher
/// `--max-replacements` was requested (prevents infinite growth).
///
/// Multi-hit (`max_replacements > 1`) advances a cursor on the **original**
/// content only: each subsequent search starts after the previous match end.
pub fn apply_fuzzy_one_pass(
    content: &str,
    pattern: &str,
    replacement: &str,
    opts: MatchOpts,
    max_replacements: Option<usize>,
) -> std::result::Result<FuzzyOnePassResult, AtomwriteError> {
    check_cancel()?;
    if pattern.is_empty() {
        return Err(AtomwriteError::InvalidInput {
            reason: "old string must not be empty".into(),
        });
    }
    if pattern.len() > crate::constants::FUZZY_MAX_PATTERN_BYTES {
        return Err(AtomwriteError::InvalidInput {
            reason: format!(
                "fuzzy pattern too large ({} bytes > {} max); shorten the block",
                pattern.len(),
                crate::constants::FUZZY_MAX_PATTERN_BYTES
            ),
        });
    }

    let embeds = replacement.contains(pattern);
    let mut limit = max_replacements
        .map(|n| n as u64)
        .unwrap_or(crate::constants::FUZZY_DEFAULT_MAX_REPLACEMENTS);
    if embeds {
        limit = 1;
    }
    limit = limit.min(crate::constants::FUZZY_HARD_MAX_REPLACEMENTS);
    if limit == 0 {
        return Ok(FuzzyOnePassResult {
            edited: content.to_string(),
            applied: 0,
            info: None,
            replacement_embeds_pattern: embeds,
        });
    }

    // Fast path: single apply (agent default / embeds).
    if limit == 1 {
        match match_pair_with(content, pattern, replacement, opts) {
            Ok((edited, info)) => {
                let cap = max_edited_len(content.len());
                if edited.len() > cap {
                    return Err(AtomwriteError::InvalidInput {
                        reason: format!(
                            "fuzzy edit would grow buffer from {} to {} bytes (cap {cap}); aborting for one-shot safety",
                            content.len(),
                            edited.len()
                        ),
                    });
                }
                Ok(FuzzyOnePassResult {
                    edited,
                    applied: 1,
                    info: Some(info),
                    replacement_embeds_pattern: embeds,
                })
            }
            Err(AtomwriteError::Cancelled { .. }) => Err(crate::signal::cancelled_error(
                "fuzzy match cancelled by signal or --timeout-secs",
            )),
            Err(_) => Ok(FuzzyOnePassResult {
                edited: content.to_string(),
                applied: 0,
                info: None,
                replacement_embeds_pattern: embeds,
            }),
        }
    } else {
        // Multi-hit on ORIGINAL content with advancing cursor (never search inside inserts).
        let mut pos = 0usize;
        let mut out = String::new();
        let cap = max_edited_len(content.len());
        out.try_reserve(content.len().min(cap)).map_err(|e| {
            AtomwriteError::InvalidInput {
                reason: format!("failed to reserve edit buffer: {e}"),
            }
        })?;
        let mut applied = 0u64;
        let mut last_info: Option<FuzzyInfo> = None;

        while applied < limit {
            check_cancel()?;
            if pos >= content.len() {
                break;
            }
            let slice = &content[pos..];
            match match_pair_with(slice, pattern, replacement, opts) {
                Ok((edited_slice, info)) => {
                    // Locate the matched span by recovering the preimage:
                    // edited_slice = prefix + replacement_adjusted + suffix of slice.
                    // Prefer exact pattern position in slice; else first line-anchor heuristic.
                    let (rel_start, rel_end, adjusted_new) =
                        locate_applied_span(slice, pattern, replacement, &edited_slice, &info)
                            .unwrap_or((0, slice.len(), edited_slice.clone()));
                    out.push_str(&content[pos..pos + rel_start]);
                    out.push_str(&adjusted_new);
                    if out.len() > cap {
                        return Err(AtomwriteError::InvalidInput {
                            reason: format!(
                                "fuzzy multi-edit exceeded growth cap ({cap} bytes); aborting"
                            ),
                        });
                    }
                    pos += rel_end;
                    applied += 1;
                    last_info = Some(info);
                    // Advance at least one byte on zero-width edge cases.
                    if rel_end == rel_start {
                        pos = pos.saturating_add(1);
                    }
                }
                Err(AtomwriteError::Cancelled { .. }) => {
                    return Err(crate::signal::cancelled_error(
                        "fuzzy match cancelled by signal or --timeout-secs",
                    ));
                }
                Err(_) => break,
            }
        }
        out.push_str(&content[pos..]);
        Ok(FuzzyOnePassResult {
            edited: out,
            applied,
            info: last_info,
            replacement_embeds_pattern: embeds,
        })
    }
}

/// Recover (start, end exclusive, `adjusted_new`) of the first apply inside `slice`.
fn locate_applied_span(
    slice: &str,
    pattern: &str,
    replacement: &str,
    edited_slice: &str,
    _info: &FuzzyInfo,
) -> Option<(usize, usize, String)> {
    if let Some(start) = find_str(slice, pattern) {
        let end = start + pattern.len();
        let prefix = &slice[..start];
        let suffix = &slice[end..];
        if edited_slice.starts_with(prefix)
            && edited_slice.ends_with(suffix)
            && edited_slice.len() >= prefix.len() + suffix.len()
        {
            let adj = edited_slice[prefix.len()..edited_slice.len() - suffix.len()].to_string();
            return Some((start, end, adj));
        }
        return Some((start, end, replacement.to_string()));
    }
    // Fuzzy: derive from longest common prefix/suffix between slice and edited.
    let prefix = common_prefix_len(slice.as_bytes(), edited_slice.as_bytes());
    // Avoid claiming the entire slice when edit failed to shrink/grow sanely.
    if prefix == slice.len() && edited_slice == slice {
        return None;
    }
    let a = &slice.as_bytes()[prefix..];
    let b = if prefix <= edited_slice.len() {
        &edited_slice.as_bytes()[prefix..]
    } else {
        &[]
    };
    let suffix = common_suffix_len(a, b);
    if prefix + suffix > slice.len() {
        return None;
    }
    let end = slice.len() - suffix;
    if end <= prefix {
        return None;
    }
    let adj_end = edited_slice.len().saturating_sub(suffix);
    if adj_end < prefix {
        return None;
    }
    let adjusted = edited_slice[prefix..adj_end].to_string();
    Some((prefix, end, adjusted))
}

fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

fn common_suffix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

