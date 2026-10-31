[Leia em Português](PRIVACY.pt-BR.md)


# Privacy Policy — atomwrite


- Last updated: 2026-10-31 (atomwrite 0.1.37)


## Summary
- `atomwrite` is a local CLI
- It has no telemetry, no analytics and no phone-home: nothing is collected, so there is no opt-in switch to turn on (an explicit opt-in would be required before any telemetry could be added)
- It never uploads the contents of your files
- The only network traffic the product starts itself is the one-time download of tree-sitter parsers, described in [Network Access](#network-access)


## What Runs Locally
- All file reads, writes, searches, backups and WAL sidecars execute on the machine that invokes the binary
- Configuration is CLI flags + XDG directories + `.atomwrite.toml` only, with no product `ATOMWRITE_*` runtime knobs (policy since v0.1.35, current line `0.1.37`)
- The product code calls no `env::var`, and a test guards it
- The OS crates `sys-locale` and `directories` read `LANG` (and `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` on Unix), `HOME` and `XDG_*` for you, see [Environment Reads By Dependencies](#environment-reads-by-dependencies)
- `doctor` reads no environment variable: host detection uses local file probes only
- Agent staging directories under `.atomwrite/scratch/` are local workspace files, never uploaded
- `atomwrite config set` and `config unset` write only the XDG `config.toml` (mode `0600` on Unix) or the workspace `.atomwrite.toml`, atomically, and `config list` and `config get` only read them
- The `watch`, `semantic-search` and `semantic-merge` verbs work on local filesystem data only


## Network Access
- Core read, write, edit, search, replace and every other verb need no network
- The AST features need the tree-sitter parsers, and the build ships none of them: `outline`, `query` and `--syntax-check` download the parser bundle the first time a language is used and the local cache has no copy
- The download comes from `github.com/xberg-io/tree-sitter-language-pack` (release assets), through the `ureq` HTTP client of the `tree-sitter-language-pack` 1.22.1 crate (`ureq`, `rustls` and `ring` are in the default binary)
- Measured on 2026-10-31 in the `v1.22.1` manifest: about 22 MiB (23,031,807 bytes for `linux-x86_64`), written to `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` (`~/.cache/tree-sitter-language-pack/v1.22.1/` by default), and the six parsers this machine already extracted take 3.9 MiB
- The request carries no data from your files, the HTTP client adds its own standard request headers (not inspected for this document), and the host that serves the file sees the IP address of your machine, as with any download
- Later runs reuse the cache and make no request
- How to avoid the download: copy a warm cache directory from a connected machine to the same path, build without `ast` (`cargo install atomwrite --no-default-features --features core`), or do not use `outline`, `query` and `--syntax-check`
- `transform` uses the grammars compiled into `ast-grep-language` and does not use this download (measured in ADR-0057)
- `cargo install atomwrite` downloads crates from crates.io, and that traffic is between your host and crates.io


## Environment Reads By Dependencies
- Third-party code reads the process environment in three places, listed in ADR-0057 so that no reader finds them by surprise
- `sys-locale` reads `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG` on Unix to detect the OS language for the UI
- `directories` reads `HOME` and `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME` on Linux to place the config, state and data paths
- `tree-sitter-language-pack` reads `HOME` and `XDG_CACHE_HOME` for the parser cache, and `ureq` reads `ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY` and `NO_PROXY` (with lowercase variants) when it downloads a parser
- None of these values is sent anywhere by atomwrite


## Files atomwrite Writes
- Config: `config.toml` under `$XDG_CONFIG_HOME/atomwrite/` (`~/.config/atomwrite/` on Linux), mode `0600` on Unix, written only by `config set` and `config unset`
- Workspace config: `.atomwrite.toml` at the workspace root, written only by `config set --scope workspace`
- Locale preference: the single-line file `locale` in the same directory, mode `0600` when the platform allows, written only by `locale --set`
- Logs: only when `log.file` is `true` (default `false`), JSON lines under `$XDG_STATE_HOME/atomwrite/logs/atomwrite.<date>.jsonl`, rotated daily by default (`log.rotation`) and capped at 7 files
- Log lines hold diagnostic events and can include the file paths and the locale of the run, so treat them as sensitive when the paths are
- WAL journals: `.atomwrite.journal.<target>.atomwrite.journal.json` next to the target file, removed after a committed write and by `wal-heal`
- Backups: `<file>.bak.<timestamp>` next to the file, created by the mutating verbs and removed after a successful write by default (`delete` and `replace` keep them, measured on 2026-10-31, and `prune-backups` removes old ones)
- Lock sidecars (`.<target>.atomwrite.lock`) and temporary files next to the target during a write
- The lock sidecar keeps the literal name until `.<target>.atomwrite.lock` would pass 255 bytes, and past that it becomes a short BLAKE3 digest in the same directory, so the name holds no path beyond the target name
- Staging: `.atomwrite/scratch/` in the workspace, created by `scratch init`
- Shell completion scripts when you run `atomwrite completions --install`
- Crash reports: the `human-panic` hook writes a `report-<uuid>.toml` file in the OS temporary directory when the program panics, with the OS name, the crate version, the panic message and a backtrace, and it can include paths that appear in them
- The crash report is a local file only and is never transmitted, you decide whether to attach it to an issue
- The parser cache described in [Network Access](#network-access)


## What Is Never Collected
- No analytics, crash-reporting service or usage beacon is embedded in the binary
- NDJSON output can contain file paths and content excerpts that you (or an agent) chose to process, so treat stdout as sensitive if those paths hold secrets


## Personal Data And Locale
- The detected or chosen locale is treated as potentially personal data
- Minimization: only a language tag (`en` or `pt-BR`) is stored, as a single line in the `locale` file
- Retention: the file stays on your disk until you remove it, and atomwrite never copies it elsewhere
- Deletion: run `atomwrite locale --clear`, or delete the file at the `preference_path` that `atomwrite locale` prints
- The OS locale is read once per run and is not sent off the host
- Because nothing is sent off the host, atomwrite does not collect personal data in the sense of the GDPR (EU) or the LGPD (Brazil), and this is a description of the design, not legal advice


## Third-Party Dependencies
- Upstream crates are governed by their own licenses (see `deny.toml` / `cargo deny`)
- If you enable tooling that calls external services (unrelated wrappers, agent hosts), those systems have separate privacy policies


## Contact
- Maintainer: see `Cargo.toml` `authors` / repository listed in the package metadata
