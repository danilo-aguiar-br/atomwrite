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
/// v14 Tier 3: identifier case conversion (snake/camel/Pascal/kebab/SCREAMING).
pub mod case;
/// Atomic file copy with checksum verification.
pub mod copy;
/// Line, match, and extension counting.
pub mod count;
/// v14 Tier 3: structured config key removal.
pub mod del;
/// File deletion with optional backup.
pub mod delete;
/// Unified diff between two files.
pub mod diff;
/// Surgical file editing by line or marker.
pub mod edit;
/// Field extraction from NDJSON or text.
pub mod extract;
/// v14 Tier 3: structured config value reader.
pub mod get;
/// BLAKE3 checksum computation for files.
pub mod hash;
/// Directory listing with metadata.
pub mod list;
/// Atomic file move and rename.
pub mod r#move;
/// v14 Tier 3 (v0.1.12): tree-sitter S-expression query against a file.
pub mod outline;
/// v0.1.19 G121: workspace-relative path resolution helper for walking commands.
pub mod path_resolution;
/// v14 Tier 3 (v0.1.12): tree-sitter S-expression query against a file.
pub mod query;
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
/// v14 Tier 3: structured config value setter.
pub mod set;
/// Structural AST code search and rewrite.
pub mod transform;
/// G119 L5 — snapshot of WAL sidecar state (read-only, no I/O side effects).
pub mod wal_stats;
/// Atomic file creation and overwrite.
pub mod write;
