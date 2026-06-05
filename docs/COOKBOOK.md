# atomwrite Cookbook


[Leia em Portugues](COOKBOOK.pt-BR.md)

> Practical recipes you can copy-paste into your agent workflows


## Latency Note
- All operations execute locally with sub-millisecond overhead
- The atomic write sequence adds ~1ms for the fsync-rename-fsync cycle
- Search parallelism scales with available CPU cores
- Batch mode amortizes startup cost across N operations


## Default Values Reference
- `--threads` defaults to number of available CPU cores
- `--max-filesize` defaults to 1 GiB (1,073,741,824 bytes)
- `--color` defaults to `auto` (detect terminal)
- `--workspace` defaults to current working directory
- `diff --context` defaults to 3 lines
- `diff --algorithm` defaults to `patience`


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
atomwrite scope --query comments --delete src/main.rs
```

- Uppercase all function names in Python:

```bash
atomwrite scope --query def --action upper src/app.py
```

- Squeeze whitespace in strings:

```bash
atomwrite scope --query strings --action squeeze src/lib.rs
```

- Replace comments with a standard header:

```bash
atomwrite scope --query comments --replace-with "// TODO: review" src/main.rs
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

- Set backup retention to 30 days:

```bash
atomwrite backup --retention 30 src/config.toml
```

- Restore the most recent backup:

```bash
atomwrite rollback src/main.rs --latest
```

- Restore a specific timestamped backup:

```bash
atomwrite rollback src/main.rs --timestamp 2026-05-29T12-00-00
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
PATTERN=$(atomwrite regex "v1.0.0" "v2.1.3" "v10.0.1" | atomwrite extract regex)
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

- Preview the batch without executing:

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
  atomwrite extract path | \
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
- Read a file and capture the checksum:

```bash
CHECKSUM=$(atomwrite read --stat src/config.toml | atomwrite extract checksum)
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
  CHECKSUM=$(atomwrite read --stat src/config.toml | atomwrite extract checksum)
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

- Set retention period for backups:

```bash
atomwrite delete --backup --retention 30 src/old_module.rs
```

- List backup files in a directory:

```bash
atomwrite list --long .atomwrite-backups/
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
<<<< SEARCH
old_function_name
==== REPLACE
new_function_name
>>>> END
EOF
```


## Agent-First Patterns (v0.1.2+)

### Bound a Long Search with Timeout

```bash
# Aborts after 60s if search doesn't finish; emits NDJSON error with error_class=transient
atomwrite --workspace . --timeout 60 search 'TODO' src/
```

### Read Only Lines Matching a Regex

```bash
# Useful for extracting logs from huge files without exhausting context
atomwrite --workspace . read --grep 'ERROR|WARN' /var/log/app.log
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
# Keep source directory clean; centralize backups
atomwrite --workspace . backup --output-dir /var/backups/atomwrite src/critical.rs
```

### Install Shell Completions on First Use

```bash
# Auto-installs to ~/.local/share/bash-completion/completions/atomwrite
atomwrite completions bash --install
```

### Use the Environment Variable for Workspace

```bash
# For agents that don't pass --workspace explicitly
export ATOMWRITE_WORKSPACE=/home/user/project
atomwrite read src/main.rs
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
atomwrite read /etc/passwd 2>/dev/null
# Output: {"error":true,"code":"WORKSPACE_JAIL","exit":126,...,"suggestion":"set --workspace <root> or export ATOMWRITE_WORKSPACE=<path>",...}

# When workspace IS provided, the suggestion says "use a path inside"
atomwrite --workspace /home/user/project read /etc/passwd 2>/dev/null
# Output: {"error":true,"code":"WORKSPACE_JAIL","exit":126,...,"suggestion":"use a path inside the workspace (/home/user/project)",...}
```


## How to Install on Windows 10/11 (v0.1.4)
- v0.1.4 finally fixes `cargo install atomwrite` on Windows
- Install Visual Studio 2019+ Build Tools with the C++ workload
- Install Rust 1.85+ via rustup
- Run `cargo install atomwrite --locked`
- See [INSTALL.md](INSTALL.md) for the full Windows troubleshooting guide

```powershell
# PowerShell 7+ or Windows Terminal
rustup default stable
rustup target add x86_64-pc-windows-msvc
cargo install atomwrite --locked
atomwrite --version  # NDJSON output
```
