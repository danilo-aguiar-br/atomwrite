# atomwrite Cookbook


[Leia em Português](COOKBOOK.pt-BR.md)

- Practical recipes you can copy-paste into your agent workflows


## What's New in v0.1.37 (2026-10-31)
- Pin agents to `^0.1.37`. 47 subcommands (SSOT via `agent-surface`); the new one is `config`
- One terminal event per run: a failing summary carries `ok:false`, `code` and `exit` itself, and every summary that publishes `files_failed` adds `partial: true` and `errors[]` beside it
- `edit-loop` with a pair that has no match exits 65 with `MATCH_FAILED`

### New Recipes (v0.1.37)
- How to Read and Change a Tuning Key with config
  - `atomwrite --workspace "$WS" config get fuzzy.max_content_bytes` shows the effective value and its origin (`default`, `global`, `workspace` or `explicit`)
  - `atomwrite --workspace "$WS" config set fuzzy.max_content_bytes 134217728 --scope workspace` writes the key into `.atomwrite.toml` atomically (`--scope global|workspace` is required)
  - `atomwrite --workspace "$WS" config unset fuzzy.max_content_bytes --scope workspace` removes it again
- How to Read the Files a Scan Could Not Reach
  - `atomwrite --workspace "$WS" search TODO src/ | jaq -c 'select(.type=="summary") | {files_failed, errors}'` lists each failed file with `path`, `code` and `message` (at most `summary.errors_max_entries` entries (50 by default); `files_failed` keeps the full count; both keys are absent when every file was reached)
- How to Branch on the Exit Code
  - 65: read `best_candidate`; 78: config problem or a verb missing from a slim build; 83: lock not acquired; 124: deadline, raise `--timeout-secs` or narrow the path; 141: the reader closed the pipe; 143: SIGTERM
- Recipes added for every command that had no example -- the sections `How to Manage Tuning Keys and Scopes with config` to `How to Shape a Stream Without jq`, below `How to Observe Batch Progress`
  - `config`, `set`, `get`, `del`, `copy`, `move`, `count`, `case`, `locale`, `query`, `outline` and `extract-artifact`
  - `edit-loop`, `backup`, `rollback`, `prune-backups`, `wal-stats`, `wal-heal`, `doctor`, `--print-schema`, the `rules.yaml` format, the exit-code branch and the single terminal event


## What's New in v0.1.36 (2026-07-31)
- Historical (v0.1.36): 46 subcommands (SSOT via `agent-surface`)
- Agent-native shape: `--select`/`--fields`, `--filter`, `--limit` (no global `-n`). Errors always emit, except a broken pipe and a checksum divergence already published as one `hash_verify` event
- Staging: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch` then `--old-file`/`--new-file` under `.atomwrite/scratch/`
- Thin agent loops without jq: `atomwrite --select ok,error,code --filter 'ok=false' --limit 1 <cmd>`
- GAP-019…033 closed/mitigated (content-source jail + `ok:false` errors + honest fuzzy)

### New Recipes (v0.1.36)
- How to Stage a Patch with scratch init -- `--old-file`/`--new-file` inside the jail
- How to Project NDJSON with --select/--filter/--limit -- agent-native shape without jq
- How to Pass a Literal `$VAR` with a Single-Quoted Heredoc -- shell-safe content
- How to Discover Host Readiness with doctor + commands -- agent bootstrap
- How to Intentional Shrink Rewrite with --allow-shrink -- override shrink guard
- How to Use Aggressive Replace Only After --dry-run -- fail-closed mass edits


## 47 Subcommands (SSOT)

Recipes below cover common agent paths; the complete surface is 47 tools (`CANONICAL_SUBCOMMAND_COUNT` / `atomwrite agent-surface`). Discovery:

```bash
atomwrite agent-surface
atomwrite commands --include-globals
```

- `agent-surface` — clap-derived tool inventory (integration by subprocess, CLI with NDJSON output)
- `apply` — apply patches from stdin (unified, search/replace, full, markdown)
- `backup` — create timestamped backups with BLAKE3 checksums
- `batch` — NDJSON-driven multi-op batch (optional transaction)
- `calc` — math expressions and unit conversions
- `case` — rename identifiers across files (`--subvert` + `--to` style), e.g. `atomwrite case --subvert old_name new_name src/main.rs`
- `codemod` — multi-rule YAML codemod campaign (delegates to transform), e.g. `atomwrite codemod --rules rules.yaml src/`
- `commands` — emit full clap command tree as JSON for agent discovery
- `completions` — generate shell completions (bash, zsh, fish, elvish, powershell)
- `config` — persistent tuning keys (`path|list|get|set|unset`, `--scope global|workspace`), e.g. `atomwrite config set fuzzy.threshold 0.9 --scope workspace`
- `copy` — copy files with checksum verification, e.g. `atomwrite copy a.txt b.txt`
- `count` — count lines, matches, or files by extension, e.g. `atomwrite count src/`
- `del` — delete a key from a structured config file (TOML/JSON), e.g. `atomwrite del Cargo.toml workspace.members`
- `delete` — delete files with optional backup (`--plan` plan-only)
- `diff` — compare two files (unified, stat, or changes)
- `doctor` — diagnose environment and host readiness
- `edit` — surgically edit by line, marker, or exact match
- `edit-loop` — apply N `old`/`new` pairs from NDJSON stdin in one atomic write, e.g. `atomwrite edit-loop src/main.rs < pairs.ndjson`
- `extract` — extract fields from NDJSON via pipe (local `--field`; never local `--select`)
- `extract-artifact` — bounded signal from PDF, OOXML and image headers (never raw artifact bytes), e.g. `atomwrite extract-artifact report.pdf`
- `get` — read a value at a dotted path in TOML/JSON, e.g. `atomwrite get Cargo.toml package.version`
- `hash` — calculate BLAKE3 checksums
- `list` — list files in a directory tree
- `locale` — show / set / clear UI locale preference (XDG)
- `move` — move or rename files atomically (EXDEV copy-fallback), e.g. `atomwrite move a.txt b.txt`
- `outline` — extract high-level AST structure (functions, classes, …), e.g. `atomwrite outline src/main.rs`
- `prune-backups` — prune `.bak.*` backups by age or count, e.g. `atomwrite prune-backups --max-count 5 .`
- `query` — walk a tree-sitter AST and emit nodes as NDJSON, e.g. `atomwrite query --kinds src/main.rs`
- `read` — read files with metadata, checksum, optional content
- `recipe` — named multi-step recipes (`list` / `run`), e.g. `atomwrite recipe list`
- `regex` — generate regex from examples
- `replace` — replace text across files with atomic writes
- `rollback` — restore a file from a previous backup
- `scope` — grammatical scoping (delete comments, case transforms, …)
- `scratch` — staging under `.atomwrite/scratch/`; never OS `/tmp` outside workspace (exit 126), e.g. `atomwrite scratch init .atomwrite/scratch`
- `search` — search file contents in parallel (ripgrep engine)
- `semantic-merge` — three-way line-based merge (not AST/embedding), e.g. `atomwrite semantic-merge --base b.txt --ours o.txt --theirs t.txt --output m.txt`
- `semantic-search` — offline token Jaccard search (no network), e.g. `atomwrite semantic-search 'atomic write' src/`
- `set` — write a value at a dotted path in TOML/JSON, e.g. `atomwrite set Cargo.toml package.name demo`
- `sparse` — budgeted sparse list/read for agent context windows, e.g. `atomwrite sparse list src/ --max-files 20`
- `stat` — metadata alias for `read --stat` (no body), e.g. `atomwrite stat src/main.rs`
- `transform` — AST refactoring via ast-grep
- `verify` — verify file integrity by BLAKE3 checksum, e.g. `atomwrite verify a.txt <checksum>`
- `wal-heal` — reap stale terminal WAL journals (safe, idempotent), e.g. `atomwrite wal-heal .`
- `wal-stats` — local WAL diagnostics snapshot (counts, age, reclaim estimate), e.g. `atomwrite wal-stats`
- `watch` — watch filesystem events (feature `watch`), e.g. `atomwrite watch src/`
- `write` — create or overwrite files atomically via stdin


## What's New in v0.1.35 (2026-07-19)
- Residual A-* gaps closed (ADR-0055): suite green without `delete --yes`; large overwrite default-deny (`--ack-overwrite`); `watch` always emits `watch_summary`; semantic-merge conflict markers default ON; monólitos `include!` SRP splits
- Write large files: existing targets above XDG `[write].confirm_large_bytes` (default 100 KiB) require `--ack-overwrite` (agent one-shot; no interactive Y/N). `--confirm` is a legacy awareness flag; `--require-large-ack` is independent (no alias collision)
- Delete: use `--plan` or `--dry-run` for plan-only; `delete --confirm` and `delete --yes`/`-y` are rejected (fail-closed). Run again without those flags to delete
- Watch: final NDJSON `type:watch_summary` on every exit path; idle default 500 ms (`--idle-exit-ms` / XDG `[watch].idle_exit_ms`); debounce via CLI or XDG `[watch].debounce_ms`
- Semantic-merge: conflict markers written by default; opt-out `--no-conflict-markers`
- Local DoD (run by hand on the workstation): `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`, `cargo check --target x86_64-pc-windows-gnu`, `cargo install --path . --force`
- Contract suite: `cargo test --test cli_e2e_v0135`
- Historical pin `^0.1.35` (superseded by `^0.1.36`). Configuration is CLI + XDG / `.atomwrite.toml` only (no product `ATOMWRITE_*` runtime knobs; no product telemetry)


## What's New in v0.1.34

v0.1.34 (2026-07-19) docs-complete publish of the v0.1.33 one-shot runtime: fuzzy multi-apply is one-pass L→R (`apply_fuzzy_one_pass`); default max applies 1; embeds force single apply; global `--timeout-secs` / `--timeout` default 120 (`0` disables, deadline exit 124). Historical (at v0.1.34): 41 subcommands.

### New Recipes (v0.1.34)
- How to Expand a Section with Fuzzy Replace Safely -- NEW contains OLD; one-pass, no hang
- Global timeout default 120s — long monorepo jobs use `--timeout-secs 0` or a higher value

### New Recipes (v0.1.29 surface + v0.1.30 residual)
- How to Fuzzy-Replace Across a Monorepo -- `replace --fuzzy auto` with optional `--progress-every`
- How to Run search-replace-verify -- `recipe run --name search-replace-verify`
- How to Budget a Sparse List -- `sparse list --max-files` / `--max-bytes`
- How to Three-Way Semantic Merge (line-based, not AST) -- `semantic-merge --base --ours --theirs`
- How to Choose write Durability -- `--durability full|fast|auto`
- How to Install Slim vs Full -- `--no-default-features --features core` vs default
- How to Recover from a Missed Match with best_candidate -- inspect error envelope near-miss and retry
- How to Inspect rename_method and durability -- `platform.*` via `jaq` (Linux `renameat2`)
- How to Read Honest Backup Method -- `platform.backup_method` is `reflink_or_copy` (never hardlink of live file)
- How to Inventory the Agent Surface -- `agent-surface`
- How to Stat Without Reading Body -- `stat` (alias of `read --stat`)
- How to Watch with Debounce -- `watch` (feature `watch`)
- How to Semantic-Search Offline (Jaccard) -- `semantic-search` (always compiled), `--index-dir`
- How to Run a Codemod Campaign -- `codemod --dry-run`, `by_rule_id`
- How to Observe Batch Progress -- batch heartbeats + `--progress-every` on replace


## How to Expand a Section with Fuzzy Replace Safely
- Agent expand-section edits often produce NEW that contains OLD (the original block plus more lines)
- Before v0.1.33, re-scanning inserted text could hang for days; one-pass L→R never re-scans inserted text
- When replacement contains pattern (fixed-string), atomwrite forces a single apply even if `--max-replacements` is huge
- Default max fuzzy applies is 1 when `--max-replacements` is omitted (hard ceiling 10_000)
- Global `--timeout-secs` defaults to 120 (exit 124 on deadline); pass `0` only if you intentionally want unlimited runtime

```bash
# NEW embeds OLD — safe, finishes in one pass (no hang)
atomwrite --workspace . replace --fuzzy auto \
  $'## Setup\n\nInstall deps.' \
  $'## Setup\n\nInstall deps.\n\n### Extra\nAlso run migrations.' \
  docs/README.md

# Explicit multi-hit still one-pass on original content
atomwrite --workspace . replace --fuzzy auto --max-replacements 3 \
  'TODO' 'DONE' src/
```


## How to Fuzzy-Replace Across a Monorepo
- Use fuzzy replace when indentation or whitespace diverges across packages
- Prefer `--fuzzy auto` (default cascade for agents); `--fuzzy off` = exact-only, not rejected
- Emit progress NDJSON on large trees with `--progress-every`
- Remember global timeout default is 120s (exit 124); use `--timeout-secs 0` for unbounded monorepo sweeps
- Fuzzy max applies hard ceiling is 10_000; default remains 1 when `--max-replacements` is omitted

```bash
# Preview first
atomwrite --workspace . replace --dry-run --fuzzy auto \
  'legacy_api(' 'new_api(' packages/ --include '*.rs'

# Apply with progress every 50 files
atomwrite --workspace . replace --fuzzy auto --progress-every 50 \
  'legacy_api(' 'new_api(' packages/ --include '*.rs'
```


## How to Run search-replace-verify
- Built-in recipe searches, replaces, then hashes for integrity
- Requires `--pattern` and `--replacement`

```bash
atomwrite --workspace . recipe list
atomwrite --workspace . recipe run --name search-replace-verify \
  --path src --pattern OLD_TOKEN --replacement NEW_TOKEN --fuzzy auto
```


## How to Budget a Sparse List
- Cap file count and path-byte budget for agent context windows
- The paths file of `sparse read` must be inside the workspace: a path outside exits 126

```bash
atomwrite --workspace . sparse list --max-files 100 --max-bytes 1048576 src/
atomwrite --workspace . sparse list --max-files 20 --include '*.rs' .
# Feed paths into sparse read (the paths file stays inside the workspace)
atomwrite --workspace . scratch init .atomwrite/scratch
atomwrite --workspace . sparse list --max-files 20 src/ \
  | jaq -r 'select(.type=="sparse_entry") | .path' > .atomwrite/scratch/paths.txt
atomwrite --workspace . sparse read --paths-file .atomwrite/scratch/paths.txt --head 30 --max-files 20
```


## How to Three-Way Semantic Merge (line-based, not AST)
- Merge concurrent agent edits with a common base
- Base, ours, theirs and output must be inside the workspace: a path outside exits 126
- By default a conflict is written as `<<<<<<<` markers in the output, the event says `status: "conflict"` and the run exits 0
- Add `--fail-on-conflict` to exit 84 `MERGE_CONFLICT` on unresolved conflicts (the output file is not written), or `--no-conflict-markers` to keep ours without markers

```bash
atomwrite --workspace . scratch init .atomwrite/scratch
# .atomwrite/scratch/{base,ours,theirs}.rs hold the three versions
atomwrite --workspace . semantic-merge \
  --base .atomwrite/scratch/base.rs \
  --ours .atomwrite/scratch/ours.rs \
  --theirs .atomwrite/scratch/theirs.rs \
  --output src/merged.rs --fail-on-conflict
```


## How to Choose write Durability
- `full` for configs and durable state
- `fast` for ephemeral artifacts
- `auto` for default policy resolution

```bash
echo critical | atomwrite --workspace . write --durability full config.toml
# Ephemeral output stays inside the workspace jail (never OS /tmp)
atomwrite --workspace . scratch init .atomwrite/scratch
echo scratch | atomwrite --workspace . write --durability fast .atomwrite/scratch/ephemeral-out.txt
# Or a workspace-relative staging dir:
mkdir -p ./tmp-agent
echo scratch | atomwrite --workspace . write --durability fast ./tmp-agent/ephemeral-out.txt
```


## How to Stage a Patch with scratch init
- Content-source paths for `--old-file`/`--new-file` are jail-checked (GAP-019)
- Never stage under OS `/tmp` — exit 126 `WORKSPACE_JAIL`
- Canonical staging: `{workspace}/.atomwrite/scratch/` via `scratch init`

```bash
atomwrite --workspace . scratch init .atomwrite/scratch
SCR=.atomwrite/scratch
printf %s "$OLD" >"$SCR/old.txt"
printf %s "$NEW" >"$SCR/new.txt"
atomwrite --workspace . edit --old-file "$SCR/old.txt" --new-file "$SCR/new.txt" src/f.rs
```


## How to Project NDJSON with --select/--filter/--limit
- Global shape flags apply to every subcommand (and error path)
- `--select`/`--fields` project top-level keys; `--filter` is top-level equality (repeatable AND)
- `--limit N` is long-only (no global `-n`); errors always emit even under filter, except a broken pipe and a checksum divergence already published as one `hash_verify` event
- Search hits carry the matched text in `lines` (there is no `text` key)

```bash
# Compact agent parse without jq
atomwrite --workspace . --select ok,error,code,match_count,suggestion \
  edit --old missing --new x --fuzzy off src/f.rs

# First failure only
atomwrite --workspace . --filter ok=false --limit 1 \
  edit --old a --new b src/f.rs

# Project success fields on search hits
atomwrite --workspace . --select path,line_number,lines --limit 20 \
  search 'TODO' src/
```


## How to Pass a Literal `$VAR` with a Single-Quoted Heredoc
- Double quotes and unquoted heredocs expand shell variables
- Single-quoted heredoc delimiter preserves `$VAR`, backticks, and `//` literally
- Prefer `--old-file`/`--new-file` for multi-line patches (still stage under workspace)

```bash
# Literal $VAR and shell metacharacters — no expansion
atomwrite --workspace . edit src/profile --old 'export PATH=$PATH' --new 'export PATH=$HOME/bin:$PATH'

# Multi-line literal body via single-quoted heredoc → staged file
atomwrite --workspace . scratch init .atomwrite/scratch
SCR=.atomwrite/scratch
cat >"$SCR/old.txt" <<'EOF'
# install
curl -fsSL https://example.com | sh
export TOKEN=$SECRET
EOF
cat >"$SCR/new.txt" <<'EOF'
# install
curl -fsSL https://example.com | sh
export TOKEN=$SECRET
# pinned
EOF
atomwrite --workspace . edit docs/install.md \
  --old-file "$SCR/old.txt" --new-file "$SCR/new.txt"
```


## How to Discover Host Readiness with doctor + commands
- `doctor` reports host/workspace/feature readiness as one NDJSON object
- `commands` emits the full clap tree for agent discovery
- Pair with `agent-surface` for the tool manifesto (integration by subprocess, CLI with NDJSON output)

```bash
atomwrite --workspace . doctor --strict
atomwrite --workspace . commands --include-globals
atomwrite --workspace . agent-surface --format json
```


## How to Intentional Shrink Rewrite with --allow-shrink
- The shrink guard blocks any overwrite that cuts an existing file by more than 50% (exit 65), with or without `--expect-checksum`
- Pass `--allow-shrink` only when truncation is intentional

```bash
CS=$(atomwrite --workspace . read target.rs | jaq -r '.checksum')
# Blocked without the flag when stdin is much smaller:
echo "tiny" | atomwrite --workspace . write --expect-checksum "$CS" target.rs
# Intentional shrink rewrite:
echo "tiny" | atomwrite --workspace . write --expect-checksum "$CS" --allow-shrink target.rs
```


## How to Use Aggressive Replace Only After --dry-run
- `--fuzzy aggressive` is powerful and can over-match; never mass-apply blind
- Always preview with `--dry-run` first; inspect NDJSON, then apply

```bash
# Preview only
atomwrite --workspace . replace --dry-run --fuzzy aggressive \
  'legacy_api(' 'new_api(' packages/ --include '*.rs'

# Apply only after dry-run envelope looks correct
atomwrite --workspace . replace --fuzzy aggressive --progress-every 50 \
  'legacy_api(' 'new_api(' packages/ --include '*.rs'
```


## How to Install Slim vs Full
- Slim (10,908,040 bytes measured on 2026-10-31, DERIVADO 10908040 / 1048576 = 10.40 MiB, local size-gate ≤15 MiB): core-only, no AST stack
- Default (57,096,064 bytes measured on 2026-10-31, DERIVADO 57096064 / 1048576 = 54.45 MiB): AST languages included
- Both numbers come from `cargo build --release --locked` with and without `--no-default-features --features core`, and the slim binary answers `atomwrite 0.1.37` and exits 78 on `outline`

```bash
# Slim
cargo install --path . --locked --force --no-default-features --features core
# Default with AST
cargo install --path . --locked --force
```


## How to Recover from a Missed Match with best_candidate
- Match failures may emit `best_candidate` (line, similarity, strategy, text, diff_preview)
- Use the near-miss to correct `old` without re-reading the whole file
- `--fuzzy off` = exact-only; the default is `auto`
- A miss in `edit` exits 65 `MATCH_FAILED` with `best_candidate`; a miss in `replace` over a tree exits 1 `NO_MATCHES`, so use `edit` when you want the near-miss

```bash
# Capture the error envelope on a near-miss (exit 65 / MATCH_FAILED)
atomwrite --workspace . scratch init .atomwrite/scratch
atomwrite --workspace . edit src/main.rs \
  --old 'fn main(){let x=1;}' --new 'fn main() { let x = 2; }' \
  | tee .atomwrite/scratch/miss.json \
  | jaq '{exit, best: .best_candidate, ok, match_count}'

# Retry with the candidate text as the new --old
atomwrite --workspace . edit src/main.rs \
  --old "$(jaq -r '.best_candidate.text' < .atomwrite/scratch/miss.json)" \
  --new 'fn main() { // entry point'
```


## How to Inspect rename_method and durability
- Write NDJSON reports `platform.durability`, `platform.rename_method`, `platform.backup_method`
- Linux prefers `renameat2` when available, else `rename`
- Parse platform fields with `jaq` after every durable write

```bash
echo payload | atomwrite --workspace . write --durability full config.toml \
  | jaq '{durability: .platform.durability, rename_method: .platform.rename_method, backup_method: .platform.backup_method, fsync: .platform.fsync}'

# Fast path for ephemeral output (workspace-relative; jail retained)
atomwrite --workspace . scratch init .atomwrite/scratch
echo scratch | atomwrite --workspace . write --durability fast .atomwrite/scratch/ephemeral-out.txt \
  | jaq -r '.platform | "\(.durability) \(.rename_method)"'
```


## How to Read Honest Backup Method
- Backup of the live file is NEVER a hardlink of the same inode
- Inspect `platform.backup_method` (`reflink_or_copy` or `copy`)
- Prefer this honesty for agent pipelines that would otherwise corrupt .bak via shared inode

```bash
# Force a backuped overwrite and read the method used
echo "v2" | atomwrite --workspace . write --keep-backup config.toml \
  | jaq -r '.platform.backup_method // "none"'

# Compare with a plain write (default transactional backup auto-removes on success)
echo "v3" | atomwrite --workspace . write config.toml \
  | jaq '{backup_method: .platform.backup_method, rename_method: .platform.rename_method}'
```


## How to Inventory the Agent Surface
- `agent-surface` emits a clap-derived tool manifesto
- Integration by subprocess, CLI with NDJSON output; wire Bash/argv from this inventory
- Use for host tool registration without hand-maintained flag lists

```bash
atomwrite --workspace . agent-surface
atomwrite --workspace . agent-surface | jaq '{version, integration: .integration, tools: [.tools[].name]}'
```


## How to Stat Without Reading Body
- `stat` is a thin alias of `read --stat`
- Returns metadata and checksum without body bytes
- Prefer for agent loops that only need size, mode, or BLAKE3

```bash
atomwrite --workspace . stat src/lib.rs
atomwrite --workspace . stat src/lib.rs | jaq '{path, bytes, checksum, lines, mode}'
# Equivalent:
atomwrite --workspace . read --stat src/lib.rs
```


## How to Watch with Debounce
- Requires a build with `--features watch` (or `--features full`); the default build exits 78 `CONFIG_INVALID` with the rebuild hint
- Coalesces per-path events with `--debounce-ms`
- Cap lifetime with `--max-events`; optional `--checksum` for BLAKE3 on regular files

```bash
# Build with watch, then stream debounced events
cargo install --path . --locked --force --features watch
atomwrite --workspace . watch . --debounce-ms 200 --max-events 20
atomwrite --workspace . watch src/ --debounce-ms 100 --checksum --max-events 5
```


## How to Semantic-Search Offline (Jaccard)
- Offline token ranking (Jaccard); not OpenRouter embeddings
- Always compiled: no feature gate
- Pass `--index-dir` for inverted-index backend under a local directory

```bash
atomwrite --workspace . semantic-search "atomic rename tempfile" src/ --k 10
atomwrite --workspace . semantic-search "fsync durability" . --k 20 --min-score 0.1 \
  --index-dir .atomwrite/semantic-index
```


## How to Run a Codemod Campaign
- Multi-rule AST campaign over a YAML rules file
- Always dry-run first; the final `codemod_summary` event carries `by_rule_id`
- The format of `rules.yaml` is in `How to Write the rules.yaml for codemod and transform --rules`
- Apply only after the dry-run envelope looks correct

```bash
atomwrite --workspace . codemod --rules rules.yaml --dry-run src/
atomwrite --workspace . codemod --rules rules.yaml --dry-run src/ \
  | jaq -c 'select(.type=="codemod_summary") | .by_rule_id'
# Apply
atomwrite --workspace . codemod --rules rules.yaml src/
```


## How to Observe Batch Progress
- `replace --progress-every N` emits NDJSON heartbeats (`type: progress`)
- Long batch/replace also checks cooperative cancel between units (SIGINT/SIGTERM → cancelled / exit 143)
- Batch emits automatic heartbeats about every max(1, total/20) ops (capped at 50)
- Suppress progress with global `-q` / `-qq` when only the final summary matters

```bash
# Bulk replace with progress every 50 files (default)
atomwrite --workspace . replace --progress-every 50 'old_api' 'new_api' src/ \
  | jaq -c 'select(.type=="progress" or .type=="summary")'

# Batch transaction with built-in heartbeats
printf '%s\n' \
  '{"op":"write","target":"a.txt","content":"one"}' \
  '{"op":"write","target":"b.txt","content":"two"}' \
  | atomwrite --workspace . batch --transaction \
  | jaq -c 'select(.type=="progress" or .type=="summary")'
```


## How to Manage Tuning Keys and Scopes with config
- `config` is the only supported way to change a persistent tuning key: it validates the value, keeps comments and writes by tempfile, fsync and rename
- `config list` and `config get KEY` take no `--scope`: they show `value`, `origin` (`default`, `global`, `workspace`, or `explicit` when `--config PATH` replaces both files) and `default`
- `config set` and `config unset` require `--scope global|workspace` and have no default scope: without it the verb exits 2 and touches nothing
- `global` is the XDG `config.toml`, `workspace` is the `.atomwrite.toml` at the workspace root
- `log.file`, `log.rotation`, `storage.home` and `locale` are global-only (workspace scope exits 65), and an unknown key or an out-of-range value exits 65 without writing
- The terminal event carries `target_resolved` and `target_source: "argv"`: check that the path is the file of the scope you meant
- The recipes below reuse the `WS` variable defined in the first block

```bash
WS="$PWD"   # absolute path of the workspace root, set once
atomwrite --workspace "$WS" config path --scope workspace
atomwrite --workspace "$WS" config list | jaq -r '.keys[] | "\(.key)=\(.value) (\(.origin))"'
atomwrite --workspace "$WS" config get fuzzy.threshold --select key,value,origin,default
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace --select ok,changed,target_resolved
atomwrite --workspace "$WS" config set write.confirm_large_bytes 204800 --scope global
atomwrite --workspace "$WS" config unset fuzzy.threshold --scope workspace --select ok,changed,old_value
atomwrite --workspace "$WS" config unset write.confirm_large_bytes --scope global

# Refused, nothing written
atomwrite --workspace "$WS" config set locale en --scope workspace    # exit 65, global-only key
atomwrite --workspace "$WS" config set fuzzy.threshold 2 --scope workspace  # exit 65, range 0..=1
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9            # exit 2, --scope is required
```


## How to Edit TOML or JSON Keys with set, get and del
- Use `set`, `get` and `del` on a dotted path instead of reading and rewriting the whole file
- `set` infers the type from the value (`9` becomes an integer, `true` a boolean)
- `get` and `del` on a key that does not exist exit 65; `del --force-missing` exits 0 with `existed: false`
- Each of them exits 65 on a file that is not valid TOML or JSON

```bash
printf '[database.pool]\nmax = 5\n' | atomwrite --workspace "$WS" write --no-backup settings.toml
atomwrite --workspace "$WS" set settings.toml database.pool.max 9
atomwrite --workspace "$WS" get settings.toml database.pool.max --select key_path,value,found,format
atomwrite --workspace "$WS" del settings.toml database.pool.max
atomwrite --workspace "$WS" del settings.toml database.pool.max                 # exit 65, key not found
atomwrite --workspace "$WS" del --force-missing settings.toml database.pool.max   # exit 0, existed false
```


## How to Copy and Move Files
- `copy` and `move` refuse an existing target with exit 65; pass `--force` (or `--backup`) to overwrite it
- `copy --recursive --preserve` copies a directory tree and keeps modes and times
- `move` falls back to copy and delete when the target is on another device (EXDEV)

```bash
atomwrite --workspace "$WS" copy src/lib.rs src/lib_copy.rs
atomwrite --workspace "$WS" move src/lib_copy.rs src/lib_moved.rs
atomwrite --workspace "$WS" copy src/lib.rs src/lib_moved.rs           # exit 65, target exists
atomwrite --workspace "$WS" copy --force src/lib.rs src/lib_moved.rs
atomwrite --workspace "$WS" copy --recursive --preserve src snapshot/
```


## How to Count Lines by Extension or Size
- `count --by-extension` returns one map with `files`, `lines`, `blank` and `bytes` per extension
- `count --by-size --top N` lists the N largest files

```bash
atomwrite --workspace "$WS" count --by-extension src/
atomwrite --workspace "$WS" count --by-size --top 3 src/ | jaq -c '.items[] | {path, bytes}'
```


## How to Rename an Identifier with case
- `case` takes files only: a directory argument exits 0 and changes nothing (`identifiers_total: 0`), so list the files
- `--subvert OLD NEW` names the pair and `--to` picks the style (`snake` by default, `camel`, `pascal`, `kebab`, `screaming-snake`)
- Preview with `--dry-run` first: the summary shows `identifiers_total` and `files_modified: 0`
- A file that changed between the read and the write fails with `STATE_DRIFT` (82): run the command again

```bash
atomwrite --workspace "$WS" case --to kebab --subvert setupThing setup-thing --dry-run src/names.rs
atomwrite --workspace "$WS" case --to kebab --subvert setupThing setup-thing src/names.rs src/other.rs
```


## How to See the Resolved Locale
- `locale` reports the language used for messages and suggestions: `resolved`, `source` (`system`, `persisted`) and `available`
- `locale --set en` stores a preference in the XDG config directory and `locale --clear` removes it
- A command can override it once with the global `--locale en`; its `suggestion_lang` field names the language of `suggestion`

```bash
atomwrite locale --select type,resolved,source,available
atomwrite locale --set en
atomwrite locale --clear
```


## How to List AST Nodes and a Code Outline
- `query --kinds` counts every node kind of a file and `query -Q KIND` emits the nodes of one kind with their text
- `outline` lists functions, structs and other items; `--kind` filters (repeatable) and `--positions` adds the byte range
- Both need the grammar of the language: the default build downloads the parser on first use and caches it, and a slim build exits 78

```bash
atomwrite --workspace "$WS" query --kinds src/main.rs --limit 5
atomwrite --workspace "$WS" query -Q function_item src/main.rs --select kind,text --limit 2
atomwrite --workspace "$WS" outline --kind function_item --positions src/main.rs --select name,start_line,end_line
```


## How to Extract a Bounded Signal from a PDF, OOXML or Image
- `extract-artifact` reads only headers and a bounded sample of a PDF, an OOXML package or an image, never the raw bytes: for an image it returns `kind`, `format`, `width`, `height` and `megapixels`
- Limits come from the `extract.*` config keys, and a flag (`--max-text-chars`, `--max-sample-rows`) wins over the key
- A file that is not a PDF, an OOXML package or an image exits 65

```bash
atomwrite --workspace "$WS" extract-artifact img.png --select type,kind,format,width,height
atomwrite --workspace "$WS" extract-artifact img.png --max-text-chars 200
atomwrite --workspace "$WS" extract-artifact src/main.rs        # exit 65, no supported magic bytes
```


## How to Apply N Pairs with edit-loop
- `edit-loop` reads `{"old": ..., "new": ...}` pairs (one per line, or one JSON array) and writes the file once
- The default is all-or-nothing: a pair that does not match exits 65 `MATCH_FAILED` with `failed_pair_index` and nothing is written
- A pair whose `old` text appears more than once is also refused with exit 65: lengthen `old` until it is unique
- `--partial` writes the pairs that match and exits 87 `EDIT_LOOP_PARTIAL` with `pairs_applied` and `pairs_unmatched`
- Read `best_candidate` after a miss when it is present: it shows the closest text in the file

```bash
printf '%s\n' '{"old":"alpha","new":"ALPHA"}' '{"old":"gamma","new":"GAMMA"}' \
  | atomwrite --workspace "$WS" edit-loop src/el.txt --select ok,pairs_total,pairs_applied,checksum_after

# One pair misses: exit 65, nothing written
printf '%s\n' '{"old":"ALPHA","new":"a"}' '{"old":"zzz","new":"y"}' \
  | atomwrite --workspace "$WS" edit-loop src/el.txt --select ok,code,exit,failed_pair_index,best_candidate

# Keep the pairs that match: exit 87
printf '%s\n' '{"old":"ALPHA","new":"a"}' '{"old":"zzz","new":"y"}' \
  | atomwrite --workspace "$WS" edit-loop --partial src/el.txt --select ok,code,exit,pairs_applied,pairs_unmatched
```


## How to Roll Back a Change with backup and rollback
- `backup` writes `FILE.bak.YYYYMMDD_HHMMSS_mmm` next to the file and returns `backup_path`
- `rollback FILE --latest --verify` restores the newest backup and checks its BLAKE3 hash; `--timestamp` picks one by prefix, `--dry-run` only plans
- The shrink guard refuses an overwrite that cuts the file by more than 50% (exit 65) unless you pass `--allow-shrink`
- Mutating verbs remove their own backup on success; `--keep-backup` keeps it, and `delete` keeps its own because the deleted file has no other copy
- `prune-backups` takes the TARGET FILES (not a directory): a directory argument matches nothing

```bash
atomwrite --workspace "$WS" backup src/lib.rs --select type,ok,backup_path
echo "changed" | atomwrite --workspace "$WS" write --no-backup --allow-shrink src/lib.rs
atomwrite --workspace "$WS" rollback src/lib.rs --latest --dry-run
atomwrite --workspace "$WS" rollback src/lib.rs --latest --verify --select ok,type,restored_from,verified
echo "again" | atomwrite --workspace "$WS" write --keep-backup --allow-shrink src/lib.rs
atomwrite --workspace "$WS" prune-backups --max-count 0 --dry-run src/lib.rs   # lists what would go
atomwrite --workspace "$WS" prune-backups --max-count 0 src/lib.rs              # removes every backup of the file
```


## How to Inspect and Heal the Write-Ahead Journals
- `wal-stats` only reads: it reports `total_journals`, `by_state`, `oldest_journal_age_secs`, `total_size_bytes`, `auto_heal_recommended` and `estimated_reclaim_bytes`
- `wal-heal PATH` removes finished journals older than `--threshold-secs` and never touches `started` ones; `PATH` is required and `--max-duration-ms` bounds the sweep
- A run cancelled by SIGTERM, SIGINT or the deadline leaves no journal; a crash (`kill -9`) can leave a `started` one, which `wal-heal` keeps on purpose, so look at it with `wal-stats` before deciding

```bash
atomwrite --workspace "$WS" wal-stats --select total_journals,auto_heal_recommended,estimated_reclaim_bytes
atomwrite --workspace "$WS" wal-heal . --threshold-secs 3600 --max-duration-ms 5000 --select removed,preserved,bytes_reclaimed
atomwrite --workspace "$WS" wal-stats | jaq -e '.total_journals == 0'   # exit 1 while journals remain
```


## How to Gate a Pipeline with doctor --strict
- `doctor` reports `checks[]` with `name`, `status` and `detail`; `doctor --strict` exits 1 (`DOCTOR_CHECKS_FAILED`) when a check does not pass
- Run it first and stop the pipeline on a non-zero exit

```bash
atomwrite doctor --strict --select type,version,ok || exit 1
atomwrite --workspace "$WS" doctor | jaq -r '.checks[] | select(.status != "pass") | "\(.name): \(.detail)"'
```


## How to Discover Output Schemas with --print-schema
- `--print-schema` with no name prints the name of every schema the binary publishes, one JSON string per line
- `--print-schema NAME` prints that JSON Schema and exits 0; an unknown name exits 65
- Each verb serves one schema for its whole stream: `search`, `replace`, `list` and `backup` answer with the union of their records, the terminal summary and the error envelope, and `count-by-ext` and `count-by-size` have names of their own
- `--json-schema SUBCOMMAND` is the older spelling and still works

```bash
atomwrite --print-schema | jaq -r '.' | head -n 5
atomwrite --print-schema search | jaq -r '.title'
atomwrite --print-schema config | jaq -r '.title'
atomwrite --print-schema no-such-schema          # exit 65
```


## How to Branch on the Exit Code and Retry Only When Retryable
- Read the exit code before the output, then read `error_class` and `retryable` from the single terminal event
- Retry only when `retryable` is `true` (exit 28, 30, 73, 74, 82, 83 and 92); `permanent`, `precondition_failed` and `conflict` with `retryable: false` need a different action
- 82 `STATE_DRIFT`: read the file again and rebuild the command; 83 `LOCK_TIMEOUT`: the verb already waited `lock.acquire_timeout_ms` (5 s by default)
- 65 `MATCH_FAILED` or `MATCH_AMBIGUOUS`: read `best_candidate` and `match_count`; 124: raise `--timeout-secs` or narrow the path; 141: the reader closed the pipe, so the output is incomplete
- The verbs already pace their own attempts with the `retry.*` keys, so cap the number of tries and leave the waiting to your scheduler

```bash
attempt=1
while [ "$attempt" -le 3 ]; do
  out=$(atomwrite --workspace "$WS" edit src/main.rs --old "let x = 1;" --new "let x = 2;")
  code=$?
  [ "$code" -eq 0 ] && break
  [ "$(printf '%s' "$out" | jaq -r '.retryable // false')" = "true" ] || break
  attempt=$((attempt + 1))
done
printf '%s\n' "$out" | jaq -c '{ok, code, exit, error_class, retryable, suggestion}'
```


## How to Write the rules.yaml for codemod and transform --rules
- The file is a YAML list; every item has `language`, `pattern` and `rewrite`, and `id` is optional
- A rule with an `id` shows up in `by_rule_id` of the final `codemod_summary` event
- The ast-grep style with `rule:` and `fix:` is not accepted: `codemod` exits 65 `INVALID_INPUT`
- Always run with `--dry-run` first; `--inline-rules` takes the same list as a string

```bash
cat >rules.yaml <<'EOF'
- id: old-to-new
  language: rust
  pattern: old_fn($$$ARGS)
  rewrite: new_fn($$$ARGS)
- language: rust
  pattern: $E.unwrap()
  rewrite: $E?
EOF
atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/ \
  | jaq -c 'select(.type=="codemod_summary") | .by_rule_id'
atomwrite --workspace "$WS" transform --rules rules.yaml --dry-run src/
atomwrite --workspace "$WS" transform --inline-rules '[{"language":"rust","pattern":"old_fn($$$A)","rewrite":"new_fn($$$A)"}]' --dry-run src/
```


## How to Read the Files a Scan Could Not Reach
- `search`, `replace`, `transform` and `scope` report each file they could not read or write as `files_failed` and `errors[]` (first `summary.errors_max_entries` entries (50 by default)) in the summary
- A root named in the invocation that does not exist fails the run even when it matched, with exit 4 `FILE_NOT_FOUND` and `ok: false` in the summary
- A failure found while walking a directory keeps exit 0, and the summary says `partial: true` whenever `files_failed` is present, so one key tells the answer came from part of the targets
- Zero matches plus a failure exits with the code of the first error (for example 13 `PERMISSION_DENIED`)
- All three keys are absent when every file was reached

```bash
atomwrite --workspace "$WS" search TODO src/ \
  | jaq -c 'select(.type=="summary") | {ok, partial, files_failed, errors: [.errors[]? | {path, code}]}'
atomwrite --workspace "$WS" replace --dry-run TODO DONE src/ \
  | jaq -c 'select(.type=="summary") | {files_modified, files_failed}'
```


## How to Read the Single Terminal Event of a Run
- A failing run ends with exactly one event that has `ok: false`, and it carries `code`, `exit`, `error_class`, `retryable` and `suggestion` itself; do not wait for a second error line
- A summary that fails (`search`, `replace`, `transform`, `scope`, `batch`) is that terminal event, so one `jaq` filter on `ok` is enough
- Every verb that writes ends with `target_resolved` (the absolute path it acted on) and `target_source: "argv"`: compare it with the path you meant
- `--filter ok=false` does not hide a failure, because errors are always emitted

```bash
atomwrite --workspace "$WS" --locale en edit src/main.rs --old zzz_missing --new x \
  | jaq -c '{ok, code, exit, error_class, retryable, suggestion}'
atomwrite --workspace "$WS" edit src/main.rs --old "let x = 2;" --new "let x = 3;" --select ok,target_resolved,target_source
```


## How to Shape a Stream Without jq
- `--sort-by path-line` orders events, `--dedupe path` keeps the first event per path, `--aggregate type:count` counts events per type and `--truncate-content lines:60` cuts a string value to 60 characters
- `--max-output-bytes N` cuts every string value, `checksum` included, and needs `--allow-truncate`: each flag alone exits 2
- Use `--select` and `--limit` first and reach for these only when a pipeline needs more

```bash
atomwrite --workspace "$WS" --sort-by path-line --select path,line_number search TODO src/
atomwrite --workspace "$WS" --dedupe path --select path search old src/
atomwrite --workspace "$WS" --aggregate type:count list src/ --depth 1
atomwrite --workspace "$WS" --truncate-content lines:60 --select path,lines search TODO src/
atomwrite --workspace "$WS" --max-output-bytes 12 --allow-truncate --select checksum hash src/lib.rs
atomwrite --workspace "$WS" --max-output-bytes 12 hash src/lib.rs     # exit 2, needs --allow-truncate
```


## What's New in v0.1.12

This section summarizes recipe-relevant changes in v0.1.12. The v0.1.12 release adds new recipes to the cookbook for the 6 new subcommands, the new flags, and the crash recovery workflow.

### New Recipes (Added in v0.1.12)
- How to Edit a Config File Without Rewriting It -- use `set`/`get`/`del` instead of `read`+`edit` for dotted paths in TOML/JSON
- How to Rename an Identifier Across a Module -- use `case --subvert OLD NEW --to <style>` for 5 case styles
- How to Walk a Code AST -- use `query --kinds` to list kinds, `query --query KIND` to filter
- How to Extract a Code Map -- use `outline --positions` for functions/structs/enums/traits
- How to Detect Syntax Errors Before Commit -- use `write --syntax-check` for tree-sitter validation
- How to Recover from a Crashed Write -- use `wal-stats` and `wal-heal` to inspect WAL sidecars
- How to Compose read with Unix Tools -- use `read --format raw` for Unix composability
- How to Limit Search to Small Files -- use `search --max-filesize` and `--max-columns`
- How to Replace Literal Strings -- use `replace --literal` to disable regex
- How to Apply Multi-Rule Refactor -- use `transform --rules <file.yaml>` for cascading rules
- How to Acquire Advisory File Lock -- nothing to pass: since 0.1.37 every atomic write takes the lock itself, waits up to 5 s and exits 83 `LOCK_TIMEOUT`
- How to Backup with CoW -- use `backup` and `copy` with default reflink on APFS/btrfs/XFS

### Updated Recipes (v0.1.12 changes reflected in existing recipes)
- How to Write a File Atomically -- now mentions `--syntax-check` and the automatic lock (`search --include-fifo` opts back into opening FIFOs, and `write` to a FIFO exits 85)
- How to Edit a File -- now mentions `--after-line`, `--before-line`, `--range`, `--delete-range`, `--between`
- How to Read a File -- now mentions `--format raw`, `--head N`, `--tail N`, `--line N`, `--grep`
- How to Search -- now mentions `--max-filesize`, `--max-columns`
- How to Replace Text in Bulk -- now mentions `--literal`, `--fuzzy`
- How to Transform Code -- now mentions `--rules`, `--inline-rules`
- How to Batch Operations -- now mentions `--batch-size`, `--file`
- How to Backup -- now mentions `--no-reflink`, `--output-dir`

### New Subcommands Available
- `set` -- write a value at a dotted path in TOML/JSON
- `get` -- read a value at a dotted path
- `del` -- remove a key at a dotted path
- `case` -- rename identifiers in 5 case styles via `heck`
- `query` -- walk a tree-sitter AST
- `outline` -- extract top-level structure

### New Flags for Existing Commands
- `read --format raw` (G81)
- `write --syntax-check` (G72)
- Advisory write lock (G54), automatic since 0.1.37, no flag
- `search --max-filesize`, `--max-columns` (G68)
- `replace --literal` (G66)
- `transform --rules`, `--inline-rules` (G44)
- `batch --batch-size` (G77)
- `backup/copy --no-reflink` (G64)

### Test Coverage (Historical, v0.1.28)
- Historical (v0.1.28): 661+ tests passing
- 9 ADRs in `docs/decisions/` (0019-0027)
- 7 new JSON schemas in `docs/schemas/`
- See [docs/decisions/README.md](decisions/README.md) for architectural decisions


## Latency Note
- All operations execute locally with sub-millisecond overhead
- The atomic write sequence adds ~1ms for the fsync-rename-fsync cycle
- Search parallelism scales with available CPU cores
- Batch mode amortizes startup cost across N operations


## Default Values Reference
- `--threads` omitted or `0` uses all logical CPUs up to a built-in ceiling of 16, capped by RAM; an explicit N is honored even above 16
- `--max-filesize` defaults to 1 GiB (1,073,741,824 bytes); `search` overrides it to 10 MiB and counts the skipped files in `files_skipped`
- `--color` defaults to `auto` (detect terminal)
- `--workspace` defaults to current working directory
- `diff --context` defaults to 3 lines
- `diff --algorithm` defaults to `patience`
- Global `--timeout-secs` / `--timeout` → 120 (`0` disables; deadline exit 124)
- Fuzzy max applies when `--max-replacements` omitted → 1 (hard ceiling 10_000); multi-apply is one-pass on original content; embeds force 1


## How to Write a File Atomically
- Pipe content from stdin to create or overwrite a file
- The write survives power failure and process crashes

```bash
echo "fn main() { println!(\"hello\"); }" | atomwrite write src/main.rs
```

- Create a backup before overwriting:

```bash
cat updated_config.toml | atomwrite write --backup config.toml
```

- Write with workspace restriction:

```bash
echo "data" | atomwrite write --workspace /home/user/project src/data.txt
```


## How to Normalize Line Endings
- Force LF line endings when writing:

```bash
echo "line1\r\nline2\r\n" | atomwrite write --line-ending lf src/file.txt
```

- Force CRLF for Windows compatibility:

```bash
cat unix_file.txt | atomwrite write --line-ending crlf src/windows_file.txt
```

- Preserve original line endings (default):

```bash
cat source.txt | atomwrite write --line-ending auto src/output.txt
```

- Normalize line endings during edit:

```bash
atomwrite edit --line-ending lf src/mixed.rs --old "old_text" --new "new_text"
```


## How to Search Across a Project
- Search for a pattern across all files in a directory:

```bash
atomwrite search 'TODO' src/
```

- Search with regex and context lines:

```bash
atomwrite search --regex 'fn\s+test_\w+' --context 2 src/
```

- Get just file paths with matches:

```bash
atomwrite search --files 'deprecated' src/
```

- Get match counts per file:

```bash
atomwrite search --count 'unwrap()' src/
```

- Combine with extract to get specific fields:

```bash
atomwrite search 'TODO' src/ | atomwrite extract path line_number lines
```


## How to Replace Text in Bulk
- Replace a string across all files in a directory:

```bash
atomwrite replace 'old_function' 'new_function' src/
```

- Preview replacements without modifying files:

```bash
atomwrite replace --dry-run 'before' 'after' src/
```

- Replace with regex:

```bash
atomwrite replace --regex 'v\d+\.\d+\.\d+' 'v2.0.0' src/
```

- Replace with workspace restriction:

```bash
atomwrite replace --workspace /home/user/project 'old' 'new' src/
```


## How to Scope Code by Grammar Category
- Delete all comments from a Rust file:

```bash
atomwrite scope --query comments --delete -l rust src/main.rs
```

- Uppercase all function names in Python:

```bash
atomwrite scope --query def --action upper -l python src/app.py
```

- Squeeze whitespace in strings:

```bash
atomwrite scope --query strings --action squeeze -l rust src/lib.rs
```

- Every `scope` call needs `--language` (`-l`, alias `--lang`); without it the verb exits 2

- Replace comments with a standard header:

```bash
atomwrite scope --query comments --replace-with "// TODO: review" -l rust src/main.rs
```

- Use custom AST pattern for titlecase:

```bash
atomwrite scope --pattern 'fn $NAME($$$ARGS)' --action titlecase -l rust src/
```


## How to Create and Restore Backups
- Create a timestamped backup with BLAKE3 checksum:

```bash
atomwrite backup src/main.rs src/lib.rs
```

- Preview backup creation without writing:

```bash
atomwrite backup --dry-run src/main.rs
```

- Keep up to 30 backup copies:

```bash
atomwrite backup --retention 30 src/config.toml
```

- Restore the most recent backup:

```bash
atomwrite rollback src/main.rs --latest
```

- Restore a specific timestamped backup:

```bash
atomwrite rollback src/main.rs --timestamp 20260529_120000
```

- Verify checksum before restoring:

```bash
atomwrite rollback --verify src/main.rs --latest
```

- Preview restore without applying:

```bash
atomwrite rollback --dry-run src/main.rs --latest
```


## How to Apply Patches From Stdin
- Apply a unified diff patch:

```bash
cat fix.patch | atomwrite apply src/main.rs
```

- Apply a markdown-fenced patch:

```bash
cat changes.md | atomwrite apply --format markdown src/main.rs
```

- Apply SEARCH/REPLACE blocks from an agent:

```bash
cat agent_output.txt | atomwrite apply --format search-replace src/main.rs
```

- Apply with automatic backup before patching:

```bash
cat fix.patch | atomwrite apply --backup src/main.rs
```

- Preview patch application without modifying:

```bash
cat fix.patch | atomwrite apply --dry-run src/main.rs
```

- Apply a full file replacement:

```bash
cat new_version.rs | atomwrite apply --format full src/main.rs
```


## How to Refactor With AST Patterns
- Rename a function across a Rust codebase:

```bash
atomwrite transform --pattern 'old_fn($$$ARGS)' --rewrite 'new_fn($$$ARGS)' -l rust src/
```

- Migrate from println to tracing:

```bash
atomwrite transform --pattern 'println!($$$ARGS)' --rewrite 'tracing::info!($$$ARGS)' -l rust src/
```

- Replace all unwrap calls with the `?` operator:

```bash
atomwrite transform --pattern '$EXPR.unwrap()' --rewrite '$EXPR?' -l rust src/
```

- Migrate JavaScript console.log:

```bash
atomwrite transform --pattern 'console.log($$$ARGS)' --rewrite 'logger.info($$$ARGS)' -l js src/
```

- Preview AST transform without applying:

```bash
atomwrite transform --dry-run --pattern 'old_api($$$ARGS)' --rewrite 'new_api($$$ARGS)' -l python src/
```


## How to Generate Regex From Examples
- Generate a date pattern regex:

```bash
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"
```

- Generate with digit and word generalization:

```bash
atomwrite regex --digits --words "user_123" "admin_456" "guest_789"
```

- Use the generated regex in a search:

```bash
PATTERN=$(atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" | jaq -r '.regex')
atomwrite search --regex "$PATTERN" src/
```


## How to Calculate Unit Conversions
- Convert time units:

```bash
atomwrite calc "2 hours + 30 minutes to seconds"
```

- Convert data sizes:

```bash
atomwrite calc "10 GiB to MB"
```

- Evaluate math expressions:

```bash
atomwrite calc "sqrt(144) + 3^2"
```

- Calculate percentages:

```bash
atomwrite calc "15% of 200"
```


## How to Batch Multiple Operations
- Batch supports 7 operations: write, replace, delete, edit, hash, move, copy
- Create an NDJSON manifest with multiple operations:

```bash
cat <<'EOF' > manifest.ndjson
{"op":"write","path":"src/a.txt","content":"hello"}
{"op":"write","path":"src/b.txt","content":"world"}
{"op":"delete","path":"src/old.txt"}
{"op":"edit","path":"src/a.txt","old":"hello","new":"hello world"}
{"op":"hash","path":"src/b.txt"}
{"op":"move","source":"src/a.txt","target":"src/renamed.txt"}
{"op":"copy","source":"src/b.txt","target":"src/b_copy.txt"}
EOF
cat manifest.ndjson | atomwrite batch
```

- Preview the batch without executing (the preview checks each operation against the disk as it is now, so an operation that depends on an earlier one of the same manifest, such as an edit of a file the manifest creates, is reported as failed and the run exits 65):

```bash
cat manifest.ndjson | atomwrite batch --dry-run
```

- Execute as all-or-nothing transaction with automatic rollback on failure:

```bash
cat manifest.ndjson | atomwrite batch --transaction
```

- Generate a manifest from search results:

```bash
atomwrite search --files 'deprecated' src/ | \
  jaq -r 'select(.type=="file") | .path' | \
  while read -r p; do echo "{\"op\":\"delete\",\"path\":\"$p\"}"; done | \
  atomwrite batch --dry-run
```


## How to Verify File Integrity
- Hash a file and store the checksum:

```bash
atomwrite hash src/main.rs
```

- Verify a file against a known checksum:

```bash
atomwrite hash --verify abc123def456 src/main.rs
```

- Hash from stdin:

```bash
echo "data" | atomwrite hash --stdin
```

- Compare two files for differences:

```bash
atomwrite diff --stat src/old.rs src/new.rs
```


## How to Use Optimistic Locking
- Since v0.1.15 the target is resolved against `--workspace` before verification (G118); on v0.1.14 and earlier, run with CWD = workspace or a relative target silently skips the checksum check
- Read a file and capture the checksum (`extract checksum` prints a JSON object, so use `jaq -r` for the bare value):

```bash
CHECKSUM=$(atomwrite read --stat src/config.toml | jaq -r '.checksum')
```

- Write with the expected checksum:

```bash
echo "updated content" | atomwrite write --expect-checksum "$CHECKSUM" src/config.toml
```

- Handle state drift (exit code 82):

```bash
echo "updated content" | atomwrite write --expect-checksum "$CHECKSUM" src/config.toml
if [ $? -eq 82 ]; then
  echo "File changed by another process, re-reading..."
  CHECKSUM=$(atomwrite read --stat src/config.toml | jaq -r '.checksum')
  echo "updated content" | atomwrite write --expect-checksum "$CHECKSUM" src/config.toml
fi
```


## How to Edit and Trigger a Build Without Manual Touch
- Edit a source file in a Rust project and trigger cargo without manually running `touch`:

```bash
atomwrite edit src/main.rs --old "old_text" --new "new_text"
cargo build
```

- This works because `edit` updates the mtime by default, so cargo sees the source as newer than its dep-info file and recompiles.
- If you opt out of mtime updates with `--preserve-timestamps`, cargo may skip the rebuild silently (the famous `Finished in 0.29s` no-op):

```bash
atomwrite edit --preserve-timestamps src/main.rs --old "old_text" --new "new_text"
cargo build  # may be a silent no-op, forcing you to touch the file manually
```

- Check whether mtime was preserved by reading the `mtime_preserved` field in the NDJSON response:

```bash
atomwrite edit src/main.rs --old "old" --new "new" | atomwrite extract mtime_preserved
```

- Use `--preserve-timestamps` only for backup, snapshot, or reproducible-build scenarios. For interactive development, leave the default in place so build systems detect your changes.


## How to Create Backups With Retention
- Write a file with automatic backup:

```bash
echo "new content" | atomwrite write --backup src/config.toml
```

- Delete a file with backup:

```bash
atomwrite delete --backup src/old_module.rs
```

- Keep up to 30 backups of the deleted file:

```bash
atomwrite delete --backup --retention 30 src/old_module.rs
```

- List backup files in a directory:

```bash
atomwrite list --long --all --include '*.bak.*' src/
```


## How to Extract Fields From NDJSON Pipeline
- Use extract to pull specific fields from atomwrite output
- Use field names for JSON keys or positional indices for text columns

```bash
atomwrite search 'TODO' src/ | atomwrite extract path line_number lines
```

- Extract just paths from search results:

```bash
atomwrite search --files 'error' src/ | atomwrite extract path
```

- Extract checksums from write results:

```bash
echo "data" | atomwrite write src/file.txt | atomwrite extract checksum
```

- Extract text columns by index:

```bash
echo "a b c d" | atomwrite extract 0 2
```


## How to List Project Structure
- List files with NDJSON output:

```bash
atomwrite list src/
```

- Long format with size, permissions and modification time:

```bash
atomwrite list --long src/
```

- Count files grouped by extension:

```bash
atomwrite list --count-by-ext src/
```

- Combine with extract for custom views:

```bash
atomwrite list --long src/ | atomwrite extract path bytes
```


## Scope Operations
### Delete All Comments from Rust Files

```bash
atomwrite --workspace . scope src/ --lang rust --query comments --delete
```

### Uppercase All Function Names (Preview)

```bash
atomwrite --workspace . scope src/ --lang rust --query fn --action upper --dry-run
```

### Remove Comments from Python Scripts

```bash
atomwrite --workspace . scope scripts/ --lang python --query comments --delete
```


## Backup and Rollback
### Create Backup Before Risky Edit

```bash
atomwrite --workspace . backup src/config.rs
echo "new config" | atomwrite --workspace . write src/config.rs
```

### Restore from Latest Backup

```bash
atomwrite --workspace . rollback src/config.rs
```

### Restore from Specific Timestamp with Verification

```bash
atomwrite --workspace . rollback src/config.rs --timestamp 20260530_120000 --verify
```


## Apply Patches
### Apply Full File Replacement

```bash
echo "new content" | atomwrite --workspace . apply src/file.txt --format full
```

### Apply Unified Diff from Git

```bash
git diff src/file.rs | atomwrite --workspace . apply src/file.rs
```

### Apply SEARCH/REPLACE Blocks

```bash
cat <<'EOF' | atomwrite --workspace . apply src/main.rs
<<<<<<< SEARCH
old_function_name
=======
new_function_name
>>>>>>> REPLACE
EOF
```


## Agent-First Patterns (v0.1.2+)

### Bound a Long Search with Timeout

```bash
# Global default is 120s (exit 124 on deadline); 0 disables
# Override to 60s if search doesn't finish; emits NDJSON error with error_class=transient
atomwrite --workspace . --timeout-secs 60 search 'TODO' src/
```

### Read Only Lines Matching a Regex

```bash
# Useful for extracting logs from huge files without exhausting context
atomwrite --workspace . read --grep 'ERROR|WARN' logs/app.log
```

### Read First N Lines of a Huge File

```bash
# Avoids loading the entire file into context
atomwrite --workspace . read --head 20 huge.log
```

### Batch from File Instead of stdin

```bash
# Persisted manifest file (NDJSON, one op per line)
atomwrite --workspace . batch --file ops.ndjson
```

### Backup to a Centralized Directory

```bash
# Keep source directory clean; centralize backups inside the workspace
atomwrite --workspace . backup --output-dir backups src/critical.rs
```

### Install Shell Completions on First Use

```bash
# Auto-installs to ~/.local/share/bash-completion/completions/atomwrite
atomwrite completions bash --install
```

### Always pass `--workspace` for agents

```bash
# Agents must pass --workspace explicitly (no product environment variables); use an absolute path
atomwrite --workspace /home/user/project read src/main.rs
```


## Agent-First Patterns (v0.1.3+)

### Edit and Trigger Cargo Build Without Manual Touch

```bash
# New default: edit updates the mtime, so cargo rebuilds automatically
atomwrite edit src/main.rs --old "old_text" --new "new_text"
cargo build  # rebuilds without needing `touch` first
```

### Read mtime_preserved From Edit Response

```bash
# Parse the NDJSON response to verify whether the timestamp was kept
atomwrite edit src/main.rs --old "old" --new "new" | atomwrite extract mtime_preserved
```

### Preserve Original mtime for Backup or Snapshot Workflows

```bash
# Opt back into the v0.1.2 behavior of preserving the original file mtime
atomwrite edit --preserve-timestamps src/snapshot.rs --old "old" --new "new"
atomwrite replace --preserve-timestamps 'old_api' 'new_api' src/
```


## How to Interpret Error Suggestions (v0.1.4)
- Every error envelope includes a `suggestion` field with actionable recovery guidance
- The `WorkspaceJail` suggestion adapts based on whether `--workspace` was provided
- Use the suggestion to drive agent retry logic instead of parsing the message text

```bash
# When workspace is NOT provided, the suggestion prompts for the flag
atomwrite --locale en read /etc/passwd 2>/dev/null
# Output: {"error":true,"code":"WORKSPACE_JAIL","exit":126,...,"suggestion":"set --workspace <root> (XDG/config only; no environment variables)",...}

# When workspace IS provided, the suggestion says "use a path inside"
atomwrite --locale en --workspace /home/user/project read /etc/passwd 2>/dev/null
# Output: {"error":true,"code":"WORKSPACE_JAIL","exit":126,...,"suggestion":"use a path inside the workspace (/home/user/project); for --old-file/--new-file stage under /home/user/project/.atomwrite/scratch/ (never OS /tmp outside the jail)",...}
```


## How to Install on Windows 10/11 (v0.1.4)
- v0.1.4 finally fixes `cargo install atomwrite` on Windows
- Install Visual Studio 2019+ Build Tools with the C++ workload
- Install Rust 1.90+ via rustup
- Run `cargo install atomwrite --locked`
- See [INSTALL.md](INSTALL.md) for the full Windows troubleshooting guide

```powershell
# PowerShell 7+ or Windows Terminal
rustup default stable
rustup target add x86_64-pc-windows-msvc
cargo install atomwrite --locked
atomwrite --version  # plain text, not NDJSON
```


## How to Discover the JSON Schema at Runtime
- Use `--print-schema NAME` to emit the JSON Schema for an output, or `--print-schema` alone to list the 55 names; `--json-schema SUBCOMMAND` is the older spelling and still works
- No need to read a static schema file; the schema is part of the binary

```bash
# Get the schema for the read subcommand output
atomwrite --json-schema read
atomwrite --json-schema write
atomwrite --json-schema edit
atomwrite --json-schema search
atomwrite --json-schema replace
atomwrite --json-schema batch
atomwrite --json-schema error  # shared by all subcommands
```

- Check live output against the keys the schema requires:

```bash
# 1. Capture the schema (any name from `atomwrite --print-schema`)
atomwrite --print-schema error > error.schema.json

# 2. Run the actual command and check the required keys of the error envelope
atomwrite --workspace . read /missing 2>/dev/null \
  | jaq -e 'has("error") and has("ok") and has("code") and has("exit") and has("error_class") and has("retryable")'
```


## How to Read NDJSON in a Shell Pipeline with jaq
- Subcommand output is NDJSON unless a flag asks otherwise, and `read --format raw`, `completions` and `--version` are the exceptions
- Pair with `jaq` for structured filtering, mapping, and aggregation

```bash
# Extract just the checksum from a read response
atomwrite read src/main.rs | jaq -r '.checksum'

# Count search matches per file
atomwrite search 'TODO' src/ | jaq -r 'select(.type=="match") | .path' | sort | uniq -c | sort -rn

# Read the batch totals
atomwrite batch < manifest.ndjson | jaq -c 'select(.type=="summary") | {operations, succeeded, failed}'

# Filter error envelopes by error class
atomwrite read /missing 2>/dev/null | jaq 'select(.error_class == "permanent")'
```


## How to Handle Persistent Errors with Retry Logic
- Retry only when the terminal event says `retryable: true` (exit 28, 30, 73, 74, 82, 83 and 92), and cap the number of tries
- The verbs already pace their own attempts with the `retry.*` and `lock.acquire_timeout_ms` keys, so the loop needs no delay of its own
- The full loop, with the exit codes that need a different action, is in `How to Branch on the Exit Code and Retry Only When Retryable`

```bash
attempt=1
while [ "$attempt" -le 5 ]; do
  output=$(atomwrite --workspace . "$@" 2>/dev/null)
  exit_code=$?
  [ "$exit_code" -eq 0 ] && break
  [ "$(printf '%s' "$output" | jaq -r '.retryable // false')" = "true" ] || break
  attempt=$((attempt + 1))
done
printf '%s\n' "$output"
```


## WAL Journal Management (v0.1.15+)

The G119 effort introduces three new layers for managing sidecar WAL journals. These recipes show practical usage of the new subcommands and flags.

### How to Reap Stale Journals Before Commit

The `wal-heal` subcommand (G119 L3) safely removes terminal journals (Committed and Aborted). Wire it into a pre-commit hook to keep the working tree clean.

```bash
# pre-commit hook body
atomwrite --workspace . wal-heal . --threshold-secs 0 \
  | jaq -e '.removed >= 0' \
  || { echo "wal-heal failed"; exit 1; }
```

The `--threshold-secs 0` flag removes terminal journals of any age. `wal-heal` NEVER touches `Started` entries. For a more conservative sweep, raise the threshold to retain recent journals for forensic analysis.

### How to Configure WAL Policy per Workload

The `--wal-policy` flag (G119 L1) on `write` and `edit` controls when the sidecar journal is written. Three values are accepted:

- `auto` (default) -- the policy chosen by the build, optimized for general use
- `always` -- always write the sidecar journal (forensic-grade audit trail)
- `never` -- never write the sidecar journal (fastest path, no recovery metadata)

```bash
# ephemeral builds: skip journal overhead, sidecars have no consumer there
atomwrite --workspace . write --wal-policy never ephemeral-config.toml < config.toml

# Production deploys: audit trail matters, force the sidecar
atomwrite --workspace . write --wal-policy always config/prod.toml < prod.toml

# General agent use: let the default decide
atomwrite --workspace . write src/lib.rs < new_lib.rs
```

### How to Inspect Journal Health

The `wal-stats` subcommand (G119 L5) emits read-only local diagnostics about the current WAL state. Pair it with `jaq` to gate local or post-build scripts.

```bash
# Full local WAL diagnostics as NDJSON
atomwrite --workspace . wal-stats

# Gate on zero journals
atomwrite --workspace . wal-stats | jaq -e '.total_journals == 0' || { echo "drift"; exit 1; }

# Extract just the reclaimable size for diagnostics
atomwrite --workspace . wal-stats | jaq -r '.estimated_reclaim_bytes'
```


## How to Batch Multiple Edit Pairs Safely (v0.1.15, G117)
- Repeated `--old`/`--new` pairs run the fuzzy cascade per pair
- The default is all-or-nothing: a failed pair aborts the batch with exit 65, no write, and `failed_pair_index` in the error envelope
- `--partial` applies the matching pairs and reports the rest with `matched: false`

```bash
# All-or-nothing with per-pair ground truth
atomwrite --workspace . edit src/main.rs --old "foo" --new "bar" --old "baz" --new "qux" \
  | jaq -e '.pair_results'

# Partial application: keep the valid work, list the misses
atomwrite --workspace . edit --partial src/main.rs --old "foo" --new "bar" --old "maybe" --new "x" \
  | jaq -e '{edits, pairs_total, missing: [.pair_results[] | select(.matched | not) | .index]}'

# Anti-masking: a bare pipe hides exit 65 as {"edits": null} with pipeline exit 0
atomwrite --workspace . edit src/main.rs --old "missing" --new "x" | jaq -e '.edits' \
  || echo "edit failed: ${PIPESTATUS[0]}" >&2
```


## v0.1.20 — What Is New

This release introduces a new safety layer called intention guards and renames the global `--lang` flag to `--locale` to disambiguate from the tree-sitter `--lang` selector used by `scope` and `transform`.

### Intention Guards (5 OPT-IN flags)
- `--require-backup` — takes no value: abort when `--backup` is missing and the target file exists
- `--confirm` — legacy flag with no effect by itself; large overwrites are guarded by `--ack-overwrite`
- `--auto-rotate` — takes no value: force an auto-rotation backup for recently modified files
- `--risk-threshold <PERCENT>` — size-delta percentage that raises a risk warning (default off, `255`)
- `--locale <en|pt-BR>` — renamed from `--lang` to disambiguate from the tree-sitter `--lang`

### Other Additions
- `count --by-size` — list the largest files in the tree with sizes and line counts
- `read --format raw|ndjson` — select between byte-stream output and structured NDJSON envelope
- `search --no-begin-end` — disable the implicit `^` and `$` anchor decoration in regex output
- `write --preserve-timestamps` — keep the source file mtime when overwriting
- `scope --lang rust` — alias of `scope --language rust` (`transform` accepts only `-l` or `--language`)

### Statistics (Historical, v0.1.20)
- Historical (v0.1.20): 542 tests passing in 47 integration suites, 0 failures
- 11 GAP-2026 closed
- 3 Windows cross-compile targets green
- 19 ADRs in `docs/decisions/` (0019-0037)

### Migration `--lang` to `--locale`

```bash
# Discover all files using --lang
rg -l -- '--lang\b' .

# Bulk replace while preserving other matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Or via ruplacer (preview first, add --go to write)
ruplacer -- '--lang' '--locale' .
```


## v0.1.21 — Recipes

### Backup Deleted After Success
- Default change — `write --backup` no longer leaves a `.bak.<timestamp>` sibling on disk after success. The backup is deleted.
- Opt-in to preserve — pass `--keep-backup` to any of `write`, `edit`, `replace`, `rollback`, `apply`, `batch`:

```bash
# Default v0.1.21: backup is created, write succeeds, backup is deleted
echo "new" | atomwrite --workspace . write --backup config.toml

# Opt-in v0.1.21: backup is created, write succeeds, backup is preserved
echo "new" | atomwrite --workspace . write --backup --keep-backup config.toml
```

### Sequential Edit Pattern with Checksum Re-capture
- Pattern A — explicit re-capture is the canonical way to chain N `edit` calls on the same file:

```bash
set -euo pipefail
for pair in "foo:bar" "baz:qux" "alpha:beta"; do
  OLD="${pair%:*}"
  NEW="${pair#*:}"
  CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
  atomwrite --workspace . edit --old "$OLD" --new "$NEW" --expect-checksum "$CS" src/foo.rs
done
```

- Pattern B — `--allow-sequential-drift` opt-in for one-shot scripts that prefer not to re-capture:

```bash
CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
for pair in "foo:bar" "baz:qux"; do
  OLD="${pair%:*}"; NEW="${pair#*:}"
  atomwrite --workspace . edit --allow-sequential-drift --old "$OLD" --new "$NEW" --expect-checksum "$CS" src/foo.rs
done
```


## v0.1.22 — Recipes

### `edit-loop` for N Pairs in 1 Invocation
- Use case: apply a batch of textual transformations to one file (rename an identifier in 7 places, sweep obsolete type aliases) where today you invoke `edit` N times in a shell loop.

```bash
# Apply 3 pairs in 1 invocation
printf '%s\n' \
  '{"old":"v0_1_20","new":"v0_1_22"}' \
  '{"old":"foo","new":"bar"}' \
  '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace . edit-loop src/version.rs

# With backup preserved for forensic timeline
printf '%s\n' '{"old":"foo","new":"bar"}' \
  | atomwrite --workspace . edit-loop --backup --keep-backup src/foo.rs

# With --partial (best-effort: apply matched, report unmatched, exit 87)
printf '%s\n' '{"old":"exists","new":"X"}' '{"old":"absent","new":"Y"}' \
  | atomwrite --workspace . edit-loop --partial src/foo.rs

# Without --partial a pair that does not match exits 65 MATCH_FAILED and nothing is written
printf '%s\n' '{"old":"exists","new":"X"}' '{"old":"absent","new":"Y"}' \
  | atomwrite --workspace . edit-loop src/foo.rs
```

### `prune-backups` for Manual Cleanup of Legacy `.bak.*`
- Use case: operators who upgraded from v0.1.20 inherit `.bak.<timestamp>` siblings that v0.1.21 no longer creates (and therefore no longer cleans up automatically).
- Targets: pass the FILES whose backups you want pruned; a directory argument matches nothing and still exits 0.

```bash
# --dry-run is a switch without a value: list what WOULD be removed
atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run src/foo.rs

# Remove backups of the file that are older than 24 hours (omit --dry-run to apply)
atomwrite --workspace . prune-backups --max-age-secs 86400 src/foo.rs

# Keep only the 3 most recent backups of the file
atomwrite --workspace . prune-backups --max-count 3 src/foo.rs

# Local gate: assert zero backups of the file after cleanup
atomwrite --workspace . prune-backups --max-age-secs 0 src/foo.rs \
  && fd -g 'foo.rs.bak.*' src | wc -l | jaq -e '. == 0'
```


## v0.1.25 — Recipes

### Verify File Checksum

```bash
# Dedicated verify subcommand: the expected hash is the second positional argument
atomwrite --workspace . verify src/main.rs abc123def456
# Exit 0 on match, exit 81 on mismatch
```

### Delete Old Files by Age

```bash
# Delete files older than 7 days (duration suffixes: s/m/h/d/w); a directory needs --recursive
atomwrite --workspace . delete --recursive --older-than 7d --dry-run logs/
atomwrite --workspace . delete --recursive --older-than 7d logs/

# Preview deletion plan with --plan
atomwrite --workspace . delete --recursive --plan --older-than 1h tmp/
```

### Replace with Case Preservation

```bash
# Automatically adapt replacement case: Hello→Goodbye, HELLO→GOODBYE, hello→goodbye
atomwrite --workspace . replace --preserve-case 'hello' 'goodbye' src/
```

### Configure Fuzzy Threshold

```bash
# Strict: only accept very close matches (0.99)
atomwrite --workspace . edit src/main.rs --old "approximate_text" --new "new_text" --fuzzy-threshold 0.99

# Loose: accept distant matches (0.5)
atomwrite --workspace . edit src/main.rs --old "approximate_text" --new "new_text" --fuzzy-threshold 0.5 --fuzzy aggressive
```

### Scope Symbols and Normalize

```bash
# Convert ASCII operators in strings to Unicode symbols (=> to ⇒, != to ≠); --query or --pattern is required
atomwrite --workspace . scope src/ --lang rust --query strings --action symbols --dry-run

# Apply NFC Unicode normalization (exit 1 when the text is already normalized)
atomwrite --workspace . scope src/ --lang rust --query strings --action normalize --dry-run
```

### Use .atomwrite.toml Config

```bash
# Create a project config (optional, CLI flags take precedence)
cat > .atomwrite.toml << 'EOF'
[defaults]
backup = true

[fuzzy]
mode = "auto"
threshold = 0.85
EOF

# atomwrite auto-loads .atomwrite.toml from project root
atomwrite --workspace . edit src/main.rs --old "foo" --new "bar"

# Or let the config verb write the same keys for you (comments are kept)
atomwrite --workspace . config set fuzzy.threshold 0.85 --scope workspace
```

### Hash Output Uses checksum Field

```bash
# v0.1.25: field is "checksum" (was "value" in v0.1.24)
atomwrite --workspace . hash src/main.rs | jaq -r '.checksum'
```


## v0.1.24 — Recipes

### Typed Error Handling in Agent Pipelines

All errors now emit structured JSON on stdout. Parse exit codes deterministically:

```bash
# Safe pipeline: check exit code BEFORE parsing stdout
output=$(atomwrite --workspace . get config.toml database.pool.max 2>/dev/null)
exit_code=$?
case $exit_code in
  0) echo "$output" | jaq -r '.value' ;;
  4) echo "file not found" ;;
  65) echo "$output" | jaq -r '.message, .suggestion' ;;   # includes a missing key
  *) echo "unexpected: exit $exit_code" ;;
esac
```

### Delete Recursive (Now Works)

```bash
# v0.1.24: delete --recursive actually traverses and removes
atomwrite --workspace . delete --recursive logs/

# Dry-run first to preview
atomwrite --workspace . delete --recursive --dry-run logs/
```

### Hash Recursive (Now Works)

```bash
# v0.1.24: hash --recursive walks directories
atomwrite --workspace . hash --recursive src/
```

### Search Multiline (Now Works)

```bash
# v0.1.24: multiline patterns actually match across lines
atomwrite --workspace . search --multiline 'fn main\(\).*\{' src/ --include '*.rs'
```

### Replace Rejects Empty Pattern

```bash
# v0.1.24: empty pattern is rejected (was silently destructive)
atomwrite --workspace . replace '' 'X' src/
# Exit 65: INVALID_INPUT — empty pattern would match every position

# Correct usage: explicit pattern
atomwrite --workspace . replace 'old_api' 'new_api' src/
```

### Backup Timestamp With Milliseconds

```bash
# v0.1.24: timestamps include milliseconds to prevent collision
atomwrite --workspace . backup config.toml
# Creates: config.toml.bak.20260621_143022_847

# Rollback accepts prefix match (backward-compatible)
atomwrite --workspace . rollback config.toml --timestamp 20260621_143022
```

### Get Values Without Double Quotes

```bash
# v0.1.24: get returns raw value, not JSON-serialized string
atomwrite --workspace . get Cargo.toml package.version
# Output: {"type":"get","value":"0.1.24",...}
# NOT: {"type":"get","value":"\"0.1.24\"",...}  (old behavior)
```


## How to Overwrite a Large File Safely (v0.1.35)
- Problem: existing target above XDG `[write].confirm_large_bytes` (default 100 KiB)
- Solution: pass `--ack-overwrite` (one-shot; no Y/N)

```bash
printf 'payload' | atomwrite --workspace . write --ack-overwrite big.txt
```


## How to Preview Deletes Without Mutating (v0.1.35)
- Problem: need a plan without deleting
- Solution: `delete --plan` (not `--confirm` / `--yes`)

```bash
atomwrite --workspace . delete --recursive --plan --older-than 7d logs/
atomwrite --workspace . delete --recursive --older-than 7d logs/
```


## How to Consume Watch Idle Output (v0.1.35)
- Problem: agents assumed empty stdout when no FS events
- Solution: always parse final `type:watch_summary`

```bash
atomwrite --workspace . watch . --idle-exit-ms 500 --max-events 20
# Needs a build with --features watch; the default build exits 78
```


## How to Merge With Conflict Markers (v0.1.35)

```bash
# b, o and t are files inside the workspace; a conflict leaves markers and the run exits 0
atomwrite --workspace . semantic-merge --base b --ours o --theirs t --output out.txt
atomwrite --workspace . semantic-merge --base b --ours o --theirs t --output out.txt --no-conflict-markers
# Exit 84 MERGE_CONFLICT on a conflict
atomwrite --workspace . semantic-merge --base b --ours o --theirs t --output out.txt --fail-on-conflict
```
