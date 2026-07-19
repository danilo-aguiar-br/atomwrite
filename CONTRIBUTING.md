[Leia em Portugues](CONTRIBUTING.pt-BR.md)


# Contributing to atomwrite

> Contribute to the atomic CLI agents trust for durable file edits


## Welcome
- Thank you for considering a contribution to atomwrite
- atomwrite is a Rust CLI for atomic file operations and NDJSON agent contracts
- Every contribution matters: code, tests, docs, bug reports, and feature ideas
- Read this guide before opening a pull request
- Follow the [Code of Conduct](CODE_OF_CONDUCT.md) in every interaction


## Quick Start
- Fork the repository on GitHub
- Clone your fork locally
- Create a feature branch from `main`
- Make a focused change
- Run the local definition of done (DoD) commands below
- Open a pull request against `main`


## Development Setup
### Prerequisites
- Install Rust 1.88 or later (edition 2024; MSRV declared in `Cargo.toml`)
- Install Git
- Install `rustfmt` and `clippy` via the active toolchain (`rust-toolchain.toml`)

### Build
```bash
git clone https://github.com/danilo-aguiar-br/atomwrite.git
cd atomwrite
cargo build
# slim core only
cargo build --release --no-default-features --features core
# full surface
cargo build --release --features full
```

### Local definition of done
- This CLI product does not ship product GitHub Actions workflows as a release gate
- Validate locally before every PR and release
- Run library and integration tests: `cargo test --lib --tests`
- Run clippy with warnings denied: `cargo clippy --all-targets -- -D warnings`
- Check Windows cross-compile when touching platform code: `cargo check --target x86_64-pc-windows-gnu`
- Install from path after substantive changes: `cargo install --path . --force`
- Optional contract suite for residual agent surface: `cargo test --test cli_e2e_v0135`
- Optional slim surface: `cargo test --no-default-features --features core`
- Optional format check: `cargo fmt -- --check`
- Optional docs lint: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- Optional supply-chain checks: `cargo audit` and `cargo deny check`

### Current release pin
- Current public version is `0.1.35`
- Pin consumers and agents to `^0.1.35`
- Keep version strings consistent across `Cargo.toml`, README, CHANGELOG, and skills


## Branching Strategy
- Branch from `main`
- Prefer short-lived, single-purpose branches
- Name branches with intent: `feat/watch-summary`, `fix/ack-overwrite`, `docs/contributing`
- Avoid mixing unrelated refactors with feature work


## Commit Convention
- Write the subject in present-tense imperative mood: `add watch_summary on idle`
- Keep the first line under 72 characters
- Reference related issues when applicable: `fix fuzzy one-pass hang (#123)`
- Keep one logical change per commit when practical
- Prefer Conventional Commit prefixes when useful: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`


## PR Process
- Describe the problem, the change, and how you validated it
- Link related issues
- Keep each PR focused on one feature, fix, or docs change
- Update bilingual documentation in the same PR when user-visible behavior changes
- Run the local DoD before requesting review
- Respond to review feedback promptly
- Squash commits when maintainers request it


## Testing
- Add tests for every new feature and bug fix
- Place unit tests next to the code under `#[cfg(test)]` when appropriate
- Place CLI integration tests under `tests/`
- Prefer `assert_cmd` and `predicates` for CLI assertions
- Prefer `insta` for NDJSON snapshot contracts
- Prefer `proptest` for property-based coverage where it fits
- Run `cargo test --lib --tests` before submitting
- Run targeted suites for the surface you touch, for example:
  - `cargo test --test cli_e2e_v0135`
  - `cargo test --test cli_v0130_agent_contract`
  - `cargo test --test cli_v0133_oneshot_fuzzy`
- See [docs/TESTING.md](docs/TESTING.md) for categories and profiles
- Do not invent or paste unmeasured pass counts into docs


## Documentation
- Keep English as the canonical public language
- Mirror every public doc change in the matching `.pt-BR` file in the same delivery
- Open public docs with a cross-link to the opposite language
- Update `README.md` and `README.pt-BR.md` when commands or install guidance change
- Update `docs/AGENTS.md` and `docs/AGENTS.pt-BR.md` when the agent contract changes
- Update `CHANGELOG.md` and `CHANGELOG.pt-BR.md` for every user-visible change
- Follow Keep a Changelog and Semantic Versioning
- Update skills under `skills/atomwrite-en/` and `skills/atomwrite-pt/` when operational contracts change
- Add or update JSON schemas under `docs/schemas/` for new NDJSON envelopes
- Add an ADR under `docs/decisions/` for non-trivial design choices
- Update `llms.txt`, `llms.pt-BR.txt`, and `llms-full.txt` when primary docs change
- Note: `gaps.md` is local audit notes and is not public packaging documentation
- Exclude private notes such as `docs_rules/`, `gaps.md`, and local agent memory from release packaging expectations


## Report Bugs
- Open a GitHub issue with a clear reproduction
- Include atomwrite version, OS, Rust version, exact command, expected result, and actual result
- Attach full NDJSON error output when available
- Prefer a minimal reproduction over a large workspace dump
- Report security issues privately through [SECURITY.md](SECURITY.md), not public issues


## Request Features
- Open a GitHub issue describing the user problem first
- Explain the expected CLI behavior and NDJSON contract impact
- Consider exit codes, workspace safety, atomic write guarantees, and bilingual docs
- Prefer features that keep atomwrite a one-shot non-interactive CLI


## Release Process
- Maintainers own releases
- Versioning follows Semantic Versioning 2.0.0
- Changelog entries follow Keep a Changelog in both EN and pt-BR
- Tags use the form `vX.Y.Z`
- Validate with the local DoD; this product does not rely on product GitHub Actions as a release gate
- Suggested pre-publish checks:
  - `cargo test --lib --tests`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo check --target x86_64-pc-windows-gnu`
  - `cargo package --list` / dry-run publish as needed
  - `cargo install --path . --force`
- Publish to crates.io only after local DoD is green
- Current release line is `0.1.35`; consumers should pin `^0.1.35`


## Recognition
- Contributors are credited in changelog entries and release notes when appropriate
- Significant contributions may be acknowledged in repository docs
- Security researchers are credited according to [SECURITY.md](SECURITY.md)


## Questions
- Open a GitHub Discussion for general questions when available
- Open an issue for concrete bugs and feature requests
- Contact the maintainer at daniloaguiarbr@proton.me for private contribution questions
- Be respectful and constructive in every interaction
