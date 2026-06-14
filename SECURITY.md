[Leia em Portugues](SECURITY.pt-BR.md)


# Security Policy


## Supported Versions
- Only the latest release receives security updates
- Upgrade to the latest version before reporting
- Older releases receive best-effort patches for critical vulnerabilities

| Version | Supported          |
|---------|--------------------|
| 0.1.15  | Yes                |
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
- Report security vulnerabilities privately via GitHub Security Advisories
- Navigate to the repository Security tab and select "Report a vulnerability"
- Do NOT open a public issue for security vulnerabilities
- Include: atomwrite version, OS, Rust version, description of the vulnerability, steps to reproduce, potential impact, proof of concept if available


## Response SLA
- Acknowledgment within 48 hours of report submission
- Initial assessment and severity classification within 5 business days
- Status updates at least every 7 days until resolution


## Fix SLA
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


## Known Security Advisories (Resolved in v0.1.12)

### RUSTSEC-2026-0009 in `time 0.3.45` (transitive via `tracing-appender`)
- **Status**: Resolved in v0.1.7 (2026-06-05)
- **Issue**: DoS via stack exhaustion in time parsing
- **Fix**: Upgraded `time` to 0.3.47 with `DEPTH_LIMIT=32`. The fix requires Rust 1.88
- **Action taken**: MSRV bumped from 1.85 to 1.88 in v0.1.7. The `ignore` entry in `deny.toml` and the `cargo audit --ignore` flag were both removed. Advisory no longer applies
- **Reference**: `CHANGELOG.md` v0.1.7 entry

### No active advisories in v0.1.18
- `cargo audit` reports 0 vulnerabilities across 640 crates
- `cargo deny check` reports 4/4 OK (advisories, bans, licenses, sources)
- All transitive dependencies with security notes have been either upgraded or replaced


## Dependency Security Posture (v0.1.18)
- **Memory safety**: 0 unsafe code blocks in `src/` (denied via `#![deny(unsafe_code)]` in lib root)
- **BLAKE3**: Used for checksums only, not for cryptographic security
- **tree-sitter-language-pack**: Parsers are downloaded on first use from the official `tree-sitter` GitHub releases via the `download` feature. The downloaded parsers are dynamically loaded but not executed as code
- **deny.toml**: Includes `MPL-2.0`, `CDLA-Permissive-2.0`, `CC0-1.0` in the allowlist. Has 10 skip entries for the unavoidable coexistence of `getrandom` 0.2/0.3, `rustix` 0.x/1.x, and `windows-sys` 0.52/0.59 across dependency trees
- **MSRV**: Rust 1.88 stable


## Hall of Fame
- Security researchers who report valid vulnerabilities are recognized here
- Report to be listed (or request anonymity)
- To request listing, open a GitHub Security Advisory with the report and include your preferred attribution


## Best Practices for Users
- Use `--workspace` to restrict operations to the project root
- Avoid running atomwrite as root
- Validate `--expect-checksum` in multi-agent environments
- Review NDJSON error output for `retryable` and `suggestion` fields
- Keep atomwrite updated to the latest version
- Audit the `batch` manifest before execution in production
- Subscribe to repository notifications for timely security updates
- Use `--strict-atomic` only when you understand the trade-off (forbids cross-device copy-fallback, exit 91 on EXDEV)
- Treat orphan journals (`.atomwrite.journal.*` sidecars from a previous crash) with suspicion: inspect the target file AND the journal content before deleting the journal
- When G72 syntax check is enabled (`--syntax-check`), do NOT pipe sensitive content through stdin on shared systems: the error envelope may echo source location and surrounding context

