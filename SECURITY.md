[Leia em Português](SECURITY.pt-BR.md)


# Security Policy


## Supported Versions
- The latest release receives security fixes, and older releases receive best-effort patches for critical vulnerabilities only
- Upgrade to the latest version before reporting, because a fix lands on the current line first
- The table lists the release line, and crates.io carries 0.1.37 while 0.1.38 is the version in preparation, so the pin in this document says which one is under test
- The versions 0.1.15, 0.1.16, 0.1.17, 0.1.21 and 0.1.33 were never published, so they are not listed

| Version | Supported          |
|---------|--------------------|
| 0.1.38  | Yes (in preparation) |
| 0.1.37  | Best-effort        |
| 0.1.36  | Best-effort        |
| 0.1.35  | Best-effort        |
| 0.1.34  | Best-effort        |
| 0.1.32  | Best-effort        |
| 0.1.31  | Best-effort        |
| 0.1.30  | Best-effort        |
| 0.1.29  | Best-effort        |
| 0.1.28  | Best-effort        |
| 0.1.27  | Best-effort        |
| 0.1.26  | Best-effort        |
| 0.1.25  | Best-effort        |
| 0.1.24  | Best-effort        |
| 0.1.23  | Best-effort        |
| 0.1.22  | Best-effort        |
| 0.1.20  | Best-effort        |
| 0.1.19  | Best-effort        |
| 0.1.18  | Best-effort        |
| 0.1.14  | Best-effort        |
| 0.1.13  | Best-effort        |
| 0.1.12  | Best-effort        |
| 0.1.11  | Best-effort        |
| 0.1.10  | Best-effort        |
| 0.1.9   | Best-effort        |
| 0.1.8   | Best-effort        |
| 0.1.7   | Best-effort        |
| 0.1.6   | Best-effort        |
| 0.1.5   | Best-effort        |
| 0.1.4   | Best-effort        |
| 0.1.3   | End of life        |
| 0.1.2   | End of life        |
| 0.1.1   | End of life        |
| 0.1.0   | End of life        |
| < 0.1.0 | Not released       |


## Reporting a Vulnerability
- Report security vulnerabilities privately by email to daniloaguiarbr@proton.me, which is the channel that works today
- GitHub Security Advisories through the repository Security tab is the second channel, available once the repository is published
- Do NOT open a public issue for security vulnerabilities
- Include: atomwrite version, OS, Rust version, description of the vulnerability, steps to reproduce, potential impact, proof of concept if available


## Response Targets
- These are best-effort targets of a single maintainer, not a contractual SLA: acknowledgment within 48 hours, severity classification within 5 business days and a status update every 7 days
- A report that misses a target is still handled, and the reporter is told which target was missed


## Fix Targets
- These are best-effort targets, and severity follows CVSS 3.1: Critical 9.0-10.0, High 7.0-8.9, Medium 4.0-6.9, Low below 4.0
- Critical severity: patch within 7 days
- High severity: patch within 14 days
- Medium severity: patch within 30 days
- Low severity: patch in the next scheduled release


## Disclosure Policy
- Coordinated disclosure: fix first, disclose after
- Reporter is credited unless they request anonymity
- Public disclosure occurs after the fix is published to crates.io
- Disclosure includes: CVE (if applicable), affected versions, fixed version, description, mitigation


## Security Update Policy
- Security patches are released as point releases (e.g., 0.1.13, 0.1.14)
- Announcements are posted via GitHub Security Advisories
- Users should subscribe to repository notifications for timely updates


## Known Security Advisories

### RUSTSEC-2026-0009 in `time 0.3.45` (transitive via `tracing-appender`)
- Status: resolved in v0.1.7 (2026-06-05)
- Issue: DoS via stack exhaustion in time parsing
- Fix: upgraded `time` to 0.3.47 with `DEPTH_LIMIT=32`, and the fix requires Rust 1.88
- Action taken: MSRV bumped from 1.85 to 1.88 in v0.1.7, the `ignore` entry in `deny.toml` and the `cargo audit --ignore` flag were both removed, and the advisory no longer applies
- Reference: `CHANGELOG.md` v0.1.7 entry

### BUG-SEC-001 — Symlink-directory escape from workspace jail (Fixed in v0.1.27)
- Severity: Critical
- Affected versions: v0.1.0 through v0.1.26
- Fixed in: v0.1.27
- Attack vector: create a symlink to a directory outside the workspace (`ln -s /tmp $WS/link`), then use atomwrite to write/read through the symlink (`atomwrite write link/escape.txt`)
- Root cause: `validate_path` in `path_safety.rs` used `soft_canonicalize` which resolved `.` and `..` but NOT symlinks in intermediate path components
- Impact: arbitrary file write and read outside the workspace jail (data exfiltration, arbitrary file creation)
- Affected subcommands: write, read, edit, edit-loop, apply, set, del, copy, move
- NOT affected: search, replace (use WalkBuilder from the `ignore` crate with own symlink verification)
- Fix: added `canonicalize_existing_prefix` that resolves symlinks via `std::fs::canonicalize` on the existing portion of the path before jail verification
- Mitigation: upgrade to v0.1.27 and do not create symlinks inside the workspace pointing outside

### v0.1.37 supply-chain audit (zip, quick-xml)
- The audit at v0.1.36 was not re-run before that release, and running it on the v0.1.37 tree found 7 vulnerabilities, 2 of them high (7.5) in `quick-xml` 0.37
- Both high ones were reachable from `extract-artifact` reading XML from an OOXML container: unbounded namespace allocation (`RUSTSEC-2026-0195`) and quadratic duplicate-attribute checking (`RUSTSEC-2026-0194`), fixed by moving to `quick-xml` 0.42
- `zip` 9.0.5 closes `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` and `RUSTSEC-2026-0369` (one of them critical, 9.1) in the OOXML container reader
- `crossbeam-epoch`, `rustls`, `anyhow` and `memmap2` were updated to their patched versions in the same pass
- Lesson recorded: run `cargo audit` and `cargo deny check` BEFORE the version bump, not after the docs are written

### Current state of the v0.1.38 tree
- `cargo audit` on `Cargo.lock` (566 crate dependencies, 1403 advisories in the local database, last updated 2026-11-03): exit 0, no vulnerability and no warning, measured on 2026-11-04
- `cargo deny check`: advisories ok, bans FAILED, licenses FAILED, sources ok, exit 6, measured on 2026-11-04 on this tree, and the failures are named in the section below
- The unmaintained-crate notices of the earlier audits, `instant` (`RUSTSEC-2024-0384`) and `unsafe-libyaml` (`RUSTSEC-2024-0449`), are gone: neither those two nor `serde_yaml` or `serde_yaml_ng` is in `Cargo.lock`, and `cargo tree --locked --all-features -i <crate>` answers "did not match any packages" for each
- The YAML rules of `transform` are read with `serde-saphyr` 1.3.0 and `notify` is on 8.x (ADR-0057 sections 4 and 5)
- Re-run both tools on the final tree before every release, the numbers above are a measurement of one day

### v0.1.38 dependency gate — FAILED, not published
- `cargo deny check` exited 6 on 2026-11-04, and the failing checks are `bans` and `licenses`
- `heic` 0.1.6 is a direct dependency of this crate and declares `AGPL-3.0-only OR LicenseRef-Imazen-Commercial`, so both halves of the expression are rejected by the `deny.toml` allowlist
- AGPL-3.0 is a copyleft licence, so publishing this tree to crates.io would put an AGPL dependency under every consumer of the binary
- The remaining two failures are duplicate entries the 33 `skip` rows do not carry: `fearless_simd` 0.7.0 next to 1.2.0 (through `hayro` -> `vello_cpu`) and the duplicate pairs of `syn` and `weezl`
- The gate stays red until `heic` is removed, replaced by a permissively licensed decoder, or the owner records a written exception with the reason, which the allowlist rule requires to be explicit

### OCR model download has no integrity check (open gap)
- `read --ocr` fetches two models from `huggingface.co/robertknight/ocrs` with a 64 MiB cap and a 120 second timeout, and neither a checksum nor a signature is verified before the file is renamed into the cache
- The tree-sitter bundle checks the `sha256` listed in its manifest; the OCR path has no equivalent, so a compromised host or a proxy in the middle can serve a different model and the product loads it in-process
- Mitigation today: pre-populate `$XDG_CACHE_HOME/atomwrite/models/` from a trusted machine, and keep that directory writable by your user only
- The fix is to pin one `sha256` per model in `src/constants.rs` and refuse a file that does not match, which is open work and is not part of 0.1.38

### Content-source paths and workspace jail (policy, not a vulnerability)
- `--old-file` / `--new-file` content-source paths remain under the workspace jail (no OS `/tmp` allowlist)
- Agents MUST stage patch bodies via `atomwrite scratch init .atomwrite/scratch` under `$workspace/.atomwrite/scratch/` then pass those jail-safe paths
- A `WORKSPACE_JAIL` (exit 126) on `/tmp/...` old/new files is intended policy (v0.1.36 / ADR-0055), not a security hole to bypass


## Dependency Security Posture (v0.1.38)
- Memory safety: `unsafe` is denied crate-wide (`#![deny(unsafe_code)]` in `src/lib.rs` and `src/main.rs`) and allowed only in `src/signal.rs` and `src/platform.rs` (`#![allow(unsafe_code)]` at line 5 of each), where 8 `unsafe` blocks wrap `signal-hook` registration and libc calls (`_exit`, `poll`, `write` to stderr, `fcntl` on macOS, `renameat2` on Linux, console setup on Windows)
- The 8 blocks are at `src/signal.rs:260`, `src/signal_handlers.inc.rs:31,143,171,219` (included from `src/signal.rs`) and `src/platform.rs:97,203,275`, counted with `rg 'unsafe\s*\{' src`, which returns 9 matches because `src/commands/scope/queries.inc.rs:77` holds the literal text of a query pattern, and each block carries a `SAFETY:` comment
- BLAKE3: used for checksums only, not for cryptographic security
- tree-sitter-language-pack 1.22.1: parsers are downloaded on first use from the release assets of `github.com/xberg-io/tree-sitter-language-pack` through the `download` feature (HTTP client `ureq` with `rustls` and `ring`, all in the default binary)
- The client checks every bundle against the `sha256` listed in the manifest fetched from the same release (`download.rs` of the crate), and a bundle that fails the check is rejected
- The downloaded parsers are native libraries loaded into the process (`dynamic-loading`), so they run as code: keep the cache directory (`$XDG_CACHE_HOME/tree-sitter-language-pack/`) writable only by your user
- To run with no download, pre-populate that cache or build without `ast`, see `PRIVACY.md`
- deny.toml: includes `MPL-2.0`, `CDLA-Permissive-2.0`, `CC0-1.0` in the allowlist, and has 33 `skip` entries, each with a reason, for the unavoidable coexistence of duplicate crate versions (`getrandom` 0.2/0.3, `rustix` 0.x/1.x, `windows-sys` 0.52/0.59 and the Windows crates of the opt-in `watch` feature)
- MSRV: Rust 1.90 stable (`rust-version` in `Cargo.toml`, raised from 1.89 in v0.1.38, see ADR-0075)


## Hall of Fame
- Security researchers who report valid vulnerabilities are recognized here
- Report to be listed (or request anonymity)
- To request listing, open a GitHub Security Advisory with the report and include your preferred attribution


## Best Practices for Users
- Use `--workspace` to restrict operations to the project root
- Avoid running atomwrite as root
- Validate `--expect-checksum` in multi-agent environments
- Review NDJSON error output for `ok: false`, `retryable`, and `suggestion` fields (exit-first parse)
- A bounded `read` answers `truncated: true` and a `next` range, so never treat a truncated read as the whole file, because content past the cap stays invisible to the review
- On network filesystems `flock` can be silently ignored, so keep `--expect-checksum` as the extra net there, and remember the sidecar is best effort while the inode lock carries a read-only directory
- Keep atomwrite updated to the latest version
- Audit the `batch` manifest before execution in production
- Subscribe to repository notifications for timely security updates
- Treat orphan journals (`.atomwrite.journal.*` sidecars from a previous crash) with suspicion: inspect the target file AND the journal content before deleting the journal
- When `--syntax-check` is enabled, do NOT pipe sensitive content through stdin on shared systems: the error envelope may echo source location and surrounding context
- Stage agent patches with `scratch init` + `--old-file`/`--new-file` under the workspace, and do not expect OS `/tmp` to be jail-exempt
- A crash report (`report-<uuid>.toml` in the OS temporary directory, written by the `human-panic` hook and never transmitted) can hold paths and a backtrace, so review it before you attach it to an issue
