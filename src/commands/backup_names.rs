// SPDX-License-Identifier: MIT OR Apache-2.0

//! Backup filename policy shared by every verb that skips or buckets backups.
//!
//! `list` and `count` bucket a timestamped backup under a dedicated extension
//! key instead of reading the stamp as a real extension, and the walk-based
//! verbs exclude the same shapes. One module owns the shape so those callers
//! cannot drift into two different notions of "this file is a backup".

use crate::constants::BACKUP_INFIX;

/// Sentence every verb uses when a requested backup targets a backup already.
///
/// One constant, because `delete` and the `batch` delete answer the same
/// question and two copies of the sentence would drift into two answers.
pub(crate) const BACKUP_OF_LINK_SKIPPED: &str =
    "the target is a link; it is removed without copying what it points at";

/// Sentence `delete` answers for a fifo, socket or device.
///
/// The entry is unlinked without being opened, so there is no content to copy
/// and the refusal names the reason instead of leaving `backup_path` empty.
pub(crate) const BACKUP_OF_SPECIAL_SKIPPED: &str =
    "the target is not a regular file; it is removed without being copied";

pub(crate) const BACKUP_OF_BACKUP_SKIPPED: &str =
    "the target is itself a backup copy; no copy of a copy is made";

/// Sentence `delete` answers when the caller also asked for `--keep-backup`.
///
/// A deletion copy is never auto-removed, so the flag only keeps a copy that
/// exists. It does not create one: `--backup` or `[defaults].backup = true` is
/// what asks for a copy on `delete`, and the sentence says so instead of
/// implying a backup.
pub(crate) const KEEP_BACKUP_REDUNDANT_FOR_DELETE: &str = "--keep-backup only preserves an existing delete copy; pass --backup or set [defaults].backup = true to create one";

/// Sentence the `batch` answers when the transaction already holds the copy.
///
/// A transactional run snapshots every target before the first mutation, so a
/// per-operation copy of the same file would double the bytes on disk without
/// adding a second way back. The operation names the snapshot it found instead
/// of writing a sibling, and says so rather than staying silent about the copy
/// it did not create.
pub(crate) const TRANSACTION_SNAPSHOT_SUPERSEDES_COPY: &str =
    "the transaction already preserved this target: no second copy was made";

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

/// The creation moment encoded in a timestamped backup name.
///
/// Returns `None` when the name carries no valid stamp, which is the signal
/// that the name is not a backup this tool created and its age must come from
/// the filesystem instead.
///
/// The stamp is UTC and follows the name, so it survives a copy tool that
/// rewrites the mtime with the source file's own time: a fresh backup of an old
/// file used to look old and was pruned in the same breath.
pub(crate) fn parse_backup_timestamp(name: &str) -> Option<std::time::SystemTime> {
    if !is_backup_timestamp_name(name) {
        return None;
    }
    let (_, stamp) = name.rsplit_once(crate::constants::BACKUP_INFIX)?;
    let bytes = stamp.as_bytes();
    let number = |range: std::ops::Range<usize>| stamp.get(range)?.parse::<u64>().ok();
    let year = number(0..4)?;
    let month = number(4..6)?;
    let day = number(6..8)?;
    let hour = number(9..11)?;
    let minute = number(11..13)?;
    let second = number(13..15)?;
    let millis = if bytes.len() > 16 { number(16..19)? } else { 0 };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    let secs = crate::atomic::utc_to_epoch(
        year as i64,
        month as i64,
        day as i64,
        hour,
        minute,
        second.min(59),
    );
    Some(
        std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(secs)
            + std::time::Duration::from_millis(millis),
    )
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
