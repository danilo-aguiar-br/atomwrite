[Read in Portuguese / Leia em Português](README.pt-BR.md)


# atomwrite


- Atomic file operations for LLM agents: one CLI, all-or-nothing writes

[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)
[![Crates.io](https://img.shields.io/crates/v/atomwrite)](https://crates.io/crates/atomwrite)
[![License](https://img.shields.io/crates/l/atomwrite)](https://crates.io/crates/atomwrite)
[![MSRV](https://img.shields.io/badge/MSRV-1.90-orange)](https://github.com/danilo-aguiar-br/atomwrite/blob/main/Cargo.toml)
[![Downloads](https://img.shields.io/crates/d/atomwrite)](https://crates.io/crates/atomwrite)
[![Rust](https://img.shields.io/badge/rust-1.90%2B-blue)](https://www.rust-lang.org)

- Agent-readable summary: [llms.txt](llms.txt) and [llms-full.txt](llms-full.txt)
- Jump to: [Installation](#installation), [Quick Start](#quick-start), [Global Flags](#global-flags), [Commands](#commands-47-total--ssot), [Configuration](#configuration), [Exit Codes](#exit-codes)


## What Is It
- A single Rust binary that handles every file operation an LLM agent needs
- 47 subcommands (SSOT: clap `Commands` / `agent-surface`), all listed with an example in [Commands](#commands-47-total--ssot)
- Every write is atomic: tempfile, fsync, rename, fsync directory
- Every response is NDJSON: one JSON object per line, machine-readable by default
- Every file gets a BLAKE3 checksum: detect drift, verify integrity, enable optimistic locking


## Installation
### From crates.io
```bash
cargo install atomwrite
```

### Slim and full builds
- Features: `core`, `ast`, `lang-rust`, `lang-ts`, `lang-py`, `lang-go`, `lang-full`, `watch`, `full` (default = `core` + `ast` + `lang-rust` + `lang-ts` + `lang-py`; `watch` is opt-in; `semantic-search` is always compiled)
- Slim build (core only, 10,908,040 bytes measured on 2026-10-31, DERIVADO 10908040 / 1048576 = 10.40 MiB, local size-gate up to 15 MiB): `cargo install --path . --locked --force --no-default-features --features core`
- Full/default build (AST languages, 54.4 MiB measured on the v0.1.37 release build of this tree): `cargo install --path . --locked --force`
- Build with `watch`: `cargo install --path . --locked --force --features watch` (the default binary exits 78 on `watch` with a rebuild hint)
- Slim build: `outline` and `sparse outline` exit 78 with a rebuild hint (measured), and `query`, `scope`, `transform` and `write --syntax-check` have stubs for the same case (read from `src/commands/*_stub.rs` and `src/syntax_check_stub.rs`, the slim build was not executed to confirm)
- `cargo xtask check-features` checks the `core` and `core,watch` sets
- The features `i18n-cjk`, `i18n-rtl`, `i18n-europe`, `i18n-full` and `slow-tests` are placeholders or test switches: do not enable them
- The 5-8 MB target of the product plan applies to the core profile, not to the default AST build: the measured core build (10.40 MiB) is above that target and under the 15 MiB gate

### From source
```bash
git clone https://github.com/danilo-aguiar-br/atomwrite.git
cd atomwrite
cargo build --release
```

### Shell completions
```bash
# Bash
atomwrite completions bash > ~/.local/share/bash-completion/completions/atomwrite

# Zsh
atomwrite completions zsh > ~/.zfunc/_atomwrite

# Fish
atomwrite completions fish > ~/.config/fish/completions/atomwrite.fish
```


## Quick Start
```bash
# Write a file atomically from stdin
echo "hello world" | atomwrite --workspace . write src/hello.txt

# Read it back with checksum
atomwrite --workspace . read src/hello.txt

# Search across a directory
atomwrite --workspace . search 'hello' src/

# Replace text with atomic writes
atomwrite --workspace . replace 'hello' 'world' src/

# Evaluate math and unit conversions
atomwrite calc "2 hours + 30 minutes to seconds"

# Set a value in a TOML file (preserves comments)
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# Walk the AST of a Rust file (downloads the parser on first use, see Network And Privacy)
atomwrite --workspace . query src/main.rs --kinds

# Extract the outline of a Python file
atomwrite --workspace . outline src/app.py

# Real tree-sitter syntax check before writing (exit 88 on a syntax error)
echo "fn broken(" | atomwrite --workspace . write --syntax-check src/x.rs

# Report stale WAL sidecars and estimate the reclaim
atomwrite --workspace . wal-stats

# Reap terminal journals older than the threshold
atomwrite --workspace . wal-heal . --threshold-secs 3600
```


## Read Expansion In v0.1.37 (2026-10-31)
- `read` takes one or more paths: explicit paths are read in argv order, and an operand carrying `*`, `?` or `[` is a pattern the binary expands itself against one level of its own directory
- Pattern order is the `LC_ALL=C` shell order, taken from the encoded bytes of each name, so one code path sorts on all three targets, and only Linux was executed, while macOS and Windows stay unvalidated
- `--format raw` concatenates every target with no header, no separator and no newline of its own, so `atomwrite read --format raw '*.rs' | cmp - <(LC_ALL=C cat *.rs)` exits 0 with an empty diff
- The batch copies in 64 KiB blocks, so memory stays flat instead of holding the sum of every file, and `--timeout-secs` still cuts the copy with exit 124 because every block advances the cancel check
- A target that fails prints one line on stderr and the run continues with the next one, and the final status is the semantic code of the first failure: 4 for a missing path, 13 for a permission, 126 outside the `--workspace`
- `read --format raw` errors now go to stderr and never to stdout, because that channel promised bytes and a JSON envelope there was the defect
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` and `--verify-checksum` need a single target: naming several exits 65 with a message that names the flag
- `**` is out of scope: a pattern with `**` or with more than one separator exits 65 instead of matching something else in silence
- The default NDJSON format is unchanged, and one target in `--format raw` is byte-identical to 0.1.37
- `[profile.dev] opt-level = 1` cut the dev-profile fuzzy cost from 18533 ms to 4232 ms on the same 15 MB corpus
- Pin agents to `^0.1.37`. 47 subcommands (SSOT)


## What Is New In v0.1.37 (2026-10-31)
- Risk assessment now runs in every verb that writes: `edit`, `edit-loop`, `replace`, `case`, `transform` and `codemod` publish `risk_assessment` when the payload matches a content-risk pattern, exactly as `write` already did. The size barriers stay out of these verbs on purpose, because they rewrite a region and a shorter result is the operation working
- A grammar this host cannot provide is exit 69 `GRAMMAR_UNAVAILABLE` with class `precondition_failed`, never 255 `INTERNAL_ERROR`. `atomwrite doctor` reports the new `grammar_cache` check with how many of the 23 grammars the AST verbs can need are already cached
- `write.content_risk_patterns` is reachable with `config get` and `config set` as a list value, so the risk patterns are XDG configuration instead of a compiled literal (37 keys)
- `log.file` and `log.rotation` are accepted in the workspace scope too, and the `[log]` section of `.atomwrite.toml` now starts the log sink instead of being validated and discarded
- `batch --transaction` publishes one verdict: the `rollback` event carries `ok: false` with the `code` and `exit` of the failure that caused it, and the transaction locks every target in path order before the snapshot
- `--version` no longer appends `-dirty`, so an untracked file cannot change the string


## What Is New In v0.1.37 (2026-10-15 draft)
- Full history lives in [CHANGELOG.md](CHANGELOG.md), the upgrade steps live in [docs/MIGRATION.md](docs/MIGRATION.md)
- New `config` verb (the 47th subcommand): `config path|list|get|set|unset|doctor` reads and writes 37 persistent tuning keys, see [Configuration](#configuration)
- New `extract-artifact` verb: bounded signal from PDF, OOXML and image headers, never raw bytes
- Global stream shaping: `--sort-by`, `--dedupe`, `--aggregate`, `--join`, `--diff-against`, `--redact`, `--normalize-encoding`, `--truncate-content`, `--max-output-bytes`, see [Global Flags](#global-flags)
- The terminal success event carries `ok: true` (the per-item events of a stream, such as `entry` and `match`, do not), and a failing run ends in exactly one terminal event with `ok: false`
- Verbs that write take an explicit path (exit 2 without one), and the terminal event carries `target_resolved` and `target_source: "argv"`
- Every terminal event also carries `workspace_source`: `argv` when the root came from `--workspace`, and `cwd` when it came from the process working directory
- `edit-loop` is all-or-nothing: a pair with no match exits 65 and writes nothing, while `--partial` writes the matches and exits 87; the checksum it reads under the lock is the write base, so a concurrent edit exits 82 `STATE_DRIFT`
- Atomic writes take an automatic advisory lock (waits up to 5 s, exit 83), and `search`, `write`, `edit` and the other stdin verbs obey SIGTERM, SIGHUP and the deadline (exit 143, 129, 124)
- `watch` left the default feature set, and `--version` prints `atomwrite 0.1.37 (<sha>) <target triple>` inside a tracked checkout and `atomwrite 0.1.37 (no-git) <target triple>` outside one
- Changes that can break a consumer:
  - top-level JSON fields whose value was `null` are no longer emitted: test for an absent key, not for `null`
  - `--max-output-bytes` now requires `--allow-truncate`, and each flag alone exits 2
  - `semantic-merge` with a conflict exits 84 (`MERGE_CONFLICT`), and `apply` with a unified diff that does not match the file exits 65 and writes nothing
  - an invalid regex or glob exits 65, not 255, and `get`, `set` and `del` on an invalid TOML or JSON file exit 65
  - `case`, `transform`, `scope` and `codemod` can exit 82 (`STATE_DRIFT`) when a file changed during the run
  - `sparse list` applies `--include` and `--exclude`, and `search.max_inflight_bytes` starts at 4 MiB
  - the Rust library API changed, see [Library API](#library-api)
- Pin agents to `^0.1.37`. 47 subcommands (SSOT). CLI + XDG only (no product `ATOMWRITE_*` knobs; no product telemetry). Error envelopes always emit, with two deliberate exceptions: a broken pipe and a checksum divergence already published as one `hash_verify` event


## Why
- LLM agents juggle dozens of shell commands to manipulate files
- A crash mid-write can corrupt a file written in place
- Parsing unstructured CLI output wastes tokens and causes hallucinations
- Agents need checksums to detect concurrent edits but rarely compute them
- atomwrite solves all four problems with one `cargo install`


## Superpowers
### Atomic Writes
- Uses tempfile + fsync + rename + directory fsync on every write
- A reader sees the old file or the new file, never a half-written one
- Durability is set by `write --durability full|fast|auto` (default `auto`); a power cut was not simulated in the tests
- Optional real syntax check via tree-sitter (`--syntax-check` flag on `write`, exit 88)
- Optional WAL sidecar for crash recovery (`.atomwrite.journal.<target>.atomwrite.journal.json`)

### NDJSON Output
- stdout is ALWAYS structured JSON, one object per line
- Every object carries a `"type"` discriminator field
- Agents parse output without regex or brittle text scraping
- Errors also emit JSON with `error: true` and `ok: false` on stdout

### BLAKE3 Checksums
- Every `read` and `write` response includes a BLAKE3 hash
- Use `--expect-checksum` for optimistic locking on concurrent edits
- Detect state drift before applying changes

### Parallel Search
- Built on the ripgrep engine for file content search
- Respects `.gitignore` automatically
- Returns structured matches with file, line, column, and context

### AST-Aware Transforms
- `transform` and `codemod` rewrite by syntax tree through ast-grep 0.43, which supports 28 languages (measured: each of the 28 names is accepted, `zig` and `erlang` exit 65 `unsupported language`)
- `query`, `outline` and `--syntax-check` use `tree-sitter-language-pack` 1.22.1, whose manifest lists 372 languages (measured on 2026-10-31 against the manifest of that version)
- Automatic language detection by file extension covers 23 languages (`src/syntax_check.rs`); `--language NAME` accepts a parser that is already in the local cache
- Refactor code by syntax tree, not fragile regex

### Grammatical Scoping
- Select AST categories like comments, functions, classes, and strings
- Apply actions: delete, uppercase, lowercase, titlecase, squeeze, or replace
- Covers Rust, Python, JavaScript, TypeScript, and Go with prepared queries
- Use `--pattern` for custom AST patterns beyond the built-in queries

### Batch Operations
- Execute write, replace, delete, edit, hash, move, and copy operations from an NDJSON manifest
- Use `--transaction` for all-or-nothing execution with automatic rollback
- All operations in a batch share the same atomic guarantees

### Structured Config Editing
- `set` writes a value at a dotted path in TOML or JSON files (preserves comments via `toml_edit`)
- `get` reads a value at a dotted path with auto-detected format
- `del` removes a key (with `--force-missing` to treat missing keys as no-op success)
- `case` renames identifiers inside files via `heck` (snake, camel, pascal, kebab, screaming-snake)


## Usage
- All output goes to stdout as NDJSON
- Logs go to stderr: warnings and errors always, info and debug with `-v` / `-vv`
- Always pass `--workspace <DIR>` to restrict operations to a project root
- Use `--dry-run` before destructive operations
- Use `--expect-checksum <HASH>` for optimistic locking
- Use `--locale <TAG>` or `atomwrite locale --set` / `locale --clear` for UI locale (`en`, `pt-BR`); NDJSON codes stay English
- Configuration is CLI flags + XDG / `.atomwrite.toml` only, see [Configuration](#configuration)
- Pipe stdin for `write` and `batch` commands
- Agent-native stdout globals (all subcommands): `--select` / `--fields` (CSV top-level keys), `--filter KEY=VAL` (repeatable AND), `--limit N` (long-only: there is no global `-n`, and `replace` owns short `-n`)
- Error NDJSON always has `ok: false` and usually `match_count`: parse `exit` / `code` first; success shaping via `--filter`/`--limit` never drops error envelopes


## Global Flags
- 33 flags are accepted before or after the subcommand (source: `atomwrite commands --include-globals`); a hidden `--json` is also accepted and ignored because the output is always NDJSON
- Aliases: `--max-concurrency` for `-j` / `--threads`, `--timeout` for `--timeout-secs`, `--fields` for `--select`
- Flags named like a global but local to one command (for example `search --max-filesize`, 10 MiB by default) are listed in `atomwrite <command> --help`

| Flag | Default | What it does |
|---|---|---|
| `-v`, `--verbose` | off | raises log verbosity on stderr (`-v` info, `-vv` debug, `-vvv` trace) |
| `-q`, `--quiet` | off | lowers it (`-q` error, `-qq` off) |
| `--workspace <DIR>` | current directory | root of the path jail, always pass it |
| `--config <PATH>` | none | explicit `.atomwrite.toml`, fails if missing or malformed; replaces both files in `config get` and `config list` |
| `--color <WHEN>` | `auto` | `auto`, `always` or `never` |
| `--no-color` | off | disables colored output |
| `--ready-file <PATH>` | none | writes the PID after the signal handlers are installed (test harness only) |
| `--no-gitignore` | off | does not respect `.gitignore` files |
| `--hidden` | off | includes hidden files and directories |
| `--follow-symlinks` | off | follows symbolic links |
| `-j`, `--threads <N>` | automatic | worker bound; omitted or `0` = the smaller of logical CPUs, 16 and the RAM cap; an explicit `N` is honoured, also above 16 |
| `--max-filesize <BYTES>` | 1 GiB | rejects a larger file (`read`, `edit`, `hash` and `write` exit 65 `FILE_TOO_LARGE`), and on `write` it also bounds the stdin payload, so the flag names one ceiling and the payload and the target obey the same one; only `search` skips silently, with its own local 10 MiB limit counted in `files_skipped` |
| `--timeout-secs <N>` | 120 | global deadline, `0` disables it, exit 124 with a `cancelled` event that has `deadline_exceeded: true` |
| `--no-progress` | off | disables NDJSON progress events on long operations |
| `--json-schema` | off | emits the JSON Schema of the command output and exits |
| `--print-schema [NAME]` | none | lists the 55 schema names, or emits one schema; an unknown name exits 65 |
| `--non-interactive` | off | declares agent-hosted execution: never prompts, never reads a confirmation from stdin |
| `--confirm-side-effect` | off | acknowledges a side effect where a command contract asks for it; on a verb that writes nothing it is rejected with exit 65 `INVALID_INPUT` instead of being a silent no-op |
| `--allow-truncate` | off | opts in to value truncation, required by `--max-output-bytes` |
| `--locale <TAG>` | resolved | `en` or `pt-BR`; precedence: this flag, the saved preference, the OS language, `en` |
| `--no-auto-heal` | off | skips the startup `wal-heal` pass (3600 s threshold, 100 ms budget) |
| `--select <KEYS>` | all keys | keeps only the listed top-level keys (CSV) |
| `--filter <KEY=VALUE>` | none | keeps events whose key equals the value (`!=` and `==` work; repeatable, AND); never drops an error envelope |
| `--limit <N>` | none | work budget: the target list is cut before the side effect and the walk stops at the record that reaches the cap, so `--limit 1` touches exactly one target and a huge tree is not read to the end; and the terminal summary carries `walker_stopped_early: true` only when the walk was actually cut before the tree ended |
| `--sort-by <KEY>` | arrival order | sorts events by `path-line`, `line-path` or any key (buffers until the end) |
| `--dedupe <KEYS>` | off | drops events whose key tuple (CSV) was already seen |
| `--aggregate <KEY[:OP[:FIELD]]>` | off | groups buffered events by `KEY`; `OP` is `count`, `sum`, `min` or `max` |
| `--join <KEY:SUFFIX>` | off | reads the NDJSON table from stdin once (up to 16 MiB) and attaches the matching rows as a `SUFFIX` array to every data event that has `KEY`; events without `KEY` or without a match, failure events and the summary stay as they were; runs before `--select`, `--filter` and `--limit` (with `--sort-by`, `--dedupe` or `--aggregate`: after the sort, before the dedupe); exits 65 `INVALID_INPUT` for a TTY stdin, a table above 16 MiB, a line that is not a JSON object, a bad format, a `KEY` with no row holding a string value, or a command that already reads stdin (`write`, `edit`, `edit-loop`, `apply`, `extract`, `batch` without `--file`, `hash --stdin`, `calc` or `regex` reading stdin, `diff` with `-`) |
| `--truncate-content <KEY:LEN>` | off | cuts one string key to `LEN` characters |
| `--redact` | off | replaces secret-shaped values with `[redacted]` |
| `--normalize-encoding` | off | normalizes every string value to Unicode NFC |
| `--diff-against <PATH>` | off | compares the events against a baseline NDJSON file inside the jail and emits one `stream_diff` event |
| `--max-output-bytes <N>` | off | cuts every string value, `checksum` included; needs `--allow-truncate` and each flag alone exits 2 |

```bash
# Stream shaping examples
atomwrite --workspace . --sort-by path-line search 'TODO' src/
atomwrite --workspace . --aggregate type:count list src/ --depth 1
atomwrite --workspace . --dedupe kind list src/ --depth 1
atomwrite --workspace . --truncate-content content:20 read src/main.rs
atomwrite --workspace . --redact read src/main.rs
atomwrite --workspace . --normalize-encoding read src/main.rs
atomwrite --workspace . list src/ --depth 1 > base.ndjson
atomwrite --workspace . --diff-against base.ndjson list src/ --depth 1
atomwrite --workspace . --max-output-bytes 16 --allow-truncate --select path,checksum read src/lib.rs
atomwrite --workspace . --non-interactive read src/lib.rs
printf '{"path":"src/lib.rs","owner":"alice"}\n' | atomwrite --workspace . --join path:meta list src/ --depth 1
```


## Commands (47 total — SSOT)

### Core I/O
- `read` — read files with metadata, checksum, optional content (`--format raw` or `ndjson`, `--head`, `--tail`, `--lines`)
- `write` — create or overwrite files atomically via stdin (large existing targets need `--ack-overwrite`)
- `edit` — surgically edit by line number, text marker, or exact match
- `edit-loop` — apply N `old`/`new` pairs from NDJSON stdin in one atomic write
- `delete` — delete files with optional backup (`--plan` plan-only; `--confirm`/`--yes` rejected)
- `copy` — copy files with checksum verification
- `move` — move or rename files atomically (EXDEV copy-fallback)
- `apply` — apply patches from stdin (unified diff, search/replace, full, markdown)
- `scratch` — create agent staging dir under workspace (never OS `/tmp`)
- `verify` — verify file integrity by comparing BLAKE3 checksum

### Search and replace
- `search` — search file contents in parallel (ripgrep engine)
- `replace` — replace text across files with atomic writes
- `transform` — AST refactoring via ast-grep (28 languages)
- `codemod` — multi-rule codemod campaign (YAML rules, delegates to transform)
- `semantic-search` — offline token Jaccard search (no network, no embeddings API)
- `semantic-merge` — three-way line-based merge (not AST/embedding) for multi-agent writes

### Inspection
- `hash` — calculate BLAKE3 checksums
- `stat` — metadata alias for `read --stat` (size, mode, checksum; no body)
- `count` — count lines, files by extension or by size
- `diff` — compare two files (unified, stat, or changes)
- `list` — list files in a directory tree
- `sparse` — budgeted sparse list/read/outline (cap files and bytes for agent context)
- `extract` — extract fields from NDJSON input via pipe (positional fields or local `--field`; never local `extract --select`: global `--select` only projects output keys)
- `extract-artifact` — bounded signal from PDF, OOXML and image headers (never raw artifact bytes)
- `scope` — grammatical scoping (delete all comments, etc.)
- `regex` — generate regex from examples
- `calc` — math and unit conversions
- `doctor` — diagnose environment and dependencies
- `locale` — show resolved UI locale / persist preference (XDG)
- `commands` — emit full command tree as JSON for agent discovery
- `config` — persistent tuning keys: `config path|list|get|set|unset|doctor`, 37 keys, `--scope global|workspace` required on `set` and `unset` (and on `path`)
- `agent-surface` — agent tool surface manifest (integration by subprocess, CLI with NDJSON output)
- `watch` — watch filesystem events (needs a build with `--features watch`; the default binary exits 78)
- `completions` — generate shell completions (bash, zsh, fish, elvish, powershell)

### Backup and recovery
- `backup` — create timestamped backups with BLAKE3 checksums
- `rollback` — restore from a previous backup
- `prune-backups` — prune `.bak.YYYYMMDD_HHMMSS` backups by age or count
- `batch` — NDJSON-driven batch operations (transactional)
- `recipe` — named multi-step recipes (`list` / `run`)

### Structured config editors
- `set <PATH> <KEY_PATH> <VALUE>` — write a value at a dotted path in a TOML or JSON file, preserving comments and key order via `toml_edit`
- `get <PATH> <KEY_PATH>` — read a value at a dotted path, NDJSON `{"type":"get","key_path","value","found","format"}`
- `del <PATH> <KEY_PATH>` — remove a key, `--force-missing` treats a missing key as a no-op success
- `case <FILES...> --subvert OLD NEW --to <style>` — rename identifiers inside the given FILES via `heck` (styles: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`); a directory argument expands through the shared walker and honours `--include` and `--exclude`, and a directory that yields no file exits 65 instead of succeeding with nothing renamed

### AST tools (via tree-sitter-language-pack)
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` — walk a tree-sitter AST and emit nodes as NDJSON
- `outline <PATH> [--kind <KIND>] [--positions]` — extract high-level structure (functions, classes, structs, enums, traits, modules) as NDJSON; needs a file, not a directory
- `write --syntax-check` — real syntax check via tree-sitter, exit 88 with first error line, column, kind and message

### WAL cleanup
- `wal-stats` — telemetry snapshot: total journals, by-state breakdown, oldest age, total size, by-directory breakdown, auto-heal recommendation, estimated reclaim bytes
- `wal-heal <PATH> [--threshold-secs N] [--max-duration-ms N]` — reap stale terminal (committed or aborted) journals older than the threshold, and also remove a `Started` journal whose owning PID no longer exists, reported as `dead_owner_removed`

### Common patterns for each subcommand
```bash
# read
atomwrite --workspace . read src/main.rs
atomwrite --workspace . read --head 20 src/main.rs
atomwrite --workspace . read --format raw --lines 1:3 src/main.rs

# write
echo "fn main() {}" | atomwrite --workspace . write src/main.rs

# edit
atomwrite --workspace . edit src/main.rs --old "old text" --new "new text"

# edit-loop (N old/new pairs, one atomic write)
printf '%s\n' '{"old":"foo","new":"bar"}' '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace . edit-loop src/version.rs

# scratch: stage under .atomwrite/scratch/, never OS /tmp outside --workspace, for --old-file/--new-file
atomwrite --workspace . scratch init .atomwrite/scratch
atomwrite --workspace . edit src/main.rs \
  --old-file .atomwrite/scratch/old.txt \
  --new-file .atomwrite/scratch/new.txt

# verify
atomwrite --workspace . verify src/main.rs abc123def456

# search
atomwrite --workspace . search 'TODO' src/ --include '*.rs'

# replace
atomwrite --workspace . replace 'old_name' 'new_name' src/ --include '*.rs'

# hash
atomwrite --workspace . hash src/main.rs src/lib.rs

# delete
atomwrite --workspace . delete src/temp.rs --backup
atomwrite --workspace . delete --plan src/temp.rs

# count
atomwrite --workspace . count src/ --by-extension

# diff
atomwrite --workspace . diff src/old.rs src/new.rs --unified

# move
atomwrite --workspace . move src/old.rs src/new.rs

# copy
atomwrite --workspace . copy src/template.rs src/new_module.rs

# list
atomwrite --workspace . list src/ --depth 2

# extract: positional fields or local --field; NEVER local extract --select
atomwrite --workspace . search 'TODO' src/ | atomwrite extract path line
atomwrite --workspace . search 'TODO' src/ | atomwrite extract --field path --field line
# thin agent shape (global --select projects output keys only):
atomwrite --workspace . search 'TODO' src/ --select path,line --limit 20

# extract-artifact (PDF, OOXML or image header; an image here)
atomwrite --workspace . extract-artifact logo.png

# calc
atomwrite calc "2 GiB to bytes"
atomwrite calc "sqrt(144) + 3^2"

# regex
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"

# transform
atomwrite --workspace . transform -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/

# scope
atomwrite --workspace . scope src/ --lang rust --query comments --delete --dry-run

# batch
cat manifest.ndjson | atomwrite --workspace . batch --transaction

# backup
atomwrite --workspace . backup src/config.toml

# rollback
atomwrite --workspace . rollback src/config.toml

# prune-backups
atomwrite --workspace . prune-backups --max-age-secs 86400 src/config.toml
atomwrite --workspace . prune-backups --max-count 3 --dry-run src/config.toml

# apply
echo "new content" | atomwrite --workspace . apply src/file.txt --format full

# set
atomwrite --workspace . set Cargo.toml package.version 0.2.0

# get
atomwrite --workspace . get Cargo.toml package.version

# del
atomwrite --workspace . del Cargo.toml package.metadata.boring

# case: a file or a directory (the directory is expanded through the walker)
atomwrite --workspace . case src/main.rs --subvert user_id UserId --to pascal

# query
atomwrite --workspace . query src/main.rs --kinds
atomwrite --workspace . query src/main.rs -Q function_item --positions

# outline
atomwrite --workspace . outline src/app.py

# wal-stats
atomwrite --workspace . wal-stats

# wal-heal
atomwrite --workspace . wal-heal . --threshold-secs 3600 --max-duration-ms 100

# stat
atomwrite --workspace . stat src/lib.rs

# sparse
atomwrite --workspace . sparse list --max-files 100 --max-bytes 1048576 src/

# semantic-merge (line-based, not AST; exit 84 when there are conflicts)
atomwrite --workspace . semantic-merge \
  --base base.rs --ours ours.rs --theirs theirs.rs --output src/merged.rs

# recipe
atomwrite --workspace . recipe list
atomwrite --workspace . recipe run --name search-replace-verify \
  --path src --pattern unwrap --replacement expect --fuzzy auto --dry-run

# agent-surface
atomwrite --workspace . agent-surface

# watch (needs a build with --features watch; the default binary exits 78)
atomwrite --workspace . watch . --debounce-ms 200 --max-events 20

# codemod
atomwrite --workspace . codemod --rules rules.yaml --dry-run src/

# semantic-search
atomwrite --workspace . semantic-search "fn main println" src/ --k 10

# doctor
atomwrite --workspace . doctor
atomwrite --workspace . doctor --strict

# locale
atomwrite locale
atomwrite locale --set pt-BR
atomwrite locale --clear

# commands (full clap tree for agent discovery)
atomwrite commands
atomwrite commands --include-globals

# completions
atomwrite completions bash

# config (persistent tuning keys, see Configuration)
atomwrite --workspace . config list
atomwrite --workspace . config get fuzzy.threshold
atomwrite --workspace . config set fuzzy.threshold 0.9 --scope workspace
atomwrite --workspace . config unset fuzzy.threshold --scope workspace
```
- `codemod --rules` reads a YAML list of items with `language`, `pattern`, `rewrite` and an optional `id`; the ast-grep style `rule:` and `fix:` form exits 65
- `prune-backups --dry-run` takes no value: omit the flag to apply the pruning


## Configuration
- Precedence: CLI flags, then the workspace `.atomwrite.toml`, then the global XDG `config.toml` (the two files are merged key by key), then the built-in defaults
- `config path|list|get|set|unset|doctor` is the only way to change a persistent tuning key
- `config set KEY VALUE --scope global|workspace` and `config unset KEY --scope global|workspace` require `--scope` (exit 2 without it, nothing is touched), and `config path` takes it too
- `config list` and `config get KEY` take no `--scope`: they show `value`, `origin` (`default`, `global`, `workspace` or `explicit` with `--config PATH`) and `default`
- Scope `global` is the XDG `config.toml` (mode 0600), scope `workspace` is `.atomwrite.toml` in the workspace root
- `set` keeps comments, validates the whole file and writes by tempfile-fsync-rename without a backup; the terminal event carries `target_resolved` and `target_source: "argv"`
- An unknown key or a value out of range exits 65 and writes nothing; a file that is not valid TOML is never rewritten (`set` and `unset` exit 78); `config` works even with a broken `config.toml`
- A flag still wins over the configuration (for example `extract-artifact --max-text-chars`)

| Key | Type | Default | Accepted | Scopes |
|---|---|---|---|---|
| `fuzzy.mode` | string | `auto` | `auto`, `aggressive`, `off` | global, workspace |
| `fuzzy.threshold` | float | 0.7 | 0.0 to 1.0 | global, workspace |
| `fuzzy.threshold_aggressive` | float | 0.5 | 0.0 to 1.0 | global, workspace |
| `fuzzy.threshold_context` | float | 0.8 | 0.0 to 1.0 | global, workspace |
| `fuzzy.threshold_jw` | float | 0.85 | 0.0 to 1.0 | global, workspace |
| `fuzzy.soft_floor_delta` | float | 0.05 | 0.0 to 1.0 | global, workspace |
| `fuzzy.max_content_bytes` | integer | 268435456 (256 MiB) | 1 or more | global, workspace |
| `write.confirm_large_bytes` | integer | 102400 (100 KiB) | 1 or more | global, workspace |
| `log.file` | boolean | `false` | `true`, `false` | global, workspace |
| `log.rotation` | string | `daily` | `daily`, `hourly` | global, workspace |
| `storage.home` | string | none | a directory | global only |
| `locale` | string | none (resolved) | `en`, `pt-BR` | global only |
| `concurrency.max_threads` | integer | 16 | 1 to 256 | global, workspace |
| `concurrency.ram_fraction_percent` | integer | 50 | 1 to 100 | global, workspace |
| `search.max_inflight_bytes` | integer | 8388608 (8 MiB) | 4194304 (4 MiB) to 1073741824 (1 GiB) | global, workspace |
| `summary.errors_max_entries` | integer | 50 | 1 to 1000 | global, workspace |
| `lock.acquire_timeout_ms` | integer | 5000 | 100 to 3600000 | global, workspace |
| `retry.base_ms` | integer | 50 | 1 to 60000, not above `retry.max_ms` | global, workspace |
| `retry.max_ms` | integer | 400 | 1 to 60000 | global, workspace |
| `retry.max_elapsed_ms` | integer | 2000 | 1 to 3600000 | global, workspace |
| `extract.max_sample_rows` | integer | 10 | 0 to 100000 | global, workspace |
| `extract.max_text_chars` | integer | 400 | 1 to 10000000 | global, workspace |
| `extract.max_sheets` | integer | 32 | 1 to 10000 | global, workspace |
| `extract.max_slides` | integer | 64 | 1 to 10000 | global, workspace |

- A scope outside the last column exits 65 (for example `config set storage.home /srv --scope workspace`)
- These fields are `config` keys like every row above, and each answers `config get`: `defaults.backup`, `defaults.retention`, `defaults.line_ending`, `defaults.max_filesize`, `write.shrink_block_percent`, `write.auto_rotate_max_age_secs`, `write.content_risk_patterns`, `watch.idle_exit_ms`, `watch.debounce_ms`, `search.context`, `search.smart_case`, `search.max_filesize` and `search.max_columns`; an unknown section in `.atomwrite.toml` makes `read` exit 78 (measured)
- Files on Linux: global config `$XDG_CONFIG_HOME/atomwrite/config.toml`, saved locale `$XDG_CONFIG_HOME/atomwrite/locale`, log `$XDG_STATE_HOME/atomwrite/logs/atomwrite.<date>.jsonl` (only with `log.file = true`, file mode 0600, directory 0700), parsers `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/`
- macOS and Windows resolve the same files through the `directories` crate; those paths were read from the code and not run there
- `storage.home` moves `config` and `state` under `{home}/config` and `{home}/state`

```bash
atomwrite --workspace . config get fuzzy.max_content_bytes
atomwrite --workspace . config set fuzzy.max_content_bytes 1000000 --scope global
```


## Environment Variables
- Product policy: atomwrite does not use product environment variables for runtime knobs (workspace, backup, locale, log filter, home override, WAL, etc.)
- Configure via CLI flags and XDG storage / `.atomwrite.toml` (and optional `--config`)
- `doctor` reads no environment variable: host detection (WSL, container, Kubernetes, Termux, Flatpak, Snap) uses file probes only, and it is not a configuration API
- Dependencies read some variables on their own, never as atomwrite configuration: the OS-language crate (`LANG`, `LC_ALL`, `LC_MESSAGES`, `LANGUAGE`), the XDG crates (`HOME`, `XDG_*`) and the parser download client (`ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`); see ADR-0057 in [docs/decisions/](docs/decisions/README.md)
- Shell-level variables used by your agent harness (PATH, etc.) are outside atomwrite product config


## Network And Privacy
- No telemetry: atomwrite sends nothing about you or your files
- The only network traffic is the parser download used by `query`, `outline` and `write --syntax-check` (the last one read from the source, not run); `transform`, `scope` and `codemod` use ast-grep parsers compiled into the binary
- On the first use of a language, the binary downloads the parser bundle for the platform from `https://github.com/xberg-io/tree-sitter-language-pack/releases/download/v1.22.1/` (for Linux x86_64 the file `parsers-linux-x86_64.tar.zst`, 23031807 bytes in the manifest, about 22 MiB, plus 3.9 MiB of extracted parser libraries for the six languages this machine used so far); the manifest carries a `sha256` for each platform bundle
- The cache lives at `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` (`bundles`, `libs`, `manifest.json`), and the download happens only while the needed parser is missing there
- The download client honours the proxy variables listed in [Environment Variables](#environment-variables)
- Offline use: fill the cache directory on a host with network access and copy it to the offline host
- A panic writes a crash report file `report-<uuid>.toml` in the OS temporary directory (through the `human-panic` crate) and prints its path; nothing is transmitted, and you choose whether to send it
- Details: [PRIVACY.md](PRIVACY.md)


## Exit Codes
- Error envelopes on stdout are always NDJSON with `ok: false` (and usually `match_count`); read `exit` / `code` before treating a line as success
- The list below is derived from the error enum and checked against `agent-surface` and `--help`
<!-- exit-codes:begin -->
- `0` SUCCESS: success
- `1` NO_MATCHES: no matches found (search, replace, transform, scope; not a crash)
- `1` DOCTOR_CHECKS_FAILED: `doctor --strict` found failed or warned checks (the report event carries every check)
- `2` ARGUMENT_PARSE_ERROR: command line rejected by the parser
- `2` UNSUPPORTED_COMBINATION: two flags this verb refuses to combine (read `flag` and `other` in the envelope)
- `4` FILE_NOT_FOUND: file not found
- `4` GLOB_NO_MATCHES: pattern expanded to no file
- `13` PERMISSION_DENIED: permission denied
- `28` DISK_FULL: disk full (no space left on device)
- `30` QUOTA_EXCEEDED: quota exceeded
- `65` INVALID_INPUT: invalid input (bad arguments or malformed data, including empty stdin without `--allow-empty-stdin` and a file above `[fuzzy].max_content_bytes` under fuzzy)
- `65` MATCH_FAILED: pattern did not match, or an `edit-loop` pair has no match (read `best_candidate`)
- `65` MATCH_AMBIGUOUS: pattern matched more than once (read `best_candidate`)
- `65` BINARY_FILE: binary content rejected
- `65` FILE_TOO_LARGE: file above the size limit
- `69` GRAMMAR_UNAVAILABLE: the tree-sitter grammar for a language is not on this host (class `precondition_failed`, retryable `false`)
- `73` CROSS_DEVICE: cross-device rename (filesystem boundary)
- `74` IO_ERROR: I/O error
- `78` CONFIG_INVALID: configuration invalid, or a verb that needs a feature missing from this build (slim)
- `80` TRANSACTION_ROLLBACK_FAILED: batch transaction rollback failed (backups kept)
- `81` CHECKSUM_VERIFY_FAILED: checksum verify failed
- `82` STATE_DRIFT: state drift (checksum mismatch, optimistic lock failed)
- `83` LOCK_TIMEOUT: lock not acquired within `lock.acquire_timeout_ms` (default 5000)
- `84` MERGE_CONFLICT: `semantic-merge` found conflicts
- `85` FIFO_DETECTED: FIFO detected (named pipe cannot be atomically written)
- `86` DEVICE_FILE: device file detected (block or character device)
- `87` EDIT_LOOP_PARTIAL: `edit-loop --partial` wrote the matching pairs and some pairs missed
- `88` SYNTAX_ERROR_DETECTED: syntax error detected (`--syntax-check` failed)
- `91` EXDEV_FALLBACK_DISABLED: EXDEV fallback disabled (cross-device copy-fallback is not allowed)
- `92` COPY_BACK_BLAKE3_FAILED: copy-back BLAKE3 verification failed
- `93` ORPHAN_JOURNAL: orphan journal detected
- `124` TIMEOUT: global `--timeout-secs` deadline exceeded (default 120; pass `0` to disable)
- `126` WORKSPACE_JAIL: workspace jail violated (path escapes workspace)
- `127` SYMLINK_BLOCKED: symlink blocked (target outside workspace)
- `128` IMMUTABLE_FILE: file immutable (cannot modify)
- `129` SIGHUP: terminal hangup (SIGHUP)
- `130` CANCELLED: run cancelled before it finished (status follows the cause)
- `130` SIGINT: interrupted by SIGINT
- `141` BROKEN_PIPE: broken pipe (SIGPIPE or a `BrokenPipe` write error)
- `143` SIGTERM: terminated by SIGTERM
- `255` INTERNAL_ERROR: internal error
<!-- exit-codes:end -->


## Signal Handling
- Unix: SIGINT (Ctrl+C), SIGTERM and SIGHUP are intercepted for a cooperative shutdown with exit 130, 143 and 129; the run ends with a `cancelled` event that carries `signal` and `deadline_exceeded`
- Unix: SIGPIPE has a handler, it is neither ignored nor left at its default action; the handler marks a cooperative shutdown with exit 141
- The default SIGPIPE action killed the process before any destructor ran and left lock sidecars, journals and tempfiles of the writes in flight; with the handler every `Drop` runs and nothing is left behind
- A write that fails because the reader closed exits 141 (`BROKEN_PIPE`) with no panic and no crash report: `completions`, `--print-schema`, `--help` and every verb (fixed in 0.1.37, test `cli_v0137_broken_pipe_outputs`)
- Very short output that fits in the pipe buffer (for example `--version | head -1`) can finish before the reader closes and then exits 0
- A write that hits the closed pipe fails with `BrokenPipe` and exits 141 as well, and a parent that ignores SIGPIPE gets the same exit 141
- A repeated SIGPIPE never forces an exit; a repeated SIGINT, SIGTERM or SIGHUP does
- Windows: Ctrl+C intercepted via console handler; there is no SIGPIPE, and `BrokenPipe` from `std` follows the same path (not validated there)
- First signal: sets the shutdown flag and prints a message on stderr in the active locale (`atomwrite: shutting down...` with `--locale en`)
- Second signal: immediate process termination via `_exit` (Unix) or `exit` (Windows)
- Walker threads (search, replace, transform, scope) stop between files
- Batch operations stop between operations
- Stdin verbs (`write`, `edit` and the others) obey the signals and the deadline while stdin is still open: SIGTERM exits 143 (measured)


## Error Handling
- All errors emit a JSON object on stdout with `error: true` and `ok: false`
- Error fields: `code`, `exit`, `message`, `path`, `error_class`, `retryable`, `suggestion`, `workspace`
- Every summary that publishes `files_failed` adds `partial: true` and `errors[]` beside it (each entry has `path`, `code` and `message`, `path` is `<walk>` when the walk failed without naming a file, at most 50), and that covers `search`, `transform`, `replace`, `scope`, `count`, `list`, `sparse`, `semantic-search` and `case`
- Error classes: `permanent`, `transient`, `conflict`, `precondition_failed`
- 35 error variants in `src/error/variants.inc.rs`
- Transient and conflict errors set `retryable: true`
- The `suggestion` field provides actionable recovery guidance for agents, in the active locale (`suggestion_lang`)
- See `docs/schemas/error-output.schema.json` for the full contract


## Performance
- Single binary; on Linux it links dynamically against the system C library and nothing else (`ldd` on the v0.1.37 build lists `libgcc_s.so.1`, `libm.so.6` and `libc.so.6`)
- Release builds use LTO, single codegen unit, and symbol stripping
- Memory-mapped file reads via `memmap2` for large files
- Parallel multi-file work is the default modus operandi: `ignore::WalkParallel` / `collect_files_parallel` (search content + `--target files`, replace, count, transform, scope, recursive discovery) + `rayon` (hash, backup, multi-path + recursive delete/copy, case multi-path, semantic-search scoring, non-txn batch, prune multi-target, sparse read, diff dual-read)
- Worker bound: `--threads` / `-j` / `--max-concurrency` (default = the smaller of the logical CPUs, 16 threads and the RAM cap; an explicit `-j N` is honoured as asked; see `src/concurrency.rs`)
- Small-file latency: medians of 5.6 ms (`read`) and 6.7 ms (`write`) in one session of 20 runs, and 9.4 ms and 9.7 ms in another of 30 runs, whole process included; machine: Intel Xeon E5-2699 v3 2.30 GHz with 72 logical CPUs, Linux, working directory on tmpfs, measured on 2026-10-19 while other jobs ran on the same host


## Known Limits
- `replace --expect-checksum` compares the checksum under the lock, so a writer that lands between the check and the write is detected as exit 82 `STATE_DRIFT`; verbs that read, change and write back (`edit`, `edit-loop`, `replace`, `set`, `del`, `config set`, `apply`, `write --append`) serialize under the lock
- A target name long enough to push the sidecar past 255 bytes gets a short BLAKE3 digest as the sidecar name, so a 240-byte target locks instead of failing with `ENAMETOOLONG`; a 244-byte target was proven with `replace`
- The directory of the target must allow creating that lock sidecar: writing to an existing file inside a read-only directory exits 13 even when the file itself is writable
- The RAM ceiling for the automatic worker count reads `MemAvailable` from `/proc/meminfo` on Linux only and ignores container (cgroup) limits
- Without `--sort-by`, the order of `search` results varies from run to run because the walkers run in parallel
- `case` expands a directory argument through the shared walker and honours `--include` and `--exclude`; a directory that yields no file exits 65 instead of succeeding with nothing renamed
- `query`, `outline` and `write --syntax-check` detect only 23 languages by extension; `--language NAME` works for a parser already extracted in the local cache, and exits 65 `unsupported language override` for a language not yet used
- `extract-artifact` emits a single `artifact` event and no summary, so a successful run has no event with `ok: true`
- `locale --set` and `locale --clear` print the resolved language from before the change; a following `atomwrite locale` shows the new state
- macOS and Windows were not run; the Unix-only code has `cfg(windows)` pairs that were only compiled with `cargo check` from Linux


## Library API
- The supported interface of atomwrite is the CLI and its NDJSON contract; the crate also ships a Rust library target that is not covered by that promise
- The public Rust API of v0.1.37 changed against the published 0.1.36 in ways SemVer treats as breaking: `cargo semver-checks` 0.50.0 reported 10 failed major checks (removed `locale::Idioma` and `locale::{negotiate_to_idioma,resolved_idioma}`, removed `LocaleSource::EnvVar`, removed `signal::reset_sigpipe`, four removed lock constants, `LockGuard` no longer `Send` and `Sync`, new `Commands` variants and new public struct fields without `#[non_exhaustive]`, and `watch` no longer a default feature)
- Pin `atomwrite` as a library dependency to an exact version (`=0.1.37`) and read [CHANGELOG.md](CHANGELOG.md) before every upgrade


## Troubleshooting FAQ

### atomwrite write hangs with no output
- Ensure you are piping content to stdin
- `write` reads from stdin and waits for EOF
- Example: `echo "content" | atomwrite --workspace . write file.txt`

### search returns exit code 1
- Exit code 1 means zero matches were found
- This is expected behavior, not an error
- Check the pattern and target path

### search with a pattern that starts with a hyphen
- A pattern that starts with `-` goes after `--`, the way `rg` and `grep` do it
- `atomwrite --workspace . search -F -- '-deprecated' src/` searches for that literal text
- Without `--` the token is a flag the verb does not have, so the run exits 2 and prints the parser tip `to pass '-Z' as a value, use '-- -Z'`

### search, replace and transform exit 4 although they printed matches
- A root named in the invocation that does not exist fails the run even when the other roots matched, in `search`, `replace` and `transform`
- The matches still reach stdout, and the summary carries `ok: false`, `code: FILE_NOT_FOUND`, `files_failed` and the path in `errors[]`
- A file that fails during the walk is the other case: `partial: true` joins the summary, and the run keeps exit 0 in `search`, `replace`, `transform`, `scope`, `case`, `sparse`, `semantic-search` and `backup`, while `count`, `list`, `hash`, `copy`, `delete` and `prune-backups` exit with the first failure because their answer is a total over the tree; the contract is ADR-0077 (`docs/decisions/0077-answer-family-and-walk-honesty.md`)

### cross-device rename fails with exit 73
- The source and destination are on different filesystems
- atomwrite falls back to copy+delete for `move` across devices
- Use `copy` followed by `delete` as an alternative

### checksum mismatch with exit 82
- Another process modified the file between read and write
- Re-read the file to get the current checksum
- Retry the operation with the updated `--expect-checksum`

### workspace jail violated with exit 126
- The target path resolves outside the `--workspace` boundary
- Verify the path does not contain `..` traversals or symlinks escaping the workspace

### syntax check failed with exit 88
- The real tree-sitter syntax check found a syntax error in the file
- Inspect the first error line/column/kind/message in the JSON error envelope
- Fix the syntax and retry, or remove `--syntax-check` to bypass

### empty stdin rejected with exit 65
- The guard rejected 0 bytes from stdin as a likely upstream-pipeline failure
- Pass `--allow-empty-stdin` to confirm the empty input is intentional
- Or pipe real content: `echo "x" | atomwrite --workspace . write file`

### edit hangs (or fails with exit 65) on --after-line/--after-match/--between/--multi
- These modes read replacement content from stdin, same as `write`
- A terminal stdin fails fast with exit 65 `INVALID_INPUT` instead of blocking forever
- Pipe content: `echo "new text" | atomwrite --workspace . edit file.rs --after-line 10`
- Or use `--new-file <PATH>` where the mode accepts it
- `--after-match`/`--before-match`/`--between`/`--after-line`/`--before-line`/`--range`/`--delete-range` reject `--old`/`--new`/`--old-file`/`--new-file` at parse time (exit 2): these modes only take content from stdin

### watch exits 78
- The default binary is built without the `watch` feature
- Rebuild with `cargo install --path . --locked --force --features watch`


## MSRV And Targets
- Minimum supported Rust version: 1.90 (edition 2024); `rust-toolchain.toml` pins channel 1.94 for development
- Locales: `en` and `pt-BR`
- Linux x86_64 is exercised end to end
- Windows is checked with `cargo check --target x86_64-pc-windows-gnu` from a Linux host only
- macOS was not exercised on a real Mac


## Documentation
- [docs/INSTALL.md](docs/INSTALL.md): install, features and build profiles
- [docs/HOW_TO_USE.md](docs/HOW_TO_USE.md): full usage guide
- [docs/COOKBOOK.md](docs/COOKBOOK.md): task recipes
- [docs/AGENTS.md](docs/AGENTS.md): contract for LLM agents
- [docs/CROSS_PLATFORM.md](docs/CROSS_PLATFORM.md): platform notes
- [docs/MIGRATION.md](docs/MIGRATION.md): upgrade paths between versions
- [docs/TESTING.md](docs/TESTING.md): how the test suite is organized
- [INTEGRATIONS.md](INTEGRATIONS.md): using atomwrite from agents and scripts
- [PRIVACY.md](PRIVACY.md): what the binary reads, writes and downloads
- [llms.txt](llms.txt) and [llms-full.txt](llms-full.txt): summaries for LLM context windows


## Architecture
- See [ARCHITECTURE.md](ARCHITECTURE.md) for module map, data flow, and design decisions
- See [docs/decisions/](docs/decisions/README.md) for the index of 60 ADRs (0019 to 0078; 3 more live in `docs/adr/`)
- Verification is running the program, not a test suite: every claim in this README is backed by a real invocation of the binary, and `docs/TESTING.md` describes the harness for those cases a maintainer chooses to keep
- See [docs/schemas/](docs/schemas/README.md) for the JSON Schema contracts of the NDJSON output, where 54 files describe the 55 names `atomwrite --print-schema` lists


## Contributing
- See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, local DoD, PR process, and release notes
- See [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md) for the Portuguese guide
- Follow the [Code of Conduct](CODE_OF_CONDUCT.md) / [Código de Conduta](CODE_OF_CONDUCT.pt-BR.md)


## Security
- See [SECURITY.md](SECURITY.md) for vulnerability reporting
- See [SECURITY.md](SECURITY.md) section "Known Security Advisories" for resolved and active advisories
- See [PRIVACY.md](PRIVACY.md) for data handling


## Changelog
- See [CHANGELOG.md](CHANGELOG.md) for English release history
- See [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) for Portuguese release history


## License
- Licensed under MIT OR Apache-2.0
- See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for details
