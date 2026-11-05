[Leia em Português](INTEGRATIONS.pt-BR.md)


# Integrations


- atomwrite is a CLI that talks NDJSON on stdout: any agent that can run a shell command can call it as a subprocess


## Objectives
- Spend fewer tokens: one NDJSON object per call replaces the shell pipeline, and `commands --brief` answers in 5 020 bytes where the full tree costs 67 492
- Lower the financial cost, because the bill follows the tokens and the round trips, and one call replaces the several shell calls a file task needs
- Reduce errors with an atomic write and a BLAKE3 checksum on every response, so a stale or partial result is caught instead of assumed
- Reduce hallucinations with a `type` discriminator, a closed schema per verb and an exit code that names the failure
- Reduce the time spent on each task, because the read, the search and the write run in one process instead of a chain of shell tools
- Reduce latency with local execution and no network on the file path, measured at medians of 5.6 ms on `read` and 6.7 ms on `write` for small files
- Multiply the speed of operations with `batch`, `edit-loop`, `replace`, `case`, `transform` and `codemod` settling many files in one call
- Cut the lead time between the request and the verified result, because the response already carries the checksum, the `target_resolved` and the next step
- Make the agent faster, more effective and more assertive, since the exit code, the `code` and the `suggestion` say what to do next without a retry guess


## Compatible Agents (v0.1.38)
- Current surface: 47 subcommands (SSOT via `atomwrite agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`), all listed with a tested example in [All 47 Commands](#all-47-commands)
- Historical: 41 subcommands at v0.1.29, and 46 before the `config` verb landed in v0.1.37 (the large write `--ack-overwrite`, `watch_summary`, merge markers and delete `--plan` came in that window)
- Agent integration (pin `^0.1.37` until 0.1.38 is published, and `^0.1.38` from that release on): discover tools with `agent-surface` / `commands`, stage large or fragile patches via `scratch init` + `--old-file`/`--new-file` under `.atomwrite/scratch/`, parse the exit code first, then treat every error line as `ok: false` (always-emit even under `--filter`)
- Bounded `read` (v0.1.38): the cap is `read.max_output_bytes` (24,000 bytes), the event carries `truncated` and `next`, which spells the range that follows, and `--all` lifts the cap
- `read` selectors compose in one pass: repeatable `--lines` with `--range` as its alias, `--number`, `--symbol NAME`, `--line N --enclosing` and `--no-comments` (`--no-comments` requires `--number` and exits 65 without it)
- Text shapes for shell pipelines (v0.1.38): `search --format text` with `search.format`, `list --format text` and `outline --format tsv`, while the default output stays NDJSON
- `search` exits 89 with `ALL_SKIPPED_BINARY` when every candidate was binary, which is a different answer from `NO_MATCHES` (exit 1)
- `apply` matches whole lines, so every SEARCH line must match the file line from its first to its last byte; `--show` prints the changed lines with their numbers, and `--all` replaces every whole-line occurrence
- `apply --format unified` accepts `@@ ... @@` and a bare `@@`, matches a hunk without numbers by context, and refuses more than one match by naming the line of each
- `apply` refuses a patch that breaks the parse of the edited file, `--no-syntax-check` skips that gate, and one patch can carry several files when each block names its own path line
- `MATCH_AMBIGUOUS` (exit 65) means the SEARCH matched more lines than the call allowed, so tighten the block or pass `--all`, and `apply.max_ellipsis_lines` bounds the `...` gap
- `diff` reports 1-based `line`, `old_line` and `new_line`, so the numbers line up with `read --number`
- `delete --recursive` owns the whole tree and publishes `backups_kept`, `backup_bytes` and `dirs_left` when a backup or a leftover exists
- `output.profile` is `full` or `lean`: `lean` drops the prose fields, and `--select`/`--fields` compose with both
- `commands --brief` prints the tree without the long descriptions, and `-h` on a subcommand lists only that verb's own flags
- The failure envelope is `{"ok":false,"code":"...","exit":N,"path":"...","suggestion":"..."}`, and `message`, `error_class`, `retryable` and `suggestion_lang` appear only under `-v`
- `target_resolved` is always an array, even when the call names a single path
- Prefer the global `--select`/`--fields`, `--filter`, `--limit` over jq for field projection
- atomwrite requires only shell access to function
- NDJSON output (one JSON object per line) is parseable by any JSON parser without a custom adapter
- No plugins, extensions or SDKs are required or offered: integration is a subprocess, by design
- Validation status: only the agents marked validated in the [Summary Table](#summary-table) were run, every other agent is marked not validated
- Tuning without editing TOML by hand: `config list` shows every key with its effective value and origin, `config set KEY VALUE --scope global|workspace` writes one key atomically (`--scope` is required, there is no default), `config unset` removes it
- One terminal event per run: when a summary fails it carries `ok: false`, `code` and `exit` itself
- Read `partial` and `files_failed` in every summary that publishes them (`search`, `transform`, `replace`, `scope`, `sparse`, `semantic-search` and `case`), and expect `MATCH_FAILED` (exit 65) when an `edit-loop` pair has no match
- `count`, `list`, `hash` and `copy` fail on the first read failure instead, and the two families are fixed by ADR-0077 (`docs/decisions/0077-answer-family-and-walk-honesty.md`)
- Global agent-native stdout shape (v0.1.36): `--select`/`--fields`, `--filter`, `--limit` (no global `-n`), errors always emit
- Global `--timeout-secs` defaults to 120 (pass `0` to disable, exit 124 on deadline)
- Fuzzy multi-apply is one-pass on the original content, never re-scans inserted text, defaults to a single apply and treats a pattern embedded in its replacement as a single apply
- Fuzzy cascade default for agents: `--fuzzy auto|aggressive`, and `--fuzzy off` = exact-only
- On successful `edit --replace-all` parse `match_count`, on indent flex parse `indent_adjusted`
- Read `best_candidate` on match failures (exit 65) instead of re-reading whole files
- Use `agent-surface` for the tool manifest, integration is by subprocess, CLI with NDJSON output
- Prefer `recipe list|run` for multi-step search-replace-verify pipelines (excludes `*.bak.*`, including the hash step)
- Backup method is `reflink_or_copy`, never a hardlink of the live file
- Exit codes: read the table with `atomwrite agent-surface --format json | jaq '.exit_codes'` (42 entries, 34 distinct numbers) and branch on `exit` and `error_class`
- Slim install: `cargo install --path . --locked --force --no-default-features --features core`
- In the slim build the AST verbs `outline`, `query`, `scope`, `transform` and `sparse outline` fail closed with exit 78 and a rebuild hint (read from `src/commands/*_stub.rs`, the slim build was not executed for this document)
- The default build has no `watch`: the `watch` verb exits 78 until you build with `--features watch`
- Pin agents to `^0.1.37` until `0.1.38` lands on crates.io, then to `^0.1.38`
- Platform status: Linux is exercised end to end, Windows is checked with `cargo check --target x86_64-pc-windows-gnu` from a Linux host, macOS is not exercised on a real Mac (see `CHANGELOG.md`, 0.1.36)
- History: v0.1.15 fixed G117 (multi-pair `edit` fuzzy parity, `pair_results`, `failed_pair_index`, opt-in `--partial`) and G118 (`write` resolves the target against the workspace, so append and checksum are safe with a divergent CWD)
- History: v0.1.16 added G119 L1 WalPolicy and L4 HeuristicsEngine, v0.1.17 wired the G119 L3 startup heal, v0.1.18 added G118 universal resolve-first propagation, G120 L3 cross-flag tests and G117 edge cases


## Vision And Documents
- `read photo.png` answers one `data:` URI line for the vision channel, and `--max-tokens`, `--crop` and `--high-res` pick the cost before the encode
- `read doc.pdf` answers the text of each page, `--pages 1-3` picks a range, `--page N` renders that page as a `data:` URI, `--dpi N` sets the resolution and `--ocr` returns the recognized text in `content`
- `extract-artifact` gives a bounded header signal from PDF, OOXML and image, never raw bytes
- The two OCR models and the parser bundles are the only downloads, and `PRIVACY.md` describes both caches


## Summary Table

| Agent | Validation | Evidence |
|-------|------------|----------|
| Claude Code | Validated on 2026-10-19 | every example of this guide was run from its shell tool |
| Custom Agents | Validated on 2026-10-19 | the Python subprocess snippet below was run |
| Cursor | Not validated | no run recorded |
| Windsurf | Not validated | no run recorded |
| Codex CLI | Not validated | no run recorded |
| Aider | Not validated | no run recorded |
| Continue | Not validated | no run recorded |
| Cline | Not validated | no run recorded |
| Roo Code | Not validated | no run recorded |
| Amazon Q Developer | Not validated | no run recorded |
| GitHub Copilot CLI | Not validated | no run recorded |


## All 47 Commands
- Set `WS` once to the absolute path of the workspace root before running an example: `WS="$PWD"`
- Every example ran with exit 0 on 2026-10-19 against `atomwrite 0.1.37` in a throwaway workspace, in the order below, except `watch` (see its entry)
- The v0.1.38 flags quoted in the bullets above were run on 2026-11-04 against `atomwrite 0.1.38`, and the 47 examples below were not re-run in full for this version
- The examples assume the files `src/a.rs` (with an `unwrap()` call), `src/b.rs` (with the word `beta`), `src/c.rs` (with `user_id`), `src/tmp.rs`, `config.toml` (with `package.name` and `features.x`), `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` and `rules.yaml`
- `read` — read a file with checksum, size and lines, optionally only head, tail, a line range, a named symbol or the block around one line, under a byte cap that `--all` lifts. Example: `atomwrite --workspace "$WS" read --lines 1:3 --number src/a.rs`
- `write` — create or overwrite a file atomically from stdin. Example: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` — edit a file by exact or fuzzy text match, line number or marker. Example: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `delete` — delete files or a whole tree with `--recursive`, or only print the plan with `--plan`; a tree delete reports `backups_kept`, `backup_bytes` and `dirs_left`. Example: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `copy` — copy a file or tree with checksum verification. Example: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `move` — move or rename a file atomically. Example: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `apply` — apply a patch (unified diff, SEARCH/REPLACE, full file) read from stdin, matching whole lines, with `--all`, `--show` and `--no-syntax-check`. Example: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `search` — search file contents in parallel with the ripgrep engine, with `--format text` for a shell pipeline and exit 89 when every candidate was binary. Example: `atomwrite --workspace "$WS" search alpha src/`
- `replace` — replace text across files with atomic writes and fuzzy matching. Example: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `transform` — rewrite code by AST pattern (ast-grep). Example: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `hash` — print the BLAKE3 checksum of files. Example: `atomwrite --workspace "$WS" hash src/a.rs`
- `count` — count lines and files by extension or by size. Example: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` — compare two files (unified, stat or changes), with 1-based `line`, `old_line` and `new_line`. Example: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `list` — list a directory tree with metadata, in NDJSON or `--format text`. Example: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` — pick fields from NDJSON read on stdin. Example: `printf '{"a":1}\n' | atomwrite extract --field a`
- `extract-artifact` — bounded signal from PDF, OOXML and image headers, never raw bytes. Example: `atomwrite --workspace "$WS" extract-artifact img.png`
- `scope` — grammatical scoping: act on comments, strings, functions and other code categories. Example: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `regex` — generate a regex from example strings. Example: `atomwrite regex abc-12 xyz-99 id-7 --digits`
- `calc` — math and unit conversion. Example: `atomwrite calc "2 hours to seconds"`
- `completions` — print or install shell completions (bash, zsh, fish, elvish, powershell). Example: `atomwrite completions bash > /dev/null`
- `backup` — create timestamped backups with BLAKE3 checksums. Example: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` — restore a file from a previous backup (run `backup` first). Example: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `batch` — run NDJSON operations from stdin, optionally as one transaction. Example: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
- `edit-loop` — apply N old/new pairs in one write, all or nothing (run the `edit` example first). Example: `printf '[{"old":"gamma","new":"delta"}]\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` — remove old `.bak` files by age or count. Example: `atomwrite --workspace "$WS" prune-backups -r --max-age-secs 86400 --dry-run .`
- `wal-stats` — show the state of the WAL journals (read only). Example: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` — remove orphan terminal WAL journals older than a threshold, and also remove a `Started` journal whose owning PID is gone, reported as `dead_owner_removed`. Example: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `verify` — check a file against an expected BLAKE3 hash (exit 0 on match, 81 on mismatch). Example: `atomwrite --workspace "$WS" verify src/a.rs "$(atomwrite --workspace "$WS" hash src/a.rs | jaq -r .checksum)"`
- `set` — write a value at a dotted path in a TOML or JSON file. Example: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` — read a value at a dotted path in a TOML or JSON file. Example: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` — remove a key from a TOML or JSON file. Example: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` — rename identifiers across files and directories between case styles, and a directory is expanded through the walker. Example: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` — walk the tree-sitter AST and emit nodes. Example: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` — emit the high-level structure of a source file (a file, never a directory), in NDJSON or `--format tsv`. Example: `atomwrite --workspace "$WS" outline src/a.rs`
- `semantic-merge` — line-based three-way merge with conflict markers, not AST. Example: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `sparse` — budgeted list, read and outline for large trees. Example: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576`
- `recipe` — list and run built-in multi-step recipes. Example: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `stat` — metadata only, alias of `read --stat`. Example: `atomwrite --workspace "$WS" stat src/a.rs`
- `agent-surface` — tool manifest for agents, the SSOT of the subcommands. Example: `atomwrite agent-surface --format json`
- `watch` — stream filesystem events with debounce, ends with `watch_summary`, needs a build with `--features watch`. Example: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500` (the default build exits 78 with a rebuild hint, and that is the only exit measured here)
- `codemod` — apply a YAML list of AST rewrite rules across a tree. Example: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` — offline Jaccard ranking of files by query. Example: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `doctor` — host readiness diagnostics. Example: `atomwrite doctor`
- `scratch` — create the staging directory `.atomwrite/scratch` inside the workspace jail (the path is required). Example: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`
- `locale` — show, set or clear the UI locale. Example: `atomwrite locale`
- `commands` — print the full command tree as JSON, or short with `--brief`. Example: `atomwrite commands --include-globals`
- `config` — read and write persistent tuning keys (43 keys, scopes global and workspace). Example: `atomwrite --workspace "$WS" config get fuzzy.threshold`


## Claude Code
- Install: `cargo install atomwrite`
- Status: validated on 2026-10-19, the examples of this guide ran from the shell tool of Claude Code
- NDJSON output is parsed line by line without adapters
- Use `--workspace` to match the project root
- Pair with `--expect-checksum` for safe concurrent edits
- Add atomwrite commands to CLAUDE.md for automatic discovery
- Use `set/get/del/case/query/outline` for structured config and AST analysis
```bash
# Example CLAUDE.md entry
# Use atomwrite for all file operations
echo "content" | atomwrite --workspace . write src/file.rs
atomwrite --workspace . read src/file.rs
atomwrite --workspace . search 'pattern' src/
# walk a Rust file's AST
atomwrite --workspace . query src/main.rs --kinds
```


## Cursor
- Install: `cargo install atomwrite`
- Status: not validated
- Use `--dry-run` for preview before destructive operations
```bash
atomwrite --workspace . search 'TODO' src/ --include '*.rs'
atomwrite --workspace . replace --dry-run 'old_api' 'new_api' src/
```


## Windsurf
- Install: `cargo install atomwrite`
- Status: not validated
- Batch operations minimize the number of calls
```bash
atomwrite --workspace . batch < manifest.ndjson
```


## Codex CLI
- Install: `cargo install atomwrite`
- Status: not validated
- atomwrite stays inside the directory given by `--workspace`
- Checksums enable state verification across execution steps
- Use `case` to refactor identifiers across files
```bash
atomwrite --workspace . read src/main.rs
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . case src/main.rs --subvert user_id UserId --to pascal
```


## Aider
- Install: `cargo install atomwrite`
- Status: not validated
- Use `edit` for surgical changes instead of full file rewrites
- Use `outline` on a file to get a map of its structure before edits
```bash
atomwrite --workspace . edit src/lib.rs --old "old code" --new "updated code"
atomwrite --workspace . outline src/lib.rs  # see the structure first
```


## Continue
- Install: `cargo install atomwrite`
- Status: not validated
```bash
atomwrite --workspace . search 'deprecated' src/ --include '*.rs'
```


## Cline
- Install: `cargo install atomwrite`
- Status: not validated
- atomwrite writes atomically, which a shell redirect does not
```bash
echo "new content" | atomwrite --workspace . write src/config.rs
atomwrite --workspace . diff src/old.rs src/new.rs
```


## Roo Code
- Install: `cargo install atomwrite`
- Status: not validated
- Structured errors with `retryable` and `suggestion` fields guide automatic recovery
```bash
atomwrite --workspace . copy src/template.rs src/new.rs
```


## Amazon Q Developer
- Install: `cargo install atomwrite`
- Status: not validated
```bash
atomwrite --workspace . list src/ --depth 3
atomwrite --workspace . count src/ --by-extension
```


## GitHub Copilot CLI
- Install: `cargo install atomwrite`
- Status: not validated
- atomwrite commands are self-documenting via `--help` on each subcommand
- Use `get` to read package metadata without writing a parser
```bash
atomwrite calc "100 MB to bytes"
atomwrite regex "192.168.1.1" "10.0.0.1" "172.16.0.1"
atomwrite --workspace . get Cargo.toml package.version
```


## Custom Agents
- Install: `cargo install atomwrite`
- Status: validated on 2026-10-19 with the snippet below
- Invoke via `std::process::Command`, `subprocess.run()`, or equivalent
- Parse stdout line by line as JSON objects
- Check exit codes for error classification (the full table is in `agent-surface --format json`)
- Use the `retryable` field in error responses for automatic retry logic
```python
import subprocess
import json

result = subprocess.run(
    ["atomwrite", "--workspace", ".", "read", "src/main.rs"],
    capture_output=True, text=True
)
for line in result.stdout.strip().split("\n"):
    data = json.loads(line)
    print(data["type"], data.get("path"))
```
