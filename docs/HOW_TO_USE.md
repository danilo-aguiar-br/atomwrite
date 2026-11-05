# How to Use atomwrite


[Leia em Português](HOW_TO_USE.pt-BR.md)

- One CLI replaces dozens of file-manipulation tool calls your agent makes today


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


## Reading and Running Time
- Reading time is about 51 minutes for the whole guide, DERIVADO 11822 words outside the code fences at 230 words per minute
- First Command in 60 Seconds takes 2 minutes to read and run
- Every example ran on Linux against atomwrite 0.1.38 inside an isolated home directory
- A plain write finished in 8 ms on the measurement host, so examples answer at once
- The first AST call downloads tree-sitter parsers, which takes seconds (see Prerequisites)
- macOS and Windows were not measured for this guide
- Examples use a POSIX shell, and on PowerShell you pipe text with `'text' | atomwrite ...`
- When you are done, the [COOKBOOK](COOKBOOK.md) holds longer recipes


## What's New in v0.1.38 (2026-11-04)
- The failure envelope carries `ok`, `code` and `exit`, and `error: true` left it because it duplicated `ok: false` plus `code`, cost 13 bytes on every failure and no caller branched on it alone; a consumer that read it switches to `.ok == false`
- `read --format raw` stops at 24,000 bytes and prints one stderr line with the `--lines` that reads the rest, and `--all` removes the cap
- `read --format raw` on a single target honors `--max-filesize` and exits 65 `FILE_TOO_LARGE` instead of streaming a file above the ceiling
- `--symbol NAME`, `--line N --enclosing`, repeatable `--lines` and `--no-comments` choose the window by syntax tree
- `outline` reads a Markdown heading as a `section` that runs to the end of its own section, and `read --symbol Part` returns that whole section with its sub-sections
- A file whose image signature is damaged fails `read` with exit 65 `INVALID_INPUT` instead of returning its bytes as text
- A picture arrives as one `data:` URI, `--high-res` raises the tier to 2 576 pixels and 4 784 visual tokens, and a picture already sent in this session answers with a short token line unless `--force` is given
- `read doc.pdf` returns the text of every page, `--pages` selects windows, `needs_vision` names a page that carries no text, and `--ocr` recognizes characters on a picture or a rendered page in pure Rust
- `search --format text` writes one `path:line:col:text` per match, and a tree where every file was binary exits `89 ALL_SKIPPED_BINARY` instead of `NO_MATCHES`
- `apply` matches whole lines, applies several files in one invocation, publishes the changed lines with `--show`, and sorts the targets before it takes the locks
- The lock targets the inode with `std::fs::File::try_lock`, and `delete` takes the same lock before it hashes and unlinks, so a deletion and a write to one target no longer interleave
- `delete -r <dir>` owns the whole named tree, including hidden, ignored, linked, fifo, socket and device entries, and a link is never followed
- `target_resolved` is always an array in invocation order, because a path may contain the comma that used to join several targets
- `[defaults].backup = true` reaches `delete` and `batch delete`, and `[delete].backup` wins over it for `delete` only
- `output.profile = "lean"` drops the digests, the byte counts and the timing from a successful mutation record, which took one `apply` on a three-line file from 433 bytes to 196
- `config` lists 43 keys, and `atomwrite commands --brief` prints one line per subcommand in 5 020 bytes against 67 492 for the full tree
- Pin agents to `^0.1.38`. 47 subcommands (SSOT)


## What's New in v0.1.37 (2026-10-31)
- Pin agents to `^0.1.37` with `cargo install atomwrite --locked --version "^0.1.37"`
- The surface has 47 subcommands (SSOT), and the new one is `config`
- `config` reads and writes 37 persistent keys with `path|list|get|set|unset|doctor`
- `--scope global|workspace` is required on `config path`, `config set` and `config unset`
- Global stream flags `--sort-by`, `--dedupe`, `--aggregate` and `--diff-against` shape the data channel
- `--join KEY:SUFFIX` reads an NDJSON table from stdin once and attaches the matching rows to each data event
- Global flags `--redact`, `--normalize-encoding`, `--max-output-bytes` and `--allow-truncate` guard what leaves the process
- `--max-output-bytes` and `--allow-truncate` are rejected with exit 2 unless both are present
- `--print-schema [NAME]` lists the 55 schema names or emits one schema
- `--non-interactive` is accepted on every command, while `--confirm-side-effect` is accepted on the verbs that change bytes on disk and rejected with exit 65 on the rest
- `extract-artifact` reads PDF, OOXML and image headers and never emits raw artifact bytes
- Every success envelope carries `ok: true`, and a failing run ends in one terminal event with `ok: false`
- Every summary that publishes `files_failed` publishes `partial` beside it, and that covers `search`, `replace`, `transform`, `scope`, `count`, `list`, `sparse`, `semantic-search` and `case`
- `search`, `replace`, `transform`, `scope`, `case`, `sparse`, `semantic-search` and `backup` keep exit 0 when they answered and some file failed, while `count`, `list`, `hash`, `copy`, `delete` and `prune-backups` exit with the first failure because their answer is a total over the tree; the contract is ADR-0077 (`docs/decisions/0077-answer-family-and-walk-honesty.md`)
- `edit-loop` is all-or-nothing, and `edit-loop --partial` exits 87 (`EDIT_LOOP_PARTIAL`)
- A fuzzy match above `[fuzzy].max_content_bytes` (256 MiB) fails with exit 65, and `--fuzzy off` stays exact-only
- Exit codes to branch on are 65, 78, 83, 84, 87, 124, 129, 141 and 143
- The write lock is automatic on Unix and on Windows, and a timeout exits 83 (`LOCK_TIMEOUT`)
- `watch` left the default build, and the default binary answers it with exit 78

### Changes That Can Break a Consumer
- A summary that fails carries `ok`, `code` and `exit` itself, and no second error line follows it
- Read `partial` and `files_failed` in every summary that publishes them before trusting a result, and read `errors[]` as the first `summary.errors_max_entries` causes (50 by default)
- Cancelled scans end on a `cancelled` envelope with `deadline_exceeded`, so the last line may be a cancel record
- `--print-schema` must come before the command name, and an unknown name exits 65 with an error envelope
- `replace`, `scope`, `codemod`, `case`, `delete`, `backup` and `prune-backups` need explicit paths, or exit 2
- `wal-heal` and `scratch init` take a positional path, and `recipe run` needs `--path` or `--target`
- `transform` needs its positional paths, and a call without them is a usage error
- `edit-loop` with an unmatched pair exits 65 and writes nothing, where it used to write the matching pairs
- A missing pair reports `MATCH_FAILED` with `best_candidate`, where it used to report `INVALID_INPUT`
- `doctor` no longer reports its former CI and privilege fields, and `agent-surface` no longer has the tool-protocol key
- An unknown key or invalid file in `config.toml` or `.atomwrite.toml` exits 78
- The workspace `.atomwrite.toml` and the global `config.toml` are merged key by key
- `watch` needs a build with `--features watch`, or the call exits 78


## Prerequisites
- Rust toolchain 1.90 or later (edition 2024)
- Linux, macOS and Windows are supported
- Install from crates.io and confirm the version

```bash
cargo install atomwrite --locked --version "^0.1.38"
atomwrite --version
```

- Expect a line that starts with `atomwrite 0.1.37`
- The default build has the features `core`, `ast`, `lang-rust`, `lang-ts` and `lang-py`
- `watch` and `lang-go` are opt-in, and `--features full` turns on everything
- The default binary answers `watch` with exit 78 and the hint `cargo install atomwrite --features watch`
- Tree-sitter parsers are not compiled in, and they are downloaded the first time a language is used
- `outline`, `query`, `scope`, `transform`, `codemod`, `sparse outline` and `write --syntax-check` trigger the download
- The download is a 23,031,807 byte bundle for linux-x86_64 (DERIVADO: 23031807 / 1048576 = 21,97 MiB), and it unpacks into 3,991,360 bytes across 6 shared libraries (DERIVADO: 3991360 / 1048576 = 3,81 MiB), measured on a cold cache on 2026-10-31
- Parsers land in `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/`, and `~/.cache/...` is the Linux default
- The bundle comes from the `xberg-io/tree-sitter-language-pack` release `v1.22.1`, whose manifest lists 372 languages, and the HTTP client follows the system proxy
- For a host without network, fill the cache directory on a connected host first (see ADR-0057)
- `atomwrite doctor` reports the check `grammar_cache` with how many of the 23 grammars the AST verbs can need are already on disk, so the operator learns the condition before the run and not in the middle of it
- A grammar that is missing and cannot be downloaded is exit 69 `GRAMMAR_UNAVAILABLE` with class `precondition_failed`, never exit 255, because the host is offline and the tool is not broken
- `human-panic` writes a panic report as a local file in the temporary directory and sends nothing anywhere
- See [INSTALL.md](INSTALL.md) for Windows build tools and troubleshooting


## First Command in 60 Seconds
- Pick a working directory and store its absolute path once
- `--workspace` anchors the jail, so a wrong value moves the jail in silence

```bash
mkdir -p demo && cd demo
WS="$(pwd)"
printf 'hello world\n' | atomwrite --workspace "$WS" write src/hello.txt
```

- Expect one NDJSON line on stdout with `type` set to `write` and `ok` set to `true`
- The line carries `path`, `bytes_written`, `checksum`, `elapsed_ms`, `wal_policy` and `platform`
- It also carries `target_resolved` and `target_source`, which prove which path was written

```json
{"type":"write","status":"success","path":"src/hello.txt","bytes_written":12,"checksum":"dc5a4edb8240b018124052c330270696f96771a63b45250a5c17d3000e823355","ok":true,"target_source":"argv"}
```

- This is an excerpt, and the real line has more fields
- Read it back with metadata and checksum

```bash
atomwrite --workspace "$WS" read src/hello.txt
```

- Expect `type` `read` with `content`, `lines`, `bytes`, `checksum`, `permissions`, `modified` and `ok`
- Stdout is always NDJSON, stderr only carries diagnostics, and the exit code comes first
- Every write survives a crash, because it goes through tempfile, fsync and rename


## Core Commands
- Each command has a summary, its main flags, one tested example and the expected output
- Examples share one sample workspace, so run them in order inside `demo`

### Sample Workspace
- Create the files that the next examples use

```bash
mkdir -p src tmp
printf 'fn main() {\n    // TODO: wire config\n    let v = load().unwrap();\n    println!("{}", v);\n}\n\nfn load() -> Option<i32> {\n    Some(1)\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'fn alpha() {}\n' > src/a.rs
printf 'fn beta() {}\n' > src/b.rs
printf '[package]\nname = "demo"\nversion = "0.1.0"\n\n[database.pool]\nmax = 4\n' > Cargo.toml
```

### write
- Creates or overwrites a file atomically from stdin, in the order tempfile, fsync, rename, fsync-dir
- Your data reaches disk or the operation fails cleanly, and success carries a BLAKE3 `checksum`

```bash
printf 'fn main() {}\n' | atomwrite --workspace "$WS" write src/scratch.rs
printf 'longer content for the file\n' | atomwrite --workspace "$WS" write --backup --keep-backup src/hello.txt
printf 'checked write\n' | atomwrite --workspace "$WS" write --dry-run src/dry.txt
```

- Expect `type` `write` for the first two and `type` `plan` with `would_modify` for the dry run
- A backup is made by default and removed after success, and `--keep-backup` keeps the `.bak.<timestamp>` sibling
- A failed write always keeps its backup, `--no-backup` turns backups off and `--backup` with `--no-backup` exits 2
- `--expect-checksum HASH` writes only if the current checksum matches, or exits 82 (`STATE_DRIFT`)

```bash
CS="$(atomwrite --workspace "$WS" hash src/hello.txt | jaq -r .checksum)"
printf 'longer content, checked write\n' | atomwrite --workspace "$WS" write --expect-checksum "$CS" src/hello.txt
```

- `--append` and `--prepend` add to the file, and `--line-ending lf|crlf|cr|auto` normalizes endings
- `--durability full|fast|auto` picks the fsync strength, and the default `auto` uses full only for config files
- `--wal-policy auto|always|never` controls the journal sidecar, and `always` forces it even for trivial writes
- `--syntax-check` parses the result with tree-sitter and exits 88 (`SYNTAX_ERROR_DETECTED`) on error

```bash
printf 'fn ok() {}\n' | atomwrite --workspace "$WS" write --syntax-check src/ok.rs
printf 'tail line\n' | atomwrite --workspace "$WS" write --append src/hello.txt
```

- A rewrite that shrinks the file by more than 50% needs `--allow-shrink`, or exits 65
- An existing target above 100 KiB (`[write].confirm_large_bytes`) needs `--ack-overwrite`, or exits 65
- Empty stdin exits 65 unless you pass `--allow-empty-stdin`
- `--preserve-timestamps` keeps the old mtime, and by default mtime is updated so build tools rebuild
- Other flags are `--max-size`, `--require-backup`, `--auto-rotate`, `--risk-threshold PERCENT`, `--require-large-ack` and `--retention N`
- `--no-checksum-when-empty` lets an empty stdin skip `--expect-checksum`
- `--require-backup` and `--auto-rotate` take no value, and `--risk-threshold` is a percentage with default 255 (off)
- Success reports `platform.durability`, `platform.rename_method` and `platform.backup_method`
- A SIGTERM during a write removes the tempfile and exits 143

### read
- Reads a file with `content`, `lines`, `bytes`, `checksum`, `permissions` and `modified`
- The event also carries `target_resolved` and `target_source`, the same pair the writing verbs publish, so the agent confirms the path it read
- Binary files are detected, and their content is omitted

```bash
atomwrite --workspace "$WS" read --stat src/main.rs
atomwrite --workspace "$WS" read --lines 1:3 src/main.rs
atomwrite --workspace "$WS" read --head 2 src/main.rs
atomwrite --workspace "$WS" read --tail 2 src/main.rs
atomwrite --workspace "$WS" read --line 3 --context 1 src/main.rs
atomwrite --workspace "$WS" read --grep unwrap src/main.rs
atomwrite --workspace "$WS" read --format raw --head 2 src/main.rs
atomwrite --workspace "$WS" read --format raw src/main.rs src/cli.rs
atomwrite --workspace "$WS" read --format raw 'src/*.rs'
```

- Expect `type` `read` with `mode` `stat` or `full`, and `lines_total` when you read a slice
- `--format ndjson` is the default, and `--format raw` writes the bytes to stdout without an envelope
- `--bytes` is kept for old pipelines and means the same as `--format raw`
- The default answer stops at `read.max_output_bytes` on a complete line, aligned to the end of the last outline item, and publishes `truncated` plus a ready `next` range
- `--format raw` prints the next range on stderr and separates non-adjacent ranges with `--`, so a reader can tell where one range ends
- Several operands concatenate in argv order, and an operand that names an existing path is read as it is
- An operand with `*`, `?` or `[` that does not exist is expanded by the binary against one level of its directory, in byte order, skipping hidden names
- A missing file, a directory or a glob that matched nothing prints one line on stderr and the other targets still reach stdout, with the first failure's status
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` and `--verify-checksum` take a single target and exit 65 when several are named
- `**` is not supported, and only the final name may be a pattern: a wildcard in a directory name exits 65 instead of expanding halfway
- `--verify-checksum HASH` exits 81 (`CHECKSUM_VERIFY_FAILED`) with an error envelope on a mismatch
- `--grep REGEX` keeps only matching lines, and an invalid regex exits 65
- `--no-comments` needs `--number`, because dropping lines without keeping the file's own numbers would point the next `apply` at the wrong one, and the call exits 65 without it
- A picture is answered as one `data:` URI line on stdout, because that is the shape the vision channel accepts
- The type of a picture comes from the magic bytes, and the size follows the 28-pixel patch rule
- `--max-tokens N` shrinks the picture until the rounded patch count fits N, and `--crop X,Y,W,H` cuts the region before the resize
- `--high-res` moves the ceiling to 2576 pixels and 4784 visual tokens, and the standard tier stays at 1568 of each
- The EXIF orientation is applied, and a picture above 100 megapixels is refused before the decode
- A file whose magic bytes name a picture but whose header is broken is refused with exit 65 `INVALID_INPUT`, and the name stays in `target_resolved`
- HEIC and HEIF pictures are decoded like any other picture, and a still image is converted after the decode, so the vision channel receives a format it reads
- `--stat` on a picture reports width, height, format, visual tokens and GIF frame count without sending pixels
- A picture already delivered in this session is answered as the token line `[image delivered earlier in this session: <hash>]`, because a second copy of the same pixels costs the whole picture again
- `--force` sends the picture again inside the same session, and it is the only way to repeat a delivery that the session cache would skip
- `read doc.pdf` returns the text of every page, and `--pages N` or `--pages A:B` selects windows, repeatable in one call
- A page that carries no text is named in `needs_vision`, because a scanned page is not an empty document
- `--page N` renders one page and answers with the same `data:` URI a picture uses, so a scan never reaches the model as a header alone
- `--dpi N` sets the render resolution, and `--stat` reports the size of the rendering without sending it
- `--ocr` recognizes the characters on a picture or a rendered page and answers with the text in `content`, exactly where a text file puts it
- The recognition models are fetched once into the cache directory, so the first `--ocr` run needs a network and every later run does not
- A rendered page goes to recognition at 300 dpi by default, because small print at 150 dpi is too coarse to separate letters
- `--crop` sharpens recognition on a wide picture, because the engine reads a line better when the line is a larger share of the frame
- `--ocr` on a file that carries text exits 65 instead of pretending, because the flag reads pixels and that file has characters

### edit
- Edits a file by exact text, by line number or by marker, and the write is atomic
- A success line carries `checksum_before`, `checksum_after`, `lines_before`, `lines_after` and `mtime_preserved`

```bash
atomwrite --workspace "$WS" edit src/main.rs --old 'wire config' --new 'wired config'
atomwrite --workspace "$WS" edit src/a.rs --old 'alpha' --new 'alpha_one' --old '{}' --new '{ 1 }'
printf '    // inserted\n' | atomwrite --workspace "$WS" edit src/main.rs --after-line 1
printf 'one\ntwo\nthree\nfour\n' | atomwrite --workspace "$WS" write tmp/five.txt
printf 'TWO\n' | atomwrite --workspace "$WS" edit tmp/five.txt --range 2:2
atomwrite --workspace "$WS" edit tmp/five.txt --delete-range 1:1
```

- Expect `type` `edit`, and repeated `--old`/`--new` pairs add `pairs_total` and `pair_results`
- Position modes read the new text from stdin: `--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`
- A terminal on stdin makes those modes exit 65 at once instead of waiting
- `--delete-range N:M` needs no stdin, and `--multi` applies several NDJSON edits in one write
- `--multi` reads one JSON object per line from stdin, never a JSON array, and applies them all in a single atomic write
- Each object names its `op`: `exact`, `insert-after`, `insert-before`, `replace-range` or `delete-range`
- The content field follows the op: `content` for the three line-insert and range-replace ops, `old` and `new` for `exact`, and nothing for `delete-range`
- A field the chosen op does not read is exit 65 and nothing is written, so a stray `new` can never become a silent delete
- Line numbers refer to the ORIGINAL file, and two operations that touch the same line are refused before anything is applied

```bash
printf "%s\n" \
  '{"op":"replace-range","start":2,"end":2,"content":"TWO"}' \
  '{"op":"insert-after","line":4,"content":"five"}' \
  '{"op":"delete-range","start":6,"end":6}' \
  | atomwrite --workspace "$WS" edit tmp/lines.txt --multi
```

- The example replaces line 2, inserts after line 4 and deletes line 6, all against the same original numbering
- `--fuzzy auto|aggressive|off` handles near matches, and `off` means exact-only, with `auto` as the default
- `--fuzzy-threshold 0.0-1.0` tunes the cascade, and a result may carry `fuzzy`, `strategy` and `similarity`
- A miss exits 65 with `MATCH_FAILED`, `match_count` 0 and, when one exists, a `best_candidate`
- Several matches need `--replace-all`, and the success line then reports `match_count`
- A realigned indent adds `indent_adjusted: true`
- `--partial` applies the pairs that match and reports the rest, with exit 1 when no pair applied
- `--expect-checksum HASH` guards against drift, and `--allow-sequential-drift` accepts a chain of your own edits
- `--old-file` and `--new-file` read text from files that live inside the workspace (see Advanced Patterns)
- Never pipe `edit` into `jaq` without checking `${PIPESTATUS[0]}`, because the error envelope also goes to stdout

### search
- Searches file contents in parallel with the ripgrep engine
- Each file gets a `begin` event, its `match` events and an `end` event, then one `summary` closes the run

```bash
atomwrite --workspace "$WS" search 'TODO' src/
atomwrite --workspace "$WS" search --regex 'fn\s+\w+' src/
atomwrite --workspace "$WS" search --count 'fn' src/
atomwrite --workspace "$WS" search --files 'fn' src/
atomwrite --workspace "$WS" --filter type=match search -F 'println!' src/
atomwrite --workspace "$WS" search --target files lib src/
```

- Expect a `match` event with `path`, `line_number`, `lines`, `byte_offset` and `submatches`
- Exit 1 with `NO_MATCHES` means zero matches, and it is not a crash
- `--regex` (`-e`), `--fixed` (`-F`), `--word` (`-w`), `--case-insensitive` (`-i`) and `--smart-case` (`-S`) pick the pattern mode
- `--context N` (`-C`), `--max-count N` (`-m`), `--invert`, `--multiline` (`-U`) and `--pcre2` (`-P`) refine matching
- A pattern that starts with `-` goes after `--`, like `rg`: `search -F -- '-deprecated' src/`; without it the token is an unknown flag and the run exits 2
- `--count` (`-c`) and `--files` (`-l`) change the event shape, and `--target content|files|both` picks what is searched
- `--include` (`-g`) and `--exclude` filter by glob, and `--sort path|modified|created|none` orders files
- `--max-filesize BYTES` skips larger files (default 10 MiB, counted in `files_skipped`), and `--max-columns N` cuts long lines
- `--binary` also searches binary files, and `--include-fifo` opens named pipes, which the default skips
- `--no-begin-end` suppresses the begin and end events of a file with no match, which keeps an empty search clean
- `--offset N` and `--limit N` page through match events
- Read `partial` and `files_failed` on the summary before you trust a result, and a root named in the invocation that does not exist fails the run with exit 4 even when it matched, in every summary that publishes them

### replace
- Replaces text across files with atomic writes, and the path argument is mandatory
- Fixed strings use the fuzzy cascade after an exact miss, and `--regex` keeps the regex engine

```bash
atomwrite --workspace "$WS" replace --dry-run alpha_one omega src/
atomwrite --workspace "$WS" replace --preview alpha_one omega src/
atomwrite --workspace "$WS" replace alpha_one omega src/a.rs
atomwrite --workspace "$WS" replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run omega alpha_one src/
```

- Expect `type` `plan` per file on a dry run, `type` `preview` with a diff, and `type` `replaced` on a write
- Every run ends in a `summary` with `files_modified` and `total_replacements`
- Zero matches exit 1 with `NO_MATCHES`, and a trailing summary carries `ok: false`
- `--dry-run` and `--preview` show the plan, so use them before any bulk change
- `--literal` (`-F`) turns regex off, `--word` (`-w`) matches whole words and `--preserve-case` adapts the case
- `--max-replacements N` (`-n`) bounds the fuzzy applies, with a default of 1 when omitted and a hard ceiling of 10000
- Fuzzy applies are one-pass from left to right, and inserted text is never scanned again
- When the new text contains the old text, a single apply is forced, so expanding a section is safe
- `--include`, `--exclude`, `--expect-checksum` and `--preserve-timestamps` work as in `edit`
- `--progress-every N` emits progress heartbeats on large trees, and `--no-progress` silences them
- A successful `replace` deletes its `.bak` like every other mutator, and `--keep-backup` is the only way to keep it


## Utility Commands

### hash
- Computes the BLAKE3 checksum of files or of stdin
- Read the field `checksum`, and never the old field `value`

```bash
atomwrite --workspace "$WS" hash src/main.rs
printf 'data\n' | atomwrite --workspace "$WS" hash --stdin
atomwrite --workspace "$WS" hash --recursive --exclude '*.bak*' src/
H="$(atomwrite --workspace "$WS" hash src/lib.rs | jaq -r .checksum)"
atomwrite --workspace "$WS" hash --verify "$H" src/lib.rs
```

- Expect `type` `hash` with `algorithm` `blake3`, `checksum`, `bytes` and `ok`
- `--verify HASH` answers with `type` `hash_verify` and `verified`, and exits 81 on a mismatch

### delete
- Deletes files or trees, keeps no copy unless `--backup` is given, which creates a `.bak.<timestamp>`, and can print only the plan
- Deleting never asks for confirmation, so use `--plan` first

```bash
printf 'x' > tmp/old.txt
atomwrite --workspace "$WS" delete --plan tmp/old.txt
atomwrite --workspace "$WS" delete tmp/old.txt
mkdir -p tmp/d && printf '1\n' > tmp/d/f
atomwrite --workspace "$WS" delete --recursive tmp/d
```

- Expect `type` `plan` with `would_modify`, then `type` `deleted` with `checksum_before`, then a `summary`
- A directory without `--recursive` (`-r`) exits 65 and names the missing flag
- `delete -r <dir>` without a narrowing filter owns the whole named tree and removes hidden, ignored, linked, fifo, socket and device entries, and a link is never followed
- A run narrowed by `--include`, `--exclude`, `--older-than` or `--limit` exits 0 with `dirs_left` and `entries_left`, and an unfiltered run that leaves a directory exits with the measured I/O code and the surviving path in `errors[]`
- `--hidden`, `--no-gitignore` and `--follow-symlinks` are no-ops on `delete` and only emit one diagnostic line
- `--dry-run` works like `--plan`, and `--older-than 1d` filters by age with the suffixes s, m, h, d and w
- `--include` (`-g`) and `--exclude` filter by glob, and `--no-backup` is accepted and turns off the copy that `[defaults].backup = true` would request, because a delete copies only when the call asks
- `delete --confirm` and `delete --yes` (`-y`) are rejected with exit 65, so run again without them
- `--backup` with a directory root is refused before any change, because the copy would be created inside the tree, so run `copy -r` first or pass `--no-backup`
- A delete copy is never auto-removed, so clear it with `prune-backups`; `--keep-backup` only adds a warning and `--backup` is what creates the copy

### count
- Counts lines, blank lines and bytes, by extension or by size

```bash
atomwrite --workspace "$WS" count src/
atomwrite --workspace "$WS" count --by-extension src/
atomwrite --workspace "$WS" count --by-size --top 3 src/
```

- Expect `type` `count` with `mode` `lines`, `by_extension` or `by_size`
- `--include` (`-g`) and `--exclude` filter the walk

### diff
- Compares two files, with unified, stat or change-by-change output
- A path of `-` reads that side from stdin

```bash
atomwrite --workspace "$WS" diff src/a.rs src/b.rs
atomwrite --workspace "$WS" diff --stat src/a.rs src/b.rs
atomwrite --workspace "$WS" diff --unified --context 5 src/a.rs src/b.rs
atomwrite --workspace "$WS" diff --algorithm patience src/a.rs src/b.rs
```

- Expect `change` events and a closing `summary` with `identical` and `similarity_ratio`
- `--stat` emits `type` `diff` with `insertions` and `deletions`, and `--unified` emits the patch in `content`
- `--algorithm` takes `myers`, `patience` or `lcs`

### move
- Moves or renames a file or directory atomically
- Across devices it falls back to copy and delete, and reports `cross_device`

```bash
atomwrite --workspace "$WS" copy src/lib.rs src/lib_copy.rs
atomwrite --workspace "$WS" move src/lib_copy.rs src/lib_moved.rs
```

- Expect `type` `moved` with `source`, `target`, `checksum`, `cross_device` and `atomic`
- An existing target needs `--force` (`-f`) or `--backup`, or the call exits 65
- `--preserve-hardlinks` keeps hard links, and `--dry-run` shows the plan

### copy
- Copies files or trees and verifies the BLAKE3 checksum after the write

```bash
atomwrite --workspace "$WS" copy --force src/lib.rs src/lib_copy.rs
atomwrite --workspace "$WS" copy --recursive --preserve src/ tmp/srccopy
```

- Expect `type` `copied` with `checksum` and `verified` set to `true`
- `--recursive` (`-r`) copies trees, `--preserve` keeps metadata and `--preserve-xattr` keeps extended attributes
- `--no-reflink` forces a byte copy, and reflink is the default where the filesystem supports it
- Backups use `reflink_or_copy` and never a hard link of the live file

### list
- Lists a directory tree with one NDJSON event per entry

```bash
atomwrite --workspace "$WS" list --long --depth 1 src/
atomwrite --workspace "$WS" list --count-by-ext src/
atomwrite --workspace "$WS" list --include '*.rs' --depth 1 src/
```

- Expect `type` `entry` with `path` and `kind`, plus `size` and `modified` with `--long`
- The closing `summary` has `files`, `dirs`, `symlinks` and `total_bytes`
- `--all` includes hidden entries, and `--exclude` filters by glob

### extract
- Picks fields from NDJSON read on stdin, or columns from plain text
- Use positional keys or repeatable `--field`, and never both in one call
- `--select` is a global flag, so `extract --select` is not valid

```bash
printf 'a b c\n' | atomwrite extract 0 2
printf '{"a":1,"b":2}\n' | atomwrite extract --field a
atomwrite --workspace "$WS" search 'TODO' src/ | atomwrite extract --field path --field line_number
printf '{"a":1,"b":2}\n' | atomwrite --select a extract --field a
```

- Expect `{"type":"text","values":["a","c"]}` for text columns and `{"a":1}` for the NDJSON case
- `--delimiter` (`-d`) sets the column separator, and `--stdin` reads the input stream

### extract-artifact
- Extracts bounded signal from PDF, OOXML (documents, sheets, slides) and image headers
- It never returns raw artifact bytes

```bash
printf '\211PNG\r\n\032\n\000\000\000\rIHDR\000\000\000\001\000\000\000\001\010\006\000\000\000\037\025\304\211' > img.png
atomwrite --workspace "$WS" extract-artifact img.png
```

- Expect `type` `artifact` with `kind` `image`, `format` `png`, `width` and `height`
- `--max-sample-rows` and `--max-text-chars` override the `extract.*` config keys

### calc
- Evaluates math and unit conversions with the fend engine

```bash
atomwrite calc "2 hours + 30 minutes to seconds"
atomwrite calc "sqrt(144) + 3^2"
atomwrite calc "10 GiB to MB"
```

- Expect `type` `calc` with `expression` and `result`, such as `9000 seconds`
- Quote the expression, because the shell would split it, and `--stdin` reads it from input

### regex
- Generates a regular expression from example strings, with the grex engine

```bash
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"
atomwrite regex abc-12 xyz-99 id-7 --digits
atomwrite regex --digits --words "user_123" "admin_456"
```

- Expect `type` `regex` with `regex`, `examples` and `anchored`
- `--digits` (`-d`), `--words` (`-w`), `--spaces` (`-s`), `--repetitions` (`-r`) and `--case-insensitive` (`-i`) widen the pattern
- `--no-anchors` drops `^` and `$`, and `--stdin` reads examples from input


## Advanced Commands

### scope
- Selects code by grammar and acts on it, with `--delete`, `--replace-with` or `--action`
- It needs `--language` (`-l`, alias `--lang`) and exactly one of `--query` or `--pattern`

```bash
atomwrite --workspace "$WS" scope src/main.rs --language rust --query comments --delete --dry-run
atomwrite --workspace "$WS" scope --query fn --action upper -l rust --dry-run src/lib.rs
atomwrite --workspace "$WS" scope --pattern 'a + b' --action upper -l rust --dry-run src/lib.rs
```

- Expect `type` `scoped` with `scopes_matched`, `bytes_before` and `bytes_after`, then a `summary`
- The summary reports `total_matches` as the sum of `scopes_matched` across the run, beside `files_visited` and `files_matched`
- `files_matched` counts the files that matched something, and a file that failed or was skipped never counts
- `--action` takes `upper`, `lower`, `titlecase`, `squeeze`, `symbols` and `normalize`
- Queries include `comments`, `strings`, `fn`, `pub-fn`, `async-fn`, `struct`, `enum`, `trait`, `impl`, `mod` and `use`
- Always run `--dry-run` before `--delete`, and remove the flag to apply

### transform
- Searches and rewrites code by AST pattern with the ast-grep engine
- `$NAME` captures one node, `$$$ARGS` captures several, and `-p`/`--pattern` and `-r`/`--rewrite` are used together

```bash
atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/
atomwrite --workspace "$WS" transform --dry-run -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/
```

- Expect `type` `transformed` with `matches` and `replacements`, then a `summary`
- Exit 1 means zero matches, and the paths are required
- `--rules FILE` and `--inline-rules YAML` apply a list of rules, in the format shown under `codemod`, and the single summary adds the failures of every rule in `files_failed`, `partial` and `errors[]` beside the counters
- A file that fails counts once in `files_failed` and once in `errors[]`, whatever the number of rules, because each rule walks the whole tree
- `files_visited`, `files_matched` and `total_replacements` still sum per rule, because every rule is a full pass
- `--verify-parse` checks that the result still parses, and `--include` and `--exclude` filter files

### batch
- Runs operations from an NDJSON manifest, with `write`, `replace`, `delete`, `edit`, `hash`, `move` and `copy`
- `--transaction` makes it all-or-nothing, and a failure rolls back

```bash
printf '%s\n' '{"op":"write","target":"batch.txt","content":"x"}' '{"op":"hash","target":"batch.txt"}' | atomwrite --workspace "$WS" batch --transaction
printf '%s\n' '{"op":"write","target":"batch.txt","content":"y"}' | atomwrite --workspace "$WS" batch --dry-run
```

- Expect one `batch_op` per line and a closing `summary` with `succeeded`, `failed` and `committed`
- `--file PATH` reads the manifest from a file, and the manifest is read one line at a time
- `move` and `copy` operations take `"force":true` to overwrite
- `--input-schema` prints the JSON Schema of the manifest, and `--batch-size N` sets how many operations buffer before the summary

### apply
- Applies a patch from stdin: unified diff, SEARCH/REPLACE blocks, markdown-fenced or a full file
- Detection is automatic, and `--format auto|unified|search-replace|full|markdown` forces one
- A body with no marker at all is refused with exit 65, and only `--format full` replaces the whole file, so a malformed patch can no longer become a silent overwrite
- `--format full` walks the same two barriers as `write`: `--ack-overwrite` above `[write].confirm_large_bytes` and `--allow-shrink` below `[write].shrink_block_percent`
- `--strict-atomic` exits 91 instead of falling back to a cross-device copy, and `--no-syntax-check` skips the parse check that refuses a patch which breaks the file
- A `---`/`+++` header with no `@@` hunk is refused with exit 65, because applying it as a full file would overwrite the target with the patch text

```bash
printf 'new body\n' | atomwrite --workspace "$WS" apply --format full tmp/applied.txt
printf -- '--- a/src/b.rs\n+++ b/src/b.rs\n@@ -1 +1 @@\n-fn beta() {}\n+fn beta_v2() {}\n' > tmp/fix.patch
atomwrite --workspace "$WS" apply --dry-run src/b.rs < tmp/fix.patch
atomwrite --workspace "$WS" apply src/b.rs < tmp/fix.patch
```

- Expect `type` `plan` on the dry run and `type` `applied` with `format_detected`, `hunks_applied` and `backup_path` when the run made a copy
- A unified diff must match context and removed lines within 64 lines, or it exits 65 with `MATCH_FAILED`
- A SEARCH/REPLACE block that is missing fails the whole batch, and nothing is written
- The target argument is optional: a patch that names its own files is applied in one call, and every file is checked before the first write
- A SEARCH/REPLACE patch names a file with a bare path line just above that file's first block, and a unified patch uses its own header pair for the same job
- The refusal of a file names it in `target_resolved`, and every applied file gets its own record with its own path and checksums
- Two sections for the same file are refused, and a write that fails after another file was written restores the files already written and publishes one `rollback` record with the count

### set
- Writes a value at a dotted path in a TOML or JSON file and keeps comments and key order
- The type is inferred from the value, so `9` becomes an integer and `true` a boolean

```bash
atomwrite --workspace "$WS" set Cargo.toml package.version 0.2.0
atomwrite --workspace "$WS" set Cargo.toml database.pool.max 9
printf '{"scripts":{"build":"tsc"}}\n' > package.json
atomwrite --workspace "$WS" set package.json scripts.build "tsc -b"
```

- Expect `type` `set` with `key_path`, `old_value`, `new_value`, `format` and `comments_preserved`
- `set` accepts the backup flags and `--preserve-timestamps`, and it needs no type flag
- Use `config set` for the tuning keys of atomwrite itself

### get
- Reads a value at a dotted path in a TOML or JSON file

```bash
atomwrite --workspace "$WS" get Cargo.toml package.version
atomwrite --workspace "$WS" get package.json scripts.build
```

- Expect `type` `get` with `value`, `found` and `format`
- A missing key exits 65 with `INVALID_INPUT`, and it does not return `found: false`
- A file that is not valid TOML or JSON also exits 65

### del
- Removes a key at a dotted path in a TOML or JSON file

```bash
atomwrite --workspace "$WS" del Cargo.toml database.pool.max
atomwrite --workspace "$WS" del --force-missing Cargo.toml package.metadata.deprecated
```

- Expect `type` `del` with `removed_value` and `existed`
- `--force-missing` turns a missing key into a success with `existed` set to `false`

### case
- Renames an identifier across source files and converts the new name to a case style
- `--to` takes `snake` (default), `camel`, `pascal`, `kebab` and `screaming-snake`
- Pass files, because a directory exits 0 and changes nothing (`identifiers_total` is 0)

```bash
printf 'fn user_id() {}\nlet userId = 1;\n' > src/c.rs
atomwrite --workspace "$WS" case --subvert user_id account_id --to camel --dry-run src/c.rs
atomwrite --workspace "$WS" case --subvert user_id account_id --to camel src/c.rs
```

- Expect `type` `case_preview` on the dry run and `type` `case` with `before` and `after` on a write
- A closing `summary` reports `identifiers_total` and `files_modified`
- Name several files to rename across a module, and run `--dry-run` first on a large tree

### query
- Walks the tree-sitter AST of a file and emits nodes as NDJSON

```bash
atomwrite --workspace "$WS" query --kinds src/lib.rs
atomwrite --workspace "$WS" query --query function_item --positions src/lib.rs
atomwrite --workspace "$WS" query --tree src/lib.rs
```

- Expect `query_kind`, `query_match` or `query_summary` events with `kind`, `text` and byte offsets
- `--query KIND` (`-Q`) filters by node kind, `--kinds` lists kinds and `--tree` lists every node
- `--positions` adds line and column, and `--language` sets the language when the extension is unclear
- An invalid query exits 65, and a slim build without `ast` exits 78

### outline
- Emits the high-level structure of a source file: functions, structs, enums, traits and modules
- It needs a file, and a directory is not accepted

```bash
atomwrite --workspace "$WS" outline src/lib.rs
atomwrite --workspace "$WS" outline --kind function_item --positions src/lib.rs
```

- Expect `outline_item` events with `kind`, `name`, `signature`, `start_line` and `end_line`, then an `outline_summary`
- `--kind` is repeatable, and `--positions` adds bytes and columns
- A slim build without `ast` exits 78 with a rebuild hint


## Safety and Maintenance Commands

### backup
- Creates a timestamped `.bak.<timestamp>` copy of a file with a BLAKE3 checksum

```bash
atomwrite --workspace "$WS" backup src/lib.rs --retention 3
atomwrite --workspace "$WS" backup --dry-run src/lib.rs
```

- Expect `type` `backup` with `backup_path` and `checksum`, then a `summary` with `files_backed_up`
- `--retention N` keeps the N newest backups, `--output-dir DIR` writes elsewhere and `--dry-run` previews

### rollback
- Restores a file from a previous backup, and `--verify` checks the BLAKE3 checksum first

```bash
atomwrite --workspace "$WS" rollback src/lib.rs --latest --verify --dry-run
atomwrite --workspace "$WS" rollback src/lib.rs --latest --verify
```

- Expect `type` `plan` on the dry run and `type` `rollback` with `restored_from` and `verified`
- `--timestamp STAMP` restores a specific backup, and a prefix of the stamp is accepted
- The pre-rollback safety snapshot is opt-in through `--backup`

### verify
- Checks a file against an expected BLAKE3 hash, with exit 0 on a match and 81 on a mismatch
- The hash is positional, so the call is `verify PATH HASH`

```bash
H="$(atomwrite --workspace "$WS" hash src/lib.rs | jaq -r .checksum)"
atomwrite --workspace "$WS" verify src/lib.rs "$H"
```

- Expect `type` `hash_verify` with `verified` set to `true` and `ok` set to `true`
- On a mismatch the same event carries `verified: false`, `code` `CHECKSUM_VERIFY_FAILED` and `exit` 81

### edit-loop
- Applies N old/new pairs to one file in a single write, all or nothing
- Send a JSON array or one JSON object per line on stdin

```bash
printf 'alpha\nbeta\ngamma\n' > tmp/loop.txt
printf '[{"old":"alpha","new":"ALPHA"},{"old":"beta","new":"BETA"}]\n' | atomwrite --workspace "$WS" edit-loop tmp/loop.txt
printf '{"old":"gamma","new":"GAMMA"}\n{"old":"absent","new":"X"}\n' | atomwrite --workspace "$WS" edit-loop --partial tmp/loop.txt
```

- Expect `type` `result` with `pairs_total`, `pairs_applied`, `pairs_unmatched` and `pair_results`
- Without `--partial`, an unmatched pair exits 65 with `MATCH_FAILED` and `failed_pair_index`, and nothing is written
- With `--partial`, matching pairs are written and the run exits 87 (`EDIT_LOOP_PARTIAL`) when any pair missed
- `--syntax-check`, `--line-ending` and `--allow-sequential-drift` are also accepted

### prune-backups
- Removes `.bak` siblings of the named target files by age or by count
- Name the target files, not a directory, and pass `--max-age-secs` or `--max-count`
- `--dry-run` is a flag without a value, so omit it to apply

```bash
atomwrite --workspace "$WS" prune-backups --max-age-secs 0 --dry-run src/hello.txt
atomwrite --workspace "$WS" prune-backups --max-age-secs 0 src/hello.txt
atomwrite --workspace "$WS" prune-backups --max-count 3 src/lib.rs
```

- Expect `pruned` events with `reason` `age` or `count`, then a `summary` whose `action` is `dry_run` or `pruned`
- Only names carrying a real backup stamp (`.bak.YYYYMMDD_HHMMSS` or `.bak.YYYYMMDD_HHMMSS_mmm`) are candidates, so `f.txt.bak.0-user-note` is never touched
- `--dry-run false` does not apply anything, because `false` is read as a path

### wal-stats
- Shows the state of WAL journals under the workspace, read only

```bash
atomwrite --workspace "$WS" wal-stats
```

- Expect `type` `wal_stats` with `total_journals`, `by_state`, `oldest_journal_age_secs` and `auto_heal_recommended`

### wal-heal
- Removes terminal journals older than a threshold, and keeps journals that are still `Started`
- It needs a path, and `--threshold-secs` defaults to 3600

```bash
atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .
atomwrite --workspace "$WS" wal-heal --max-duration-ms 200 .
```

- Expect `type` `wal_heal` with `removed`, `preserved`, `malformed` and `bytes_reclaimed`
- `--max-duration-ms` bounds the wall-clock budget, and `--dry-run` previews
- A crash (`kill -9`) can leave a journal in the `Started` state, and `wal-heal` preserves it for you to inspect
- The global `--no-auto-heal` skips the startup heal pass, which runs only on commands that write
- A cancelled run (SIGTERM, SIGINT or a deadline) leaves no journal behind


## Orchestration Commands

### recipe
- Lists and runs built-in multi-step recipes: `search-replace-verify` and `edit-loop-syntax-check`
- `recipe run` needs `--path` or `--target`, and recipe steps skip `*.bak.*` files

```bash
atomwrite --workspace "$WS" recipe list
atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern beta --replacement omega --path src --dry-run
```

- Expect `recipe` events for the list and a final `recipe_result` with `steps`, `dry_run` and `ok`
- A recipe whose search finds nothing stops at the replace step with exit 1 and `failed_step_id`
- `edit-loop-syntax-check` needs `--pairs-file FILE`, `--target FILE` and `--syntax-check LANGUAGE`
- `--fuzzy`, `--fuzzy-threshold`, `--include` and `--exclude` tune the search-replace recipe

### sparse
- Lists, reads and outlines large trees under a file and byte budget

```bash
printf 'src/lib.rs\nsrc/b.rs\n' > tmp/paths.txt
atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576 --all
atomwrite --workspace "$WS" sparse read --paths-file tmp/paths.txt --head 2
atomwrite --workspace "$WS" sparse outline --max-files 5 --all src/
```

- Expect `sparse_entry`, `sparse_read` or `outline_item` events and a `sparse_summary` with `truncated`
- `sparse list` also takes `--include` and `--exclude` globs, and an invalid glob exits 65
- `--all` adds the dotfile to the visit, so `files_scanned` grows by the hidden entries and the default run keeps them out
- `sparse outline` reports `sparse_outline_budget`, and a slim build without `ast` exits 78

### semantic-merge
- Merges three files line by line (base, ours, theirs), and it is not AST-aware
- Conflict markers are written by default, `--write-conflict-markers true|false` states it explicitly, and `--no-conflict-markers` turns them off

```bash
printf 'a\nb\nc\n' > tmp/base.txt
printf 'a\nB\nc\n' > tmp/ours.txt
printf 'a\nb\nC\n' > tmp/theirs.txt
atomwrite --workspace "$WS" semantic-merge --base tmp/base.txt --ours tmp/ours.txt --theirs tmp/theirs.txt --output tmp/merged.txt --fail-on-conflict
```

- Expect `type` `semantic_merge` with `status` `merged` and the three input checksums
- A real conflict has `status` `conflict`, and `--fail-on-conflict` turns it into exit 84 (`MERGE_CONFLICT`)
- `--expect-checksum` and the backup flags guard the output file

### semantic-search
- Ranks files and lines by offline Jaccard similarity to a query, and it is always compiled in

```bash
atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01
atomwrite --workspace "$WS" semantic-search "fn" src/ --k 2 --index-dir .atomwrite/semantic-index
```

- Expect `semantic_match` events with `rank`, `score`, `path`, `line` and `snippet`, then a `semantic_summary`
- `backend` is `jaccard`, or `inverted-index` when `--index-dir` is used
- `--include-backups` also scans `.bak` files, and zero hits is not a crash

### codemod
- Applies a YAML list of AST rewrite rules across a tree
- Each rule is an item with `id`, `language`, `pattern` and `rewrite`

```bash
printf -- '- id: unwrap-to-try\n  language: rust\n  pattern: "$E.unwrap()"\n  rewrite: "$E?"\n' > rules.yaml
atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/
```

- Expect `transformed` events and a `codemod_summary` with `by_rule_id`
- A rules file in ast-grep style with `rule:` and `fix:` exits 65
- `transform --rules` and `--inline-rules` read the same format

### watch
- Streams filesystem events with debounce and always ends with a `watch_summary`
- It needs a build with `--features watch`, and the default build exits 78 with a rebuild hint

```bash
atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500
```

- On the default build, expect `code` `CONFIG_INVALID` and exit 78
- On a `watch` build, expect `watch_summary` as the last line, even on a signal
- Always pass `--max-events` or `--idle-exit-ms` (default idle exit is 500 ms), or `--timeout-secs`
- `--debounce-ms MS`, `--checksum` and `--gitignore true|false` tune the stream

### stat
- Returns metadata only, as an alias of `read --stat`

```bash
atomwrite --workspace "$WS" stat src/lib.rs
```

- Expect `type` `read` with `mode` `stat`, plus `lines`, `bytes` and `checksum`


## Discovery and Host Commands
- Use these before you invent flag lists by hand

### doctor
- Diagnoses the host and reports checks for heal, concurrency, features and the environment
- It reads no environment variable, and `--strict` exits 1 (`DOCTOR_CHECKS_FAILED`) when a check fails

```bash
atomwrite --workspace "$WS" doctor --strict
```

- Expect `type` `doctor_report` with `ok` and a `checks` array
- On the default build a check reports that `watch` is disabled

### commands
- Prints the full command tree as JSON, with or without the global arguments

```bash
atomwrite --workspace "$WS" commands --include-globals
```

- Expect `type` `commands_tree` with 47 `commands` and, with the flag, 33 `global_args`
- Aliases such as `--max-concurrency`, `--timeout`, `--fields` and the hidden `--json` are not listed

### agent-surface
- Prints the tool manifest for agents, and it is the single source of truth for the subcommands

```bash
atomwrite --workspace "$WS" agent-surface --format json
```

- Expect `type` `agent_surface` with `tools`, `exit_codes`, `discovery`, `flag_contract` and `notes`
- `tools` has 47 entries, and `exit_codes` has 42 entries with 34 distinct numbers
- `discovery` lists the `--print-schema` names, and `watch` reports `available: false` on the default build

### locale
- Shows, persists or clears the display language, and it is distinct from the global `--locale`
- Precedence is `--locale`, then the saved preference, then the system language, then `en`

```bash
atomwrite locale
atomwrite locale --set pt-BR
atomwrite locale --clear
```

- Expect `type` `locale_report` with `resolved`, `source`, `available` and `preference_path`
- Only `en` and `pt-BR` exist, and any other tag exits 2 with the list of available tags
- The preference lives in `$XDG_CONFIG_HOME/atomwrite/locale`
- Error `code` and `message` stay in English, and `suggestion` follows the locale, with `suggestion_lang` saying which

### completions
- Prints a shell completion script, or installs it under the XDG data directory
- Supported shells are bash, zsh, fish, elvish and powershell

```bash
atomwrite completions bash > atomwrite.bash
atomwrite completions bash --install
```

- Expect the raw script on stdout for the first call and an `installed` event for the second
- That event is `{"type":"installed","path":"<completion file>","ok":true,"target_resolved":["<same path>"],"target_source":"argv"}`
- The path comes from the XDG data directory of the shell, as listed in Where Files Live
- A write that fails because the reader closed exits 141 (`BROKEN_PIPE`) with no panic and no crash report
- Very short output, such as `--version | head -1`, can finish before the reader closes and then exits 0

### scratch
- Creates the staging directory under the workspace jail, and never under the OS temporary directory
- `scratch init` needs a path

```bash
atomwrite --workspace "$WS" scratch init .atomwrite/scratch
```

- Expect `type` `scratch_init` with `path`, `relative` and `already_existed`
- Stage `--old-file` and `--new-file` content there, because paths outside the jail exit 126

### config
- Reads and writes persistent tuning keys in the global `config.toml` or the workspace `.atomwrite.toml`
- `list` and `get` need no scope, and `set`, `unset` and `path` need `--scope global|workspace`

```bash
atomwrite --workspace "$WS" config get fuzzy.threshold
atomwrite --workspace "$WS" config list
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace
atomwrite --workspace "$WS" config path --scope workspace
atomwrite --workspace "$WS" config unset fuzzy.threshold --scope workspace
```

- Expect `type` `config` with `action`, `key`, `value`, `origin` and `default`
- `origin` is `default`, `global`, `workspace`, or `explicit` when `--config PATH` is used
- The 43 keys, their ranges and scopes are in the Configuration section


## 47 Subcommands (SSOT)
- Pin agents to `^0.1.38`, and read `atomwrite agent-surface` for the live manifest
- `CANONICAL_SUBCOMMAND_COUNT` is 47, and the machine tree is `atomwrite commands --include-globals`

| Command | Summary |
|---|---|
| `read` | Reads a file with checksum, size and lines, or only a slice |
| `write` | Creates or overwrites a file atomically from stdin |
| `edit` | Edits by exact or fuzzy text, line number or marker |
| `delete` | Deletes files or trees, owning the whole named tree on `-r`, copying one only when the call asks (`--backup` or `[defaults].backup = true`), or prints the plan |
| `copy` | Copies a file or tree with checksum verification |
| `move` | Moves or renames a file atomically |
| `apply` | Applies a unified diff, SEARCH/REPLACE or full file from stdin |
| `search` | Searches file contents in parallel with the ripgrep engine |
| `replace` | Replaces text across files with atomic writes and fuzzy matching |
| `transform` | Rewrites code by AST pattern with ast-grep |
| `hash` | Prints the BLAKE3 checksum of files or stdin |
| `count` | Counts lines and files by extension or by size |
| `diff` | Compares two files as unified, stat or changes |
| `list` | Lists a directory tree with metadata |
| `extract` | Picks fields from NDJSON on stdin |
| `extract-artifact` | Extracts bounded signal from PDF, OOXML and image headers |
| `scope` | Acts on comments, strings, functions and other code categories |
| `regex` | Generates a regex from example strings |
| `calc` | Evaluates math and unit conversions |
| `completions` | Prints or installs shell completions |
| `backup` | Creates timestamped backups with BLAKE3 checksums |
| `rollback` | Restores a file from a previous backup |
| `batch` | Runs NDJSON operations from stdin, optionally as one transaction |
| `edit-loop` | Applies N old/new pairs in one write, all or nothing |
| `prune-backups` | Removes old `.bak` files by age or count |
| `wal-stats` | Shows the state of WAL journals, read only |
| `wal-heal` | Removes stale terminal WAL journals |
| `verify` | Checks a file against an expected BLAKE3 hash |
| `set` | Writes a value at a dotted path in TOML or JSON |
| `get` | Reads a value at a dotted path in TOML or JSON |
| `del` | Removes a key from a TOML or JSON file |
| `case` | Renames identifiers between case styles in files |
| `query` | Walks the tree-sitter AST and emits nodes |
| `outline` | Emits the structure of a source file |
| `semantic-merge` | Three-way line-based merge with conflict markers |
| `sparse` | Budgeted list, read and outline for large trees |
| `recipe` | Lists and runs built-in multi-step recipes |
| `stat` | Metadata only, alias of `read --stat` |
| `agent-surface` | Tool manifest for agents |
| `watch` | Streams filesystem events, needs the `watch` build feature |
| `codemod` | Applies a YAML list of AST rewrite rules |
| `semantic-search` | Offline Jaccard ranking of files by query |
| `doctor` | Host readiness diagnostics |
| `scratch` | Creates the staging directory inside the jail |
| `locale` | Shows, sets or clears the display language |
| `commands` | Prints the full command tree as JSON |
| `config` | Reads and writes persistent tuning keys |


## Global Flags
- Global flags go before the subcommand, and `atomwrite commands --include-globals` lists all 33
- `--json` is accepted and ignored, because stdout is always NDJSON
- There is no global `-n`, because `replace` owns `-n`

| Flag | Default | Meaning |
|---|---|---|
| `-v`, `--verbose` | off | Raises tracing on stderr: `-v` info, `-vv` debug, `-vvv` trace |
| `-q`, `--quiet` | off | Lowers tracing: `-q` error, `-qq` off |
| `--workspace PATH` | current directory | Jail root, and every path must stay inside it |
| `--config PATH` | none | Explicit `.atomwrite.toml`, and it fails if missing or malformed |
| `--color auto\|always\|never` | `auto` | Controls colored output |
| `--no-color` | off | Same as `--color never` |
| `--ready-file PATH` | none | Test harness: writes the PID once signal handlers are ready |
| `--no-gitignore` | off | Does not respect `.gitignore` |
| `--hidden` | off | Includes hidden files and directories |
| `--follow-symlinks` | off | Follows symbolic links |
| `-j`, `--threads N` | automatic | Alias `--max-concurrency`, omitted or 0 means all cores up to 16, limited by RAM, and an explicit N is honored above 16 |
| `--max-filesize BYTES` | 1 GiB | Rejects larger files with exit 65, while `search` skips above its own 10 MiB limit and counts the skips in `files_skipped` |
| `--timeout-secs N` | 120 | Alias `--timeout`, 0 disables the deadline, and a deadline exits 124 |
| `--no-progress` | off | Disables NDJSON progress events on long operations |
| `--json-schema` | off | Emits the JSON Schema of the subcommand output and exits |
- `--print-schema [NAME]` | off | Lists the 55 schema names, or emits one, and it goes before the command |
| `--non-interactive` | off | Declares agent-hosted execution: never prompts, never reads confirmation from stdin |
| `--confirm-side-effect` | off | Acknowledges a side effect where the command contract asks for it |
| `--allow-truncate` | off | Opt-in to value truncation, required by `--max-output-bytes` |
| `--locale en\|pt-BR` | resolved | Overrides the display language of suggestions |
| `--no-auto-heal` | off | Skips the startup `wal-heal` pass |
| `--select KEYS` | all keys | Alias `--fields`, projects events to top-level keys (CSV) |
| `--filter EXPR` | none | Keeps events where `key=value`, `key==value` or `key!=value`, repeatable AND |
| `--limit N` | none | Work budget: cuts the target list before the action and stops the walk at the record that reaches the cap, so at most N answer records leave; and the terminal summary carries `walker_stopped_early: true` only when the walk was actually cut before the tree ended |
| `--sort-by KEY` | arrival order | Orders buffered events by a key, `path-line` or `line-path` |
| `--dedupe KEYS` | off | Keeps the first event of each group of equal key values |
| `--aggregate KEY[:OP[:FIELD]]` | off | Collapses events per group, with OP `count`, `sum`, `min` or `max` |
| `--join KEY:SUFFIX` | off | Reads an NDJSON table from stdin once (up to 16 MiB) and attaches the matching rows under SUFFIX |
| `--truncate-content KEY:LEN` | off | Cuts string values of KEY to LEN characters |
| `--redact` | off | Replaces secret-shaped values with `[redacted]` |
| `--normalize-encoding` | off | Normalizes every string value to Unicode NFC |
| `--diff-against PATH` | off | Replaces the stream with one `stream_diff` line of added, removed and unchanged counts |
| `--max-output-bytes BYTES` | off | Caps every string value, `checksum` included, and needs `--allow-truncate` |

- `--select`, `--filter` and `--limit` keep error envelopes, so a failure with `ok: false` still appears
- Ordering, dedupe, aggregate and stream diff buffer the whole run and apply at the end
- `--max-output-bytes` cuts every string value, so never use it when the full `checksum` or `content` matters
- `--diff-against` needs a baseline file inside the workspace, and `--print-schema NAME` or `--print-schema=NAME` both work
- A tail shape flag never hides the `cancelled` record of an interrupted run

```bash
atomwrite --workspace "$WS" --sort-by path-line --select type,path,line_number --filter type=match search 'fn' src/
atomwrite --workspace "$WS" --aggregate kind:count list --depth 1 src/
atomwrite --workspace "$WS" --max-output-bytes 12 --allow-truncate --select path,checksum hash src/lib.rs
atomwrite --workspace "$WS" --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 edit src/main.rs --old nothere --new x --fuzzy off
atomwrite --print-schema search
atomwrite --workspace "$WS" list --depth 1 src/ > tmp/base.ndjson
atomwrite --workspace "$WS" --diff-against tmp/base.ndjson list --depth 1 src/
printf '%s\n' '{"path":"src/lib.rs","owner":"alice"}' | atomwrite --workspace "$WS" --join path:owners --select type,path,owners list --depth 1 src/
```

- Expect sorted `match` events, then group lines such as `{"kind":"file","count":11}`, then cut strings, then one error envelope
- The last call prints the JSON Schema of the whole `search` stream
- `--aggregate` with `sum` reads a numeric field, as in `--aggregate kind:sum:size`
- `--diff-against` compares the stream with a saved baseline, as shown next
- The `--join` call prints `{"type":"entry","path":"src/lib.rs","owners":[{"path":"src/lib.rs","owner":"alice"}]}` and exits 0
- `--join` runs once after sorting and before dedupe, select, filter and limit
- An event without KEY, or with no matching row, stays as it was
- Failure events and the `summary` are never changed by a join
- A join exits 65 (`INVALID_INPUT`) when stdin is a terminal or the table is over 16 MiB
- It also exits 65 for a non-object line, a bad `KEY:SUFFIX`, or a table with no string KEY
- Commands that read stdin refuse with exit 65: `write`, `edit`, `edit-loop`, `apply`, `extract`, `batch` (no `--file`), `hash --stdin`, `diff -`
- Use it with read-only commands such as `search`, `list` and `count`
- A `.gitignore` or `.ignore` at the top of a root that cannot be read or parsed is reported in `files_failed` and `errors[]` with its own path, so a scan never hides a broken ignore file


## Exit Codes and Error Classes
- Read the exit code before parsing stdout, because an error envelope also goes to stdout
- The table has 42 rows and 34 distinct exit numbers, and it comes from `atomwrite agent-surface`
- `atomwrite --help` prints the same list under `Exit codes`
- The error enum has 38 variants, counted with `rg -c '#\[error' src/error/variants.inc.rs`, and `code` in the envelope is a free string, not a closed enum

| Exit | Code | Class | Retryable |
|---|---|---|---|
| 0 | `SUCCESS` | none | no |
| 1 | `NO_MATCHES` | permanent | no |
| 1 | `DOCTOR_CHECKS_FAILED` | precondition_failed | no |
| 2 | `UNSUPPORTED_COMBINATION` | permanent | no |
| 2 | `ARGUMENT_PARSE_ERROR` | permanent | no |
| 4 | `FILE_NOT_FOUND` | permanent | no |
| 4 | `GLOB_NO_MATCHES` | permanent | no |
| 13 | `PERMISSION_DENIED` | permanent | no |
| 28 | `DISK_FULL` | transient | yes |
| 30 | `QUOTA_EXCEEDED` | transient | yes |
| 65 | `INVALID_INPUT` | permanent | no |
| 65 | `MATCH_FAILED` | permanent | no |
| 65 | `MATCH_AMBIGUOUS` | permanent | no |
| 65 | `BINARY_FILE` | precondition_failed | no |
| 65 | `FILE_TOO_LARGE` | precondition_failed | no |
| 69 | `GRAMMAR_UNAVAILABLE` | precondition_failed | no |
| 73 | `CROSS_DEVICE` | conflict | yes |
| 74 | `IO_ERROR` | transient | yes |
| 78 | `CONFIG_INVALID` | permanent | no |
| 79 | `SYMLINK_BLOCKED` | precondition_failed | no |
| 80 | `TRANSACTION_ROLLBACK_FAILED` | permanent | no |
| 81 | `CHECKSUM_VERIFY_FAILED` | precondition_failed | no |
| 82 | `STATE_DRIFT` | conflict | yes |
| 83 | `LOCK_TIMEOUT` | conflict | yes |
| 84 | `MERGE_CONFLICT` | permanent | no |
| 85 | `FIFO_DETECTED` | precondition_failed | no |
| 86 | `DEVICE_FILE` | precondition_failed | no |
| 87 | `EDIT_LOOP_PARTIAL` | permanent | no |
| 88 | `SYNTAX_ERROR_DETECTED` | precondition_failed | no |
| 89 | `ALL_SKIPPED_BINARY` | permanent | no |
| 91 | `EXDEV_FALLBACK_DISABLED` | precondition_failed | no |
| 92 | `COPY_BACK_BLAKE3_FAILED` | conflict | yes |
| 93 | `ORPHAN_JOURNAL` | precondition_failed | no |
| 124 | `TIMEOUT` | precondition_failed | no |
| 126 | `WORKSPACE_JAIL` | precondition_failed | no |
| 128 | `IMMUTABLE_FILE` | precondition_failed | no |
| 129 | `SIGHUP` | none | no |
| 130 | `CANCELLED` | permanent | no |
| 130 | `SIGINT` | none | no |
| 141 | `BROKEN_PIPE` | permanent | no |
| 143 | `SIGTERM` | none | no |
| 255 | `INTERNAL_ERROR` | permanent | no |

- Exit 0 is success and exit 1 `NO_MATCHES` is a normal answer of `search`, `replace`, `transform`, `scope` and `semantic-search`
- A failing run ends in exactly one terminal event with `ok: false`, and `suggestion` lives inside it
- The default error envelope carries `ok`, `code`, `exit`, `path` when there is one, `suggestion`, `target_resolved` and `target_source`
- Pass `-v` to add the diagnostic prose back: `message`, `error_class`, `retryable` and `suggestion_lang`
- The same `-v` raises stderr logging to info, while stdout keeps its single terminal line
- `MATCH_FAILED` adds `match_count` and `best_candidate`, and `MATCH_AMBIGUOUS` adds `best_candidate` and `match_count`
- Classes are `none`, `permanent`, `precondition_failed`, `transient` and `conflict`
- Retry only when `retryable` is true, back off on `transient` and re-read the file on `conflict`
- Exit 82 means reload the checksum, exit 124 means raise `--timeout-secs` or narrow the path
- Exit 126 is the jail and exit 143 is a clean SIGTERM
- Exit 129 is SIGHUP, and exit 130 is SIGINT or a cancelled run
- Exit 141 means the reader closed the pipe, so output is incomplete and the state needs a re-check
- Exit 78 covers a bad config file, an unknown config key and a verb that is missing from the build
- Every verb that walks a tree lists unreadable files in `files_failed`, `partial` and `errors[]`, so read them, and `errors[].path` is the marker `<walk>` when the walk failed without naming a file of its own
- A write that fails because the reader closed exits 141 (`BROKEN_PIPE`), including `completions`, `--print-schema`, `--help` and every verb

```json
{"ok":false,"ok":false,"code":"MATCH_FAILED","exit":65,"target_resolved":["src/a.rs"],"target_source":"argv","match_count":0,"suggestion":"inspect best_candidate in the error NDJSON and adjust --old, or lower --fuzzy-threshold"}
```

- SIGPIPE has a cooperative handler that ends the run with exit 141 after cleanup
- The run never leaves a lock, journal or tempfile behind
- This is a deliberate deviation from the literal rule of resetting SIGPIPE to its default action


## Configuration
- Precedence is flags first, then the workspace `.atomwrite.toml`, then the global `config.toml`, then built-in defaults
- The two files are merged key by key, so a workspace file hides only the keys it names
- `--config PATH` replaces both files, and `config get` then reports `origin` `explicit`
- atomwrite has no product environment variables, so a value comes from a flag or from one of these files
- Change a key with `atomwrite config set KEY VALUE --scope global|workspace`, and avoid editing the files by hand
- A `set` keeps comments, validates the whole file and lands atomically, with mode 0600 for the global file on Unix
- `config` still works when `config.toml` is broken, and a file that is not valid TOML is never rewritten
- The terminal event of `set` and `unset` carries `target_resolved`, so check that the path is the file you meant

### The 43 Keys
- Run `atomwrite config list` for the live table with type, default, value, origin and scopes
- `log.file`, `log.rotation`, `storage.home` and `locale` are global-only, and the workspace scope exits 65
- An unknown key or a value outside its range exits 65 and writes nothing

| Key | Type | Default | Accepted values | Scopes |
|---|---|---|---|---|
| `defaults.line_ending` | string | `auto` | `auto`, `lf`, `crlf` | global, workspace |
| `defaults.backup` | boolean | none (true for rewrites, false for delete) | `true`, `false` | global, workspace |
| `delete.backup` | boolean | none (falls back to `defaults.backup`) | `true`, `false` | global, workspace |
| `defaults.retention` | integer | 5 | 0 to 255 | global, workspace |
| `defaults.max_filesize` | integer | 1073741824 | 1 or more | global, workspace |
| `fuzzy.mode` | string | `auto` | `auto`, `aggressive`, `off` | global, workspace |
| `fuzzy.threshold` | float | 0.7 | 0 to 1 | global, workspace |
| `fuzzy.threshold_aggressive` | float | 0.5 | 0 to 1 | global, workspace |
| `fuzzy.threshold_context` | float | 0.8 | 0 to 1 | global, workspace |
| `fuzzy.threshold_jw` | float | 0.85 | 0 to 1 | global, workspace |
| `fuzzy.soft_floor_delta` | float | 0.05 | 0 to 1 | global, workspace |
| `fuzzy.max_content_bytes` | integer | 268435456 | 1 or more | global, workspace |
| `apply.max_ellipsis_lines` | integer | 200 | 1 or more | global, workspace |
| `write.confirm_large_bytes` | integer | 102400 | 1 or more | global, workspace |
| `write.shrink_block_percent` | integer | 50 | 1 to 99 | global, workspace |
| `write.auto_rotate_max_age_secs` | integer | 86400 | 1 or more | global, workspace |
| `write.content_risk_patterns` | list of strings | the built-in risky-command list | any string | global, workspace |
| `watch.idle_exit_ms` | integer | 500 | 0 or more (0 disables) | global, workspace |
| `watch.debounce_ms` | integer | 200 | 1 or more | global, workspace |
| `log.file` | boolean | false | `true`, `false` | global |
| `log.rotation` | string | `daily` | `daily`, `hourly` | global |
| `storage.home` | string | none | a directory | global |
| `locale` | string | none | `en`, `pt-BR` | global |
| `concurrency.max_threads` | integer | 16 | 1 to 256 | global, workspace |
| `concurrency.ram_fraction_percent` | integer | 50 | 1 to 100 | global, workspace |
| `search.context` | integer | 0 | 0 or more | global, workspace |
| `search.smart_case` | boolean | true | `true`, `false` | global, workspace |
| `search.max_filesize` | integer | 10485760 | 1 or more | global, workspace |
| `search.max_columns` | integer | 500 | 1 or more | global, workspace |
| `search.max_inflight_bytes` | integer | 8388608 | 4 MiB to 1 GiB | global, workspace |
| `search.format` | string | `ndjson` | `ndjson`, `text` | global, workspace |
| `read.max_output_bytes` | integer | 24000 | 1 or more | global, workspace |
| `read.number` | boolean | false | `true`, `false` | global, workspace |
| `output.profile` | string | `full` | `full`, `lean` | global, workspace |
| `summary.errors_max_entries` | integer | 50 | 1 to 1000 | global, workspace |
| `lock.acquire_timeout_ms` | integer | 5000 | 100 to 3600000 | global, workspace |
| `retry.base_ms` | integer | 50 | 1 to 60000, not above `retry.max_ms` | global, workspace |
| `retry.max_ms` | integer | 400 | 1 to 60000 | global, workspace |
| `retry.max_elapsed_ms` | integer | 2000 | 1 to 3600000 | global, workspace |
| `extract.max_sample_rows` | integer | 10 | 0 to 100000 | global, workspace |
| `extract.max_text_chars` | integer | 400 | 1 to 10000000 | global, workspace |
| `extract.max_sheets` | integer | 32 | 1 to 10000 | global, workspace |
| `extract.max_slides` | integer | 64 | 1 to 10000 | global, workspace |

- A flag still wins over a key, as `extract-artifact --max-text-chars` wins over `extract.max_text_chars`
- `fuzzy.max_content_bytes` is the ceiling above which the fuzzy cascade does not run, and exact matches still work
- `write.confirm_large_bytes` is the size above which an overwrite needs `--ack-overwrite`
- `search.format` decides whether `search` writes NDJSON or text when `--format` is absent, and the flag always wins over the key
- `output.profile = "lean"` drops the digests, the byte counts and the timing from a successful mutation record
- `retry.*` shapes the jittered retry of lock and rename waits, and `--timeout-secs` still caps it
- `storage.home` relocates config and state under `{home}/config` and `{home}/state`

```bash
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace
atomwrite --workspace "$WS" config get fuzzy.threshold
atomwrite --workspace "$WS" config set log.file true --scope global
```

- The first call writes `.atomwrite.toml` in the workspace, and the second reports `origin` `workspace`
- The third call turns on the diagnostics file, which only the global scope can do

### Config File Layout
- A file that is found must be readable, valid TOML and free of unknown keys
- Otherwise the run exits 78 with `CONFIG_INVALID`
- The message names the file, and a `version` newer than the supported value 1 also exits 78
- The loader accepts `version` and 11 sections: `[defaults]`, `[write]`, `[watch]`, `[fuzzy]`, `[search]`, `[storage]`, `[log]`, `[concurrency]`, `[lock]`, `[retry]` and `[extract]`
- Some fields have no `config` key, so you set them by editing the file

| Field | Meaning |
|---|---|
| `defaults.backup`, `defaults.retention` | Default backup switch and retention for mutating commands; an absent backup key keeps each verb's own default |
| `write.shrink_block_percent` | Shrink guard threshold, default 50 |
| `write.auto_rotate_max_age_secs` | Age window of `--auto-rotate`, default 24 hours |
| `write.content_risk_patterns` | Extra patterns for the write risk assessment |
| `watch.idle_exit_ms`, `watch.debounce_ms` | Defaults of `watch` |
| `search.context`, `search.smart_case`, `search.max_filesize`, `search.max_columns` | Defaults of `search` |
| `defaults.line_ending`, `defaults.max_filesize` | Read by the loader but not used by any command |

```toml
[defaults]
backup = true
retention = 5

[fuzzy]
mode = "auto"
threshold = 0.85

[search]
max_filesize = 10485760
```

- This is a valid `.atomwrite.toml` for the workspace root, and an unknown field such as `defaults.workspace` would exit 78
- `[log]` in a workspace file is ignored, because tracing starts before the workspace file is read

### Where Files Live
- Linux paths were measured, and macOS and Windows paths come from the code (`directories::ProjectDirs`) and were not validated

| Item | Linux path |
|---|---|
| Global config | `$XDG_CONFIG_HOME/atomwrite/config.toml` (default `~/.config/atomwrite/config.toml`) |
| Workspace config | `.atomwrite.toml` in the workspace root |
| Locale preference | `$XDG_CONFIG_HOME/atomwrite/locale` |
| Diagnostics log | `$XDG_STATE_HOME/atomwrite/logs/atomwrite.<date>.jsonl`, file mode 0600 in a 0700 folder |
| Tree-sitter parsers | `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` |
| Bash completions | `$XDG_DATA_HOME/bash-completion/completions/atomwrite` |
| Zsh completions | `$XDG_DATA_HOME/zsh/site-functions/_atomwrite` |
| Fish completions | `$XDG_DATA_HOME/fish/vendor_completions.d/atomwrite.fish` |
| Elvish and PowerShell | `$XDG_DATA_HOME/elvish/lib/atomwrite.elv` and `$XDG_DATA_HOME/powershell/Completions/_atomwrite.ps1` |

- The diagnostics log is off by default, so no file is opened and no writer thread starts
- When on, it rotates daily or hourly, keeps 7 files and writes JSON lines
- Stderr diagnostics stay JSON lines either way


## Advanced Patterns

### Optimistic Locking With a Checksum
- Capture the checksum, then write only if the file did not change

```bash
CS="$(atomwrite --workspace "$WS" read src/lib.rs | jaq -r .checksum)"
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn sub(a: i32, b: i32) -> i32 {\n    a - b\n}\n' | atomwrite --workspace "$WS" write --expect-checksum "$CS" src/lib.rs
```

- Expect `type` `write` with `checksum_before` equal to `$CS`
- A changed file exits 82 (`STATE_DRIFT`), and you reload the checksum and decide again

### Sequential Edits on One File
- Re-capture the checksum before each guarded edit, or accept drift on purpose

```bash
CS="$(atomwrite --workspace "$WS" hash src/b.rs | jaq -r .checksum)"
atomwrite --workspace "$WS" edit --expect-checksum "$CS" src/b.rs --old 'beta' --new 'beta_one'
atomwrite --workspace "$WS" edit --expect-checksum "$CS" --allow-sequential-drift src/b.rs --old 'beta_one' --new 'beta_two'
```

- The first edit passes, and the second would exit 82 without `--allow-sequential-drift`
- Drift accepted that way is logged as a warning on stderr

### Scratch Staging for Multi-line Patches
- Text with quotes, `$` or slashes is fragile in argv, so stage it in files inside the jail

```bash
atomwrite --workspace "$WS" scratch init .atomwrite/scratch
SCR=".atomwrite/scratch"
OLD=$'fn load() -> Option<i32> {\n    Some(1)\n}'
NEW=$'fn load() -> Option<i32> {\n    Some(2)\n}'
printf %s "$OLD" > "$SCR/old.txt"
printf %s "$NEW" > "$SCR/new.txt"
atomwrite --workspace "$WS" edit src/main.rs --old-file "$SCR/old.txt" --new-file "$SCR/new.txt"
```

- Expect `type` `edit` with `edits` 1
- A content file under the OS temporary directory exits 126 (`WORKSPACE_JAIL`)

### Find, Preview, Replace, Confirm
- Locate first, preview the change, apply it, then confirm nothing is left

```bash
atomwrite --workspace "$WS" search 'beta' src/
atomwrite --workspace "$WS" replace --dry-run beta_two beta src/
atomwrite --workspace "$WS" replace beta_two beta src/b.rs
atomwrite --workspace "$WS" search 'beta_two' src/b.rs
```

- The last search exits 1, which means no match is left in the file
- A successful `replace` deletes the `.bak` copy it made, and `search` still walks `.bak` files, so name the file

### Atomic Multi-file Change
- A batch with `--transaction` commits all operations or none

```bash
printf '%s\n' '{"op":"write","target":"b1.txt","content":"x"}' '{"op":"copy","source":"b1.txt","target":"b2.txt"}' '{"op":"move","source":"b2.txt","target":"b3.txt"}' '{"op":"delete","target":"b3.txt"}' > tmp/manifest.ndjson
atomwrite --workspace "$WS" batch --file tmp/manifest.ndjson --transaction
```

- Expect four `batch_op` lines and a `summary` with `committed` set to `true`

### Undo a Bad Edit
- Take a backup before a risky change and restore it if the result is wrong

```bash
atomwrite --workspace "$WS" backup src/lib.rs
printf 'broken\n' | atomwrite --workspace "$WS" write --allow-shrink src/lib.rs
atomwrite --workspace "$WS" rollback src/lib.rs --latest --verify
```

- Expect `type` `rollback` with `verified` set to `true`
- The file is back to its content before the bad write

### Compact Output for Agents
- Ask for only the keys you need, and filter on the outcome

```bash
atomwrite --workspace "$WS" --select ok,code,match_count,suggestion --filter ok=false --limit 5 edit src/main.rs --old nothere --new x --fuzzy off
```

- Expect one line such as `{"ok":false,"code":"MATCH_FAILED","match_count":0,"suggestion":"..."}` and exit 65
- `--filter` never hides an error envelope, so a failure with `ok: false` always arrives


## Integration With Shell Scripts
- Check the exit code first, and treat exit 1 from search-like commands as an answer
- Use `jaq` for fields, and keep `${PIPESTATUS[0]}` when a pipe follows `atomwrite`

```bash
if atomwrite --workspace "$WS" search 'TODO' src/ > tmp/hits.ndjson; then
  echo "found"
else
  code=$?
  if [ "$code" -eq 1 ]; then echo "none"; else echo "failed: $code"; fi
fi
```

- Expect `found` while a `TODO` exists, and `none` once it is gone
- A write whose pipe feeds `jaq` hides its status unless you read `${PIPESTATUS[0]}`

```bash
out="$(atomwrite --workspace "$WS" edit src/main.rs --old nothere --new x --fuzzy off)"; code=$?
printf '%s\n' "$out" | jaq -r .ok
echo "$code"
```

- Expect `false` and `65`
- Pass an absolute `--workspace` to detached processes, because they do not inherit a useful working directory


## Integration With AI Agents
- Every subcommand writes deterministic NDJSON on stdout, and every write returns a BLAKE3 `checksum`
- Run `atomwrite agent-surface --format json` once to learn tools, exit codes and schema names
- Always pass `--workspace` as an absolute path, and use `--select`, `--filter` and `--limit` to save tokens
- Read the exit code before the fields, and read `best_candidate` after exit 65
- Run `--dry-run`, `--plan` or `--preview` before any bulk change
- Use `--expect-checksum` for concurrent edits, and `--ack-overwrite` only for intended large overwrites
- Never pass `delete --confirm` or `delete --yes`, and never run `watch` without `--max-events` or a timeout
- See [AGENTS.md](AGENTS.md) for the full agent contract and the exit code handling


## Integration With Rust Crates
- atomwrite is a command line tool, and the supported way to call it from Rust is a subprocess
- Check the exit status first, then parse the NDJSON lines from stdout

```rust
use std::process::Command;

fn main() -> std::io::Result<()> {
    let ws = std::env::current_dir()?;
    let out = Command::new("atomwrite")
        .arg("--workspace")
        .arg(&ws)
        .args(["hash", "src/lib.rs"])
        .output()?;
    match out.status.code() {
        Some(0) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let first = stdout.lines().next().unwrap_or("");
            println!("ok: {first}");
        }
        Some(code) => eprintln!("atomwrite exited with {code}: {}", String::from_utf8_lossy(&out.stdout)),
        None => eprintln!("atomwrite was killed by a signal"),
    }
    Ok(())
}
```

- Expect `ok:` followed by the `hash` event when run from the sample workspace
- Parse each line with `serde_json` in a real project, and validate it against `atomwrite --print-schema NAME`


## Common Errors
- The first bullet of each entry is the symptom, and the next ones are cause and fix

### Exit 65 MATCH_FAILED
- An `edit` or `edit-loop` pair found no match
- Read `best_candidate`, widen `--old`, or use `--fuzzy auto` or `--fuzzy aggressive`

### Exit 65 on a Large or Shrinking Write
- The target is above 100 KiB, or the new content is under half of the old size
- Pass `--ack-overwrite` for the first case and `--allow-shrink` for the second, only when intended

### Exit 82 STATE_DRIFT
- The file changed since you read its checksum
- Reload the checksum and decide again, or use `--allow-sequential-drift` in a controlled pipeline

### Exit 126 WORKSPACE_JAIL
- A path points outside `--workspace`
- Use a path inside the workspace, and stage content files under `.atomwrite/scratch`

```json
{"ok":false,"ok":false,"code":"WORKSPACE_JAIL","exit":126,"suggestion":"use a path inside the workspace (/home/user/project); for --old-file/--new-file stage under /home/user/project/.atomwrite/scratch/ (never OS /tmp outside the jail)","path":"/etc/passwd","workspace":"/home/user/project"}
```

### Exit 4 FILE_NOT_FOUND
- The path does not exist, and the envelope lists `similar_paths`
- Check the spelling, or use `list` and `search --target files` to find the file

### Exit 2 ARGUMENT_PARSE_ERROR
- A flag does not exist or a required argument is missing, and `message` carries the clap usage
- Run `atomwrite COMMAND --help` and use only flags listed there

### Exit 1 NO_MATCHES
- A search-like command found nothing, which is an answer and not a crash
- Broaden the pattern, or review `--include`, `--exclude` and `--max-filesize`

### Exit 78 CONFIG_INVALID
- A config file is malformed, has an unknown key, or the verb is missing from the build, as with `watch`
- Fix the file named in `message`, or reinstall with `--features watch`

### Exit 88 SYNTAX_ERROR_DETECTED
- `--syntax-check` or `--verify-parse` found a parse error in the result
- Fix the content, and expect the first call per language to download the parser

### Silent Zero From case on a Directory
- `case` on a directory exits 0 with `identifiers_total` 0 and changes nothing
- Pass the files instead

### The prune-backups Dry Run
- `--dry-run` takes no value, and `--dry-run false` reads `false` as a path
- Omit `--dry-run` to apply, and name the target files whose backups you want to prune

### Broken Pipe
- `read` and `search` exit 141 when the reader closes the pipe, and the output is incomplete
- This covers `completions`, `--print-schema`, `--help` and every verb, with no panic and no crash report


## Modification Time And Build Systems
- `edit` and `replace` update the file mtime to the end of the write, which build tools need
- Cargo, make, cmake, gradle, bazel and ninja compare mtimes to decide what to rebuild
- A preserved old mtime can make cargo skip a rebuild and report a stale binary as finished
- Use `--preserve-timestamps` only for snapshots, reproducible artifacts or files outside any build
- The field `mtime_preserved` in the response tells you which case happened


## Windows Installation
- Install Visual Studio Build Tools with the C++ desktop workload and Rust 1.90 or later
- Use Windows Terminal or PowerShell 7 for UTF-8 output and ANSI sequences
- The advisory write lock is Unix-only, and Windows has no write lock yet
- See [INSTALL.md](INSTALL.md) for the full Windows guide and troubleshooting


## Release History
- This section is historical, and the details live in the [CHANGELOG](../CHANGELOG.md) and [MIGRATION](MIGRATION.md)

### v0.1.36 (2026-07-31)
- Historical: 46 subcommands, and `scratch init` staging under `.atomwrite/scratch/`
- Failures emit `ok: false` and a stable `match_count`, so check the exit before success fields
- Global `--select`, `--filter` and `--limit` shape stdout on every subcommand
- Normalized fuzzy strategies report `similarity: null`, and `block_anchor` uses a dual gate

### v0.1.35 (2026-07-19)
- Large overwrites became default-deny, and `--ack-overwrite` is the one-shot acknowledgement
- `delete --plan` became the way to list targets, and `delete --confirm` and `--yes` were rejected
- `watch` always emits a final `watch_summary`, with a default idle exit of 500 ms
- `semantic-merge` writes conflict markers by default

### v0.1.34 and v0.1.30
- The global timeout default became 120 seconds, and exit 124 marks a deadline
- Fuzzy multi-apply became one-pass from left to right, with a default of one apply
- Success lines gained `match_count` and `indent_adjusted`, and misses gained `best_candidate`
- Historical: 41 subcommands at that point

### v0.1.12 to v0.1.29
- v0.1.12 added `set`, `get`, `del`, `case`, `query` and `outline`
- v0.1.18 added `wal-stats` and `wal-heal`, v0.1.22 added `edit-loop` and `prune-backups`, v0.1.25 added `verify`
- v0.1.23 made backups default on, and v0.1.28 unified the backup flags across the mutating commands
- v0.1.29 added `recipe`, `sparse`, `semantic-merge`, `agent-surface`, `watch`, `codemod`, `semantic-search` and `stat`


## Next Steps
- Browse longer recipes in the [COOKBOOK](COOKBOOK.md)
- Read the agent contract in [AGENTS.md](AGENTS.md)
- Check platform notes in [CROSS_PLATFORM.md](CROSS_PLATFORM.md)
- Upgrade with [MIGRATION.md](MIGRATION.md), and install with [INSTALL.md](INSTALL.md)
- Validate output with the schemas in [docs/schemas](schemas/README.md)
