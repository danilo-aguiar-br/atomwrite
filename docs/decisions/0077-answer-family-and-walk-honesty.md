# ADR-0077 — Answer family, ignore-file honesty and target stamping on read verbs

## Status
Accepted (2026-10-30)

## Context
- The 0.1.37 audit measured `scope` publishing `files_matched` as `modified + failed`, so a directory that was never read counted as a match
- `transform --rules` walks the tree once per rule, so one unreadable file failed once per rule and `errors[]` repeated the same path
- A `.gitignore` with bad syntax or an unreadable mode ended the run with exit 0, `ok: true` and nothing on stderr, because `ignore` keeps that error inside its own value
- `case` exited 13 after renaming a file, while `search`, `replace`, `transform` and `scope` answered with exit 0 in the same situation
- `read`, `count`, `list`, `hash` and `diff` published no `target_resolved` and no `target_source`, so the agent could not confirm the path it read

## Decision
### 1. A match is a file that answered
- `scope` keeps a dedicated `matched_files` counter, incremented where `scopes_matched` is summed, and publishes it as `files_matched`
- A file that failed, a file that was skipped and a file with no match never reach that counter, so `files_matched` never exceeds `files_visited`
- The same rule already held in `search`, where a file counts only when it published an answer record

### 2. One failure per distinct file, whatever the rule count
- `transform --rules` keeps a `BTreeSet` of failed paths, and `files_failed` is the size of that set plus failures no entry could name plus missing roots
- `errors[]` lists each path once, and the per-rule `rule_error` event still names the rule that failed, so deduplication never hides which rule broke
- `files_visited`, `files_matched` and `total_replacements` keep summing per rule, because each rule is a full pass and the document says so

### 3. An ignore file that cannot be read is a reported failure
- `ignore_file_failures(roots)` reads the top `.gitignore` and `.ignore` of every root with `GitignoreBuilder::add`, which returns the parse or I/O error with the file path attached
- A syntax error is `INVALID_INPUT` and an unreadable file is the I/O code, both recorded through `FailureTally` with the ignore file's own path
- The check runs once per root, before the walk, and it covers the eleven scanning verbs that already carry a tally
- `scope` records the same failure into its atomic counter and its own event channel, so the verb reports it without a tally

### 4. The family rule is the answer, not the verb
- A verb whose answer is per item keeps exit 0 and publishes `partial: true` when it published at least one answer record: `search`, `replace`, `transform`, `case`, `scope`, `sparse`, `semantic-search` and `backup`
- A verb whose answer is a total over the tree exits with the first failure, because a total computed over a partial tree is false: `count`, `list`, `hash`, `copy`, `delete` and `prune-backups`
- A root named in the invocation that does not exist still exits 4 in every verb, even when the rest matched
- The guard is the verb's own answer, never a flag: `case` and `scope` need `files_modified > 0`, `sparse` needs at least one entry emitted, `semantic-search` at least one hit, and `backup` at least one copy on disk
- `scope` gained the escape `transform` already had, so a rewrite that landed is not discarded by a later write error, and `backup` stopped losing its summary when one source failed
- `prune-backups` stays in the total family: it answers "how many backups went away", and that total is false while a removal failed

### 5. Read verbs carry the same target stamp
- `read_target_roots` returns the roots of `read`, `stat`, `count`, `list`, `verify`, `get`, `query`, `outline`, `sparse`, `semantic-search`, `watch`, `extract-artifact`, `hash` and `diff`, and `resolve_read_targets` resolves them through `resolve_mutation_targets`
- `target_source` is `argv` for every typed root and `workspace` for the `.` default that `count`, `list`, `sparse list`, `sparse outline`, `semantic-search` and `watch` carry
- `hash --stdin` stamps `-` with `target_source: "stdin"`, because the subject of the run is the stream and naming it is more expressive than silence
- A `-` operand of `diff` stays the token instead of being resolved as a workspace path, so `diff a.txt -` names one file and one stream joined with `,` in argv order
- The error envelope consults the same resolver, so a failing run names its subject with the same origin the succeeding run uses, instead of falling back to `argv`
- A verb with no path of its own stays unstamped and is named as such: `extract`, `calc`, `regex`, `batch`, `agent-surface`, `doctor`, `commands`, `completions` without `--install`, `wal-stats`, reading `locale` and `recipe list`

### 6. A field description describes every producer
- `files_skipped` says the run did not answer that file, and names the cases the verbs use
- `files_visited` says total files examined everywhere, and `search --target files` counts what the walk handed over instead of the names that matched
- `files_matched` is what the names matched count lives in, so the page omitted by `--offset` and `--limit` is visible as `files_matched` against `total_matches`
- `files_modified` names every verb that publishes it: `replace`, `transform`, `case`, `scope`, `codemod`, `delete` and `copy`
- `total_matches` names `transform`'s replacements and `scope`'s matched scopes, which are the answer records those verbs publish
- `list` publishes `total_bytes` on every run, and the description no longer claims otherwise
- `delete` publishes `files_skipped` as the age-skipped count, and the tautological expression is gone

### 7. The gap inventory is not a live pointer
- `gaps.md` left the package `include`, and `**/gaps.md` stays in `exclude`, because a private note is never packaged
- Every document that cited it as a live source now points at the ADR series, and the historical mentions of `MIGRATION` stay as narrative of old releases

### 8. The rule holds in every mode of the same verb
- Single-rule `transform` answered with exit 0 when a replacement landed and another file failed, while `transform --rules` exited with the first failure code on the same tree
- The multi-rule path now carries the same escape, guarded by `total_replacements > 0`, and `errors[]` still names every file it could not touch
- A run that rewrote nothing keeps the first failure, because a campaign with no answer has nothing to keep

### 9. The failure list is a budget, not a constant
- `errors[]` stops at `summary.errors_max_entries`, 50 by default, and `files_failed` always keeps the full count
- The key accepts 1 to 1000, so a caller that needs more causes can ask for them and a caller that wants less can cut the record
- The ceiling exists because an unbounded list lets one broken tree push megabytes of failure text into a single record, which is the cost the cap avoids

## Consequences
- An agent that sums `files_matched` and `files_visited` gets two numbers that can be compared, and `files_matched` never counts a failure
- A campaign split into more rules no longer inflates `files_failed`, so a failure budget means the same thing at any rule count
- A tree whose ignore file is broken now says so, with the path of the ignore file and the line that broke
- `case` no longer throws away a rename it performed, and the CI of a caller stops failing over a file the run never touched
- A campaign of several rules keeps the same exit code as the same campaign of one rule, so splitting work never changes what the exit code means
- The five read verbs answer the ETD question the writing verbs already answered
