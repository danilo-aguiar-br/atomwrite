[Leia em Portugues](INTEGRATIONS.pt-BR.md)


# Integrations

> atomwrite works with every LLM agent that can execute shell commands


## Compatible Agents
- atomwrite requires only `bash` access to function
- Any agent that can run a shell command can use atomwrite
- NDJSON output is parseable by every major LLM without custom adapters
- No plugins, extensions, or SDKs required
- As of v0.1.4, atomwrite runs on Windows 10/11, Linux, and macOS with identical NDJSON contract


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
```bash
# Example CLAUDE.md entry
# Use atomwrite for all file operations
echo "content" | atomwrite write src/file.rs
atomwrite read src/file.rs
atomwrite search 'pattern' src/
```


## Cursor
- Install: `cargo install atomwrite`
- Cursor executes terminal commands via its built-in shell
- NDJSON responses integrate directly with tool-use flows
- Use `--dry-run` for preview before destructive operations
```bash
atomwrite search 'TODO' src/ --include '*.rs'
atomwrite replace 'old_api' 'new_api' src/
```


## Windsurf
- Install: `cargo install atomwrite`
- Windsurf runs shell commands through its terminal integration
- Structured output reduces token consumption compared to raw CLI tools
- Batch operations minimize the number of tool calls
```bash
cat manifest.ndjson | atomwrite batch
```


## Codex CLI
- Install: `cargo install atomwrite`
- Codex CLI executes commands in a sandboxed shell
- atomwrite respects sandbox boundaries via `--workspace`
- Checksums enable state verification across execution steps
```bash
atomwrite read src/main.rs
atomwrite hash src/main.rs
```


## Aider
- Install: `cargo install atomwrite`
- Aider runs shell commands for code editing workflows
- atomwrite provides atomic guarantees that shell built-ins lack
- Use `edit` for surgical changes instead of full file rewrites
```bash
atomwrite edit src/lib.rs --old "old code" --new "updated code"
```


## Continue
- Install: `cargo install atomwrite`
- Continue executes terminal commands as part of its agent loop
- NDJSON output is machine-readable without post-processing
```bash
atomwrite search 'deprecated' src/ --include '*.rs'
```


## Cline
- Install: `cargo install atomwrite`
- Cline uses shell commands for file operations
- atomwrite replaces fragile sed/awk pipelines with atomic operations
```bash
echo "new content" | atomwrite write src/config.rs
atomwrite diff src/old.rs src/new.rs
```


## Roo Code
- Install: `cargo install atomwrite`
- Roo Code runs bash commands in its agent execution environment
- Structured errors with `retryable` and `suggestion` fields guide automatic recovery
```bash
atomwrite copy src/template.rs src/new.rs
```


## Amazon Q Developer
- Install: `cargo install atomwrite`
- Amazon Q executes CLI commands in its development environment
- atomwrite provides consistent cross-platform behavior
```bash
atomwrite list src/ --depth 3
atomwrite count src/ --by-extension
```


## GitHub Copilot CLI
- Install: `cargo install atomwrite`
- Copilot CLI suggests and executes shell commands
- atomwrite commands are self-documenting via `--help` on each subcommand
```bash
atomwrite calc "100 MB to bytes"
atomwrite regex "192.168.1.1" "10.0.0.1" "172.16.0.1"
```


## Custom Agents
- Install: `cargo install atomwrite`
- Invoke via `std::process::Command`, `subprocess.run()`, or equivalent
- Parse stdout line by line as JSON objects
- Check exit codes for error classification
- Use the `retryable` field in error responses for automatic retry logic
```python
import subprocess
import json

result = subprocess.run(
    ["atomwrite", "read", "src/main.rs"],
    capture_output=True, text=True
)
for line in result.stdout.strip().split("\n"):
    data = json.loads(line)
    print(data["type"], data.get("path"))
```
