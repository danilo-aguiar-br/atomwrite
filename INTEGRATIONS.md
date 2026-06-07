[Leia em Portugues](INTEGRATIONS.pt-BR.md)


# Integrations

> atomwrite works with every LLM agent that can execute shell commands


## Compatible Agents (v0.1.12)
- atomwrite requires only `bash` access to function
- Any agent that can run a shell command can use atomwrite
- NDJSON output is parseable by every major LLM without custom adapters
- No plugins, extensions, or SDKs required
- **28 subcommands** as of v0.1.12 (6 new: set, get, del, case, query, outline)
- As of v0.1.12, atomwrite runs on Windows 10/11, Linux, and macOS with identical NDJSON contract
- The v0.1.12 release added 5 new error variants and 445 tests across 43 test suites


## Summary Table

| Agent | Shell Access | NDJSON Parsing | Integration Effort |
|-------|-------------|----------------|-------------------|
| Claude Code | Native | Native | Zero config |
| Cursor | Native | Native | Zero config |
| Windsurf | Native | Native | Zero config |
| Codex CLI | Native | Native | Zero config |
| Aider | Native | Native | Zero config |
| Continue | Native | Native | Zero config |
| Cline | Native | Native | Zero config |
| Roo Code | Native | Native | Zero config |
| Amazon Q Developer | Native | Native | Zero config |
| GitHub Copilot CLI | Native | Native | Zero config |
| Custom Agents | Via subprocess | Via JSON parser | One `cargo install` |


## Claude Code
- Install: `cargo install atomwrite`
- Claude Code executes bash commands natively
- NDJSON output is parsed directly without adapters
- Use `--workspace` to match the project root
- Pair with `--expect-checksum` for safe concurrent edits
- Add atomwrite commands to CLAUDE.md for automatic discovery
- v0.1.12: use `set/get/del/case/query/outline` for structured config and AST analysis
```bash
# Example CLAUDE.md entry
# Use atomwrite for all file operations
echo "content" | atomwrite --workspace . write src/file.rs
atomwrite --workspace . read src/file.rs
atomwrite --workspace . search 'pattern' src/
# v0.1.12: walk a Rust file's AST
atomwrite --workspace . query src/main.rs --kinds
```


## Cursor
- Install: `cargo install atomwrite`
- Cursor executes terminal commands via its built-in shell
- NDJSON responses integrate directly with tool-use flows
- Use `--dry-run` for preview before destructive operations
```bash
atomwrite --workspace . search 'TODO' src/ --include '*.rs'
atomwrite --workspace . replace 'old_api' 'new_api' src/
```


## Windsurf
- Install: `cargo install atomwrite`
- Windsurf runs shell commands through its terminal integration
- Structured output reduces token consumption compared to raw CLI tools
- Batch operations minimize the number of tool calls
```bash
cat manifest.ndjson | atomwrite --workspace . batch
```


## Codex CLI
- Install: `cargo install atomwrite`
- Codex CLI executes commands in a sandboxed shell
- atomwrite respects sandbox boundaries via `--workspace`
- Checksums enable state verification across execution steps
- v0.1.12: use `case` to refactor identifiers across multiple files
```bash
atomwrite --workspace . read src/main.rs
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . case src/ --subvert user_id UserId --to pascal
```


## Aider
- Install: `cargo install atomwrite`
- Aider runs shell commands for code editing workflows
- atomwrite provides atomic guarantees that shell built-ins lack
- Use `edit` for surgical changes instead of full file rewrites
- v0.1.12: use `outline` to give Aider a map of the codebase before edits
```bash
atomwrite --workspace . edit src/lib.rs --old "old code" --new "updated code"
atomwrite --workspace . outline src/  # see the structure first
```


## Continue
- Install: `cargo install atomwrite`
- Continue executes terminal commands as part of its agent loop
- NDJSON output is machine-readable without post-processing
```bash
atomwrite --workspace . search 'deprecated' src/ --include '*.rs'
```


## Cline
- Install: `cargo install atomwrite`
- Cline uses shell commands for file operations
- atomwrite replaces fragile sed/awk pipelines with atomic operations
```bash
echo "new content" | atomwrite --workspace . write src/config.rs
atomwrite --workspace . diff src/old.rs src/new.rs
```


## Roo Code
- Install: `cargo install atomwrite`
- Roo Code runs bash commands in its agent execution environment
- Structured errors with `retryable` and `suggestion` fields guide automatic recovery
```bash
atomwrite --workspace . copy src/template.rs src/new.rs
```


## Amazon Q Developer
- Install: `cargo install atomwrite`
- Amazon Q executes CLI commands in its development environment
- atomwrite provides consistent cross-platform behavior
```bash
atomwrite --workspace . list src/ --depth 3
atomwrite --workspace . count src/ --by-extension
```


## GitHub Copilot CLI
- Install: `cargo install atomwrite`
- Copilot CLI suggests and executes shell commands
- atomwrite commands are self-documenting via `--help` on each subcommand
- v0.1.12: use `get` to read package metadata without writing a parser
```bash
atomwrite calc "100 MB to bytes"
atomwrite regex "192.168.1.1" "10.0.0.1" "172.16.0.1"
atomwrite --workspace . get Cargo.toml package.version
```


## Custom Agents
- Install: `cargo install atomwrite`
- Invoke via `std::process::Command`, `subprocess.run()`, or equivalent
- Parse stdout line by line as JSON objects
- Check exit codes for error classification
- Use the `retryable` field in error responses for automatic retry logic
- v0.1.12: handle 5 new exit codes (83, 88, 91, 92, 93) for lock timeout, syntax error, EXDEV disabled, copy-back BLAKE3 failed, orphan journal
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
