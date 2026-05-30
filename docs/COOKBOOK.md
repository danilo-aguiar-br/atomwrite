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
- `--max-filesize` defaults to 100 MiB
- `--color` defaults to `auto` (detect terminal)
- `--workspace` defaults to current working directory
- `diff --context` defaults to 3 lines
- `diff --algorithm` defaults to `myers`


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
atomwrite scope --query comments --action delete src/main.rs
```

- Uppercase all function names in Python:

```bash
atomwrite scope --query functions --action upper src/app.py
```

- Squeeze whitespace in strings:

```bash
atomwrite scope --query strings --action squeeze src/lib.rs
```

- Replace comments with a standard header:

```bash
atomwrite scope --query comments --action replace --replacement "// TODO: review" src/main.rs
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

- Find all unwrap calls without modifying:

```bash
atomwrite transform --pattern '$EXPR.unwrap()' -l rust src/
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
- Create an NDJSON manifest with multiple operations:

```bash
cat <<'EOF' > manifest.ndjson
{"op":"write","path":"src/a.txt","content":"hello"}
{"op":"write","path":"src/b.txt","content":"world"}
{"op":"delete","path":"src/old.txt"}
EOF
cat manifest.ndjson | atomwrite batch
```

- Preview the batch without executing:

```bash
cat manifest.ndjson | atomwrite batch --dry-run
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
