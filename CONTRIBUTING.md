[Leia em Portugues](CONTRIBUTING.pt-BR.md)


# Contributing to atomwrite


## Welcome
- Thank you for considering a contribution to atomwrite
- Every contribution matters: code, tests, docs, bug reports, feature ideas
- This guide helps you get started quickly


## Quick Start
- Fork the repository on GitHub
- Clone your fork locally
- Create a feature branch from `main`
- Make your changes
- Run the test suite
- Open a pull request


## Development Setup
### Prerequisites
- Rust 1.88 or later (edition 2024) — MSRV bumped in v0.1.7
- Git

### Build
```bash
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo build
```

### Run Tests
```bash
cargo test
```

### Run Linter
```bash
cargo clippy -- -D warnings
```

### Check Formatting
```bash
cargo fmt -- --check
```


## Branching
- Create branches from `main`
- Use descriptive branch names: `feat/batch-parallel`, `fix/checksum-race`, `docs/readme-update`
- Keep branches short-lived and focused on a single change


## Commit Convention
- Use present tense imperative mood: "add batch support", not "added batch support"
- Keep the first line under 72 characters
- Reference issue numbers when applicable: `fix search exit code (#42)`
- One logical change per commit


## PR Process
- Fill in the PR template with a clear description
- Link related issues
- Ensure all CI checks pass before requesting review
- Keep PRs focused: one feature or fix per PR
- Respond to review feedback promptly
- Squash commits when requested by maintainers


## Testing
- Write tests for every new feature and bug fix
- Place unit tests in the same file as the code under `#[cfg(test)]`
- Place integration tests in the `tests/` directory
- Use `assert_cmd` and `predicates` for CLI integration tests
- Use `insta` for snapshot testing of NDJSON output
- Use `proptest` for property-based testing where applicable
- Target at least 80% coverage for new code
- Run the full suite before submitting: `cargo test` (502 tests in v0.1.18)


## Documentation
- Update the README when adding or changing commands
- Update AGENTS.md when modifying the output contract or exit codes
- Update CHANGELOG.md (English) and CHANGELOG.pt-BR.md (Portuguese) for any user-visible change
- For non-trivial architecture decisions, add an ADR in `docs/decisions/` following the Michael Nygard format (Status, Context, Decision, Consequences, Alternatives, Trigger to revisit). See `docs/decisions/README.md` for the index
- For new NDJSON output envelopes, add a JSON Schema in `docs/schemas/` (versioned per release)
- Add doc comments to all public functions and types
- Keep code examples in docs tested and up to date


## Architecture Decision Records (ADRs)
- atomwrite uses ADRs in `docs/decisions/` to document non-trivial design choices
- 12 ADRs have been added since v0.1.12 (0019-0030), all following the Michael Nygard format
- Each new architecture decision should add a new ADR file and update `docs/decisions/README.md`
- ADRs are NOT updated once written — instead, supersede with a new ADR


## Adding a New Subcommand
- Add a module under `src/commands/your_subcommand.rs`
- Register the subcommand in `src/commands/mod.rs`
- Define the argument struct in `src/cli_args.rs` with `clap` derives
- Add the dispatch arm in `src/lib.rs`
- Add an entry to `Commands` enum in `src/cli.rs`
- Add a corresponding JSON Schema in `docs/schemas/`
- Add the subcommand to the README and llms.txt inventories
- Write at least 3 integration tests in `tests/cli_your_subcommand.rs`
- Update llms-full.txt to reference the new subcommand in the right category
- v0.1.18 has 30 subcommands; the count must stay in sync across all docs


## Report Bugs
- Open an issue on GitHub with the `bug` label
- Include: atomwrite version, OS, Rust version, steps to reproduce, expected vs actual behavior
- Include the full NDJSON error output when applicable
- Minimal reproduction cases are greatly appreciated


## Request Features
- Open an issue on GitHub with the `enhancement` label
- Describe the problem the feature solves
- Describe the expected behavior
- Consider how it fits the NDJSON output contract


## Release Process
- Maintainers handle releases
- Version follows Semantic Versioning 2.0.0
- Changelog updated before each release (both EN and PT-BR)
- Tags follow the format `vX.Y.Z`
- Published to crates.io after CI passes
- v0.1.12 was published on 2026-06-07 with commit 6af0d76. Note: v0.1.15, v0.1.16, v0.1.17, and v0.1.18 are unreleased as of 2026-06-14. The next crates.io publish will be v0.1.18 after the consolidated 3-agent P2 cleanup is committed and tagged.


## Recognition
- Contributors are recognized in the changelog and release notes
- Significant contributions are acknowledged in the repository


## Quality Gates
- Run `cargo fmt --check` before committing
- Run `cargo clippy --all-targets -- -D warnings` for lint checks
- Run `cargo test` for the full test suite (502 tests in v0.1.18)
- Run `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` for documentation checks
- Run `cargo audit` for security advisories
- Run `cargo deny check` for license and dependency policy (see `deny.toml`)
- Run `cargo check --all-features` against the MSRV (Rust 1.88) for toolchain compatibility
- Run `cargo package --no-verify --list` and `cargo publish --dry-run --allow-dirty` to validate release artifacts

## Cross-Platform Validation (added in v0.1.4)
- Install Windows targets: `rustup target add x86_64-pc-windows-gnu` and `i686-pc-windows-gnu`
- On Linux, install mingw: `mingw64-gcc` (Fedora) or `mingw-w64` (Ubuntu)
- Run the cross-compile gate: `cargo test --test cross_compile_check -- --ignored`
- The gate fails on any `E0433`, `E0308`, or `E0507` regression in `#[cfg(windows)]` blocks
- Required for any change that touches `src/atomic.rs`, `src/platform.rs`, `src/signal.rs`, or other Windows-only code
- The gate is a defense against the GAP 14 regression: `cargo install atomwrite` was broken on Windows 10/11 in v0.1.3 because three Windows-only compile errors were not caught by the Linux-only CI

## v0.1.12 Specific Gates
- If you add a new subcommand, update the subcommand count in ALL of: `README.md`, `README.pt-BR.md`, `llms.txt`, `llms.pt-BR.txt`, `llms-full.txt`, `docs/AGENTS.md`, `docs/AGENTS.pt-BR.md`, `docs/MIGRATION.md`, `docs/MIGRATION.pt-BR.md`, `CHANGELOG.md`, `CHANGELOG.pt-BR.md`, `skill/atomwrite-en/SKILL.md`, `skill/atomwrite-pt/SKILL.md`
- If you add a new error variant, update the exit codes in: `README.md`, `README.pt-BR.md`, `llms-full.txt`, `docs/AGENTS.md`, `docs/AGENTS.pt-BR.md`, `skill/atomwrite-en/SKILL.md`, `skill/atomwrite-pt/SKILL.md`, `locales/en.toml`, `locales/pt-BR.toml`
- The single source of truth for the subcommand count is the binary: `atomwrite --help | rg "^  [a-z]" | wc -l` (currently 29 in v0.1.12 = 28 user-facing + `help`)


## Questions
- Open a GitHub Discussion for general questions
- Open an issue for bugs and feature requests
- Be respectful and constructive in all interactions
