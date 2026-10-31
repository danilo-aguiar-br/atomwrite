// SPDX-License-Identifier: MIT OR Apache-2.0

//! Backup filename policy shared by every verb that skips or buckets backups.
//!
//! `list` and `count` bucket a timestamped backup under a dedicated extension
//! key instead of reading the stamp as a real extension, and the walk-based
//! verbs exclude the same shapes. One module owns the shape so those callers
//! cannot drift into two different notions of "this file is a backup".

use crate::constants::BACKUP_INFIX;

/// True for atomwrite backup filenames like `foo.txt.bak.20260615_035515`
/// (optionally with a `_NNN` millisecond suffix).
///
/// A plain byte check: the shape is fixed (`.bak.` + 8 digits + `_` + 6 digits
/// + optional `_` + 3 digits at the end of the name), so no regex is compiled.
pub(crate) fn is_backup_timestamp_name(name: &str) -> bool {
    use crate::constants::{
        BACKUP_STAMP_DATE_DIGITS as DATE, BACKUP_STAMP_MILLIS_DIGITS as MILLIS,
        BACKUP_STAMP_SEPARATOR as SEP, BACKUP_STAMP_TIME_DIGITS as TIME,
    };
    let Some((_, stamp)) = name.rsplit_once(BACKUP_INFIX) else {
        return false;
    };
    let b = stamp.as_bytes();
    let digits = |range: std::ops::Range<usize>| b[range].iter().all(u8::is_ascii_digit);
    let time_end = DATE + 1 + TIME;
    let millis_end = time_end + 1 + MILLIS;
    if b.len() != time_end && b.len() != millis_end {
        return false;
    }
    let base = digits(0..DATE) && b[DATE] == SEP && digits(DATE + 1..time_end);
    if b.len() == time_end {
        base
    } else {
        base && b[time_end] == SEP && digits(time_end + 1..millis_end)
    }
}

/// True when `path` looks like an atomwrite timestamped backup (`*.bak.*`).
///
/// DRY helper for recipe / semantic-search / hash excludes (A-008b / O-007).
pub(crate) fn is_backup_path(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name.contains(BACKUP_INFIX) || is_backup_timestamp_name(name))
}

/// A-027: owned exclude globs for walks that skip atomwrite backups.
#[must_use]
pub(crate) fn backup_exclude_globs() -> Vec<String> {
    crate::constants::BACKUP_EXCLUDE_GLOBS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// A-027: append backup exclude globs to an existing exclude list (idempotent-ish).
pub(crate) fn push_backup_excludes(exclude: &mut Vec<String>) {
    for g in crate::constants::BACKUP_EXCLUDE_GLOBS {
        if !exclude.iter().any(|e| e == *g) {
            exclude.push((*g).to_string());
        }
    }
}
