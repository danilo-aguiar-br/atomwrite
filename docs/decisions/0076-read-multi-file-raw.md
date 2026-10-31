# ADR-0076 — `read` takes several operands and expands one level of glob

## Status
Accepted (2026-10-28)

## Context
- `read` took one `PathBuf`, so the Claude Code hook could not rewrite `cat a b` or `cat *.rs` into an atomwrite call, and every glob denial cost an extra turn of context.
- Measured before the change: `atomwrite read --format raw a.rs b.rs` exited 2 with `unexpected argument 'b.rs' found`, and `cat` on the same operands printed both files.
- Measured before the change: `atomwrite read --format raw <dir>` exited 74 and wrote the NDJSON error envelope on STDOUT, so the byte channel carried a JSON record where a file should be.
- The shell order of a glob follows `LC_COLLATE`: with `LC_ALL=C` the order is the byte order of the names, which is the only order a portable binary can promise.
- `globset 0.4` is already in the graph through `ignore 0.4`, so matching a name against a compiled pattern adds no crate to the build.

## Decision
### 1. One variadic slot, not two
- `ReadArgs.path` became `ReadArgs.paths: Vec<PathBuf>` with `num_args = 1..` and `required = true`, and a bare `read` still fails at parse time.
- Explicit operands keep their argv position, so `read a b` and `read b a` differ exactly the way `cat` differs.
- `read` and `stat` are read-only verbs and do not appear in `explicit_target_roots`, so the variadic slot carries no ambient authority and the ETD rule is satisfied rather than bent.
- `stat` reuses `ReadArgs`, so several targets exit 65 with a message that names the verb and points at `--format raw`.

### 2. The binary expands the pattern, and pays for it with a narrow contract
- `src/commands/read_globs.inc.rs` splits an operand into the directory to list and the final component to match, lists ONE level with `std::fs::read_dir`, and matches with `globset::GlobBuilder::literal_separator(true)`.
- Names are sorted by `OsStr::as_encoded_bytes`, which is the bytes on Unix and WTF-8 on Windows, so one comparator orders the three platforms.
- A name that starts with `.` is skipped, because the shell `*` never matches a leading dot.
- A pattern with `**` exits 65, and so does a pattern with a wildcard in a directory name. Refusing is chosen over expanding halfway: a silent partial answer hands the caller fewer bytes than it asked for, which is the failure mode this verb exists to remove.
- A literal directory prefix may be as deep as the caller writes, because only the final component is a pattern, and an operand that names an existing path is read as it is instead of being expanded.
- The directory the pattern reaches goes through `path_safety::validate_path`, and every expanded target goes through it again, so a pattern cannot widen the `--workspace` jail.

### 3. Concatenation streams, and one target is untouched
- One target keeps the exact pre-change path: `read_file_bytes` plus `apply_line_filters`, so `--head`, `--lines`, `--verify-checksum` and the byte-for-byte proof against 0.1.37 still hold.
- Several targets copy block by block into one locked stdout, with no header, no separator and no added newline, and the buffer is a `RAW_COPY_BUF_BYTES` constant so the peak memory does not follow the sum of the sizes.
- The copy advances a `CancelCheck::for_bytes`, so `--timeout-secs` cuts a 196 MB concatenation in the middle instead of after the last file.
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` and `--verify-checksum` exit 65 with several targets, because `cat` has no line range and silently applying one to every file is worse than a refusal.

### 4. The byte channel never carries an error envelope
- `is_text_mode` became `is_byte_stream_mode` and covers `read --format raw` and `stat --format raw` as well as `search --format text`, so a failure in those modes goes to stderr and stdout stays empty.
- A missing file, a directory or a glob that matched nothing prints one line on stderr and the other targets still reach stdout.
- The status is the FIRST failure's semantic code, which is the owner's choice over `cat`'s flat 1: absent is 4, permission is 13, jail is 126.

## Consequences
- The hook can rewrite `cat a b` and `cat 'src/*.rs'` into one atomwrite call, and `read_replace` in the guard now resolves every operand before it rewrites a segment.
- A recursive glob is out of scope by decision, not by measurement, and the refusal names the limit instead of hiding it.
- macOS and Windows are not executed on this host: the byte order and the one-level listing are proven on Linux only.
