# ADR-0047: scope read-only mode fix and find_all migration

- **Status**: Accepted
- **Date**: 2026-06-21
- **Context**: The `scope` subcommand reported `files_matched: 0` when invoked without an action (`--delete`, `--action`, or `--replace-with`). Investigation revealed the root cause was NOT the matching mechanism (DFS + `match_node` works correctly) but the result counting logic. When no action is configured, `apply_scope_action()` returns the original text unchanged. After applying the "edits" (which are identity operations), `checksum_before == checksum_after`, causing the code to count the file as "skipped" and never emit a `ScopeResult` event. This made the read-only/audit mode of scope completely inoperative — users could not list which AST nodes matched a pattern without also specifying a mutation. Additionally, the matching used `root.dfs().filter(|node| pattern.match_node(node.clone()).is_some())` which bypasses the `potential_kinds()` pre-filtering that `Node::find_all()` provides.

- **Decision**: Two changes applied:
  1. **Read-only mode branch**: After collecting matches and before the edit loop, detect `is_read_only = !delete && action.is_none() && replace_with.is_none()`. When true, emit `ScopeResult` with match count, increment `files_matched`, and return early without checksum comparison or write attempt.
  2. **DFS → find_all migration**: Replace the manual `root.dfs().filter(...)` with `root.find_all(&pattern)`. The `find_all` method (ast-grep-core 0.43.0, `Node::find_all`) calls `pat.potential_kinds()` once and uses the result as a short-circuit filter before invoking `match_node` on each candidate. This reduces unnecessary `match_node` calls on nodes whose `kind_id` can never match the pattern root. `find_all` returns `Iterator<Item = NodeMatch>` where `NodeMatch: Deref<Target = Node>`, so `.range()` works unchanged downstream.

- **Consequences**:
  - **+** `scope` without action now correctly reports matches (read-only/audit mode works).
  - **+** Performance improvement from `potential_kinds()` pre-filtering in `find_all`.
  - **+** Consistency with `transform` which uses `replace_all` (internally uses the same Visitor/find_all mechanism).
  - **+** All 30+ prepared queries produce results when invoked in read-only mode.
  - **+** Removed unused `MatcherExt` import.
  - **-** (none) Mutation mode (`--delete`, `--action`, `--replace-with`) continues to work unchanged.

- **Alternatives considered**:
  1. **Only add read-only branch, keep manual DFS.** Rejected: find_all is strictly better (same semantics + performance from kind pre-filtering) and is the idiomatic ast-grep-core API.
  2. **Use `Visitor` directly instead of `find_all`.** Rejected: `Visitor` is lower-level and requires more boilerplate; `find_all` encapsulates the same logic in a one-liner.
  3. **Emit a warning when no action is specified.** Rejected: read-only mode is a legitimate use case (audit, count, list matches) and should work silently.

- **Trigger to revisit**: If ast-grep-core changes the `find_all` API or if a streaming/non-collecting approach is needed for very large files.
