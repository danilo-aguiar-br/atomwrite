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
- Rust 1.88 or later (edition 2024)
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
- Run the full suite before submitting: `cargo test`


## Documentation
- Update the README when adding or changing commands
- Update AGENTS.md when modifying the output contract or exit codes
- Add doc comments to all public functions and types
- Keep code examples in docs tested and up to date


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
- Changelog updated before each release
- Tags follow the format `vX.Y.Z`
- Published to crates.io after CI passes


## Recognition
- Contributors are recognized in the changelog and release notes
- Significant contributions are acknowledged in the repository


## Quality Gates
- Run `cargo fmt --check` before committing
- Run `cargo clippy --all-targets -- -D warnings` for lint checks
- Run `cargo test` for the full test suite
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


## Questions
- Open a GitHub Discussion for general questions
- Open an issue for bugs and feature requests
- Be respectful and constructive in all interactions
