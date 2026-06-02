[Leia em Portugues](SECURITY.pt-BR.md)


# Security Policy


## Supported Versions
- Only the latest release receives security updates
- Upgrade to the latest version before reporting

| Version | Supported          |
|---------|--------------------|
| 0.1.2   | Yes                |
| 0.1.1   | Yes                |
| 0.1.0   | No (upgrade)       |
| < 0.1.0 | No (pre-release)   |


## Reporting a Vulnerability
- Report security vulnerabilities privately via GitHub Security Advisories
- Navigate to the repository Security tab and select "Report a vulnerability"
- Do NOT open a public issue for security vulnerabilities
- Include: atomwrite version, OS, description of the vulnerability, steps to reproduce, potential impact


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
- Security patches are released as point releases (e.g., 0.1.2)
- Announcements are posted via GitHub Security Advisories
- Users should subscribe to repository notifications for timely updates

## Known Security Advisories (v0.1.2)
- **RUSTSEC-2026-0009** in `time 0.3.45` (transitive via `tracing-appender`): DoS via stack exhaustion in time parsing. The fix requires `time >= 0.3.47` which needs Rust 1.88. Our MSRV is 1.85, and atomwrite uses `time` only via `tracing-appender` for log timestamps — not exploitable via user input. Acknowledged in `deny.toml` and tracked for MSRV bump in v0.2.0.


## Hall of Fame
- Security researchers who report valid vulnerabilities are recognized here
- Report to be listed (or request anonymity)


## Best Practices
- Use `--workspace` to restrict operations to the project root
- Avoid running atomwrite as root
- Validate `--expect-checksum` in multi-agent environments
- Review NDJSON error output for `retryable` and `suggestion` fields
- Keep atomwrite updated to the latest version
- Audit the `batch` manifest before execution in production
