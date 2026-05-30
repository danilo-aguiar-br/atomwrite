// SPDX-License-Identifier: MIT OR Apache-2.0

//! Subcommand handler implementations for all atomwrite operations.

/// Patch application from stdin (unified diff, SEARCH/REPLACE, full file).
pub mod apply;
/// Standalone file backup with BLAKE3 checksums.
pub mod backup;
/// Batch operation execution from NDJSON manifest.
pub mod batch;
/// Math expression evaluation via fend.
pub mod calc;
/// Atomic file copy with checksum verification.
pub mod copy;
/// Line, match, and extension counting.
pub mod count;
/// File deletion with optional backup.
pub mod delete;
/// Unified diff between two files.
pub mod diff;
/// Surgical file editing by line or marker.
pub mod edit;
/// Field extraction from NDJSON or text.
pub mod extract;
/// BLAKE3 checksum computation for files.
pub mod hash;
/// Directory listing with metadata.
pub mod list;
/// Atomic file move and rename.
pub mod r#move;
/// File reading with metadata and content.
pub mod read;
/// Regex generation from examples via grex.
pub mod regex_gen;
/// Parallel text replacement with atomic writes.
pub mod replace;
/// File restoration from backup.
pub mod rollback;
/// Grammatical scoping with AST-based actions.
pub mod scope;
/// Parallel file content search via ripgrep.
pub mod search;
/// Structural AST code search and rewrite.
pub mod transform;
/// Atomic file creation and overwrite.
pub mod write;
