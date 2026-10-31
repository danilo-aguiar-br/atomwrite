[Leia em Português](CONTRIBUTING.pt-BR.md)


# Contributing to atomwrite


- Contribute to the atomic CLI agents trust for durable file edits


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
- Install Rust 1.90 or later (edition 2024; MSRV declared in `Cargo.toml` as `rust-version = "1.90"`)
- The local `rust-toolchain.toml` selects channel 1.94 with `rustfmt` and `clippy`, and the MSRV stays 1.90
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

### MSRV Policy
- The MSRV is the `rust-version` field of `Cargo.toml` (1.90 today, raised from 1.89 because `ast-grep-core` 0.51 and `tree-sitter` 0.27 require it, see ADR-0075)
- Raising the MSRV is a minor-version change, never a patch, and the CHANGELOG lists it
- Keep `README.md`, `docs/INSTALL.md` and `docs/CROSS_PLATFORM.md` in step with `Cargo.toml` when it changes

### Local definition of done
- Every gate runs locally on your machine, there is no remote automation
- Validate locally before every PR and release
- Run library and integration tests with the default features: `cargo test --lib --tests`
- Run clippy with warnings denied: `cargo clippy --all-targets -- -D warnings`
- Check Windows cross-compile when touching platform code: `cargo check --target x86_64-pc-windows-gnu`
- Install from path after substantive changes: `cargo install --path . --force`
- Optional contract suite for residual agent surface: `cargo test --test cli_e2e_v0135`
- Optional v0.1.36 agent-contract suite: `cargo test --test cli_v0136_gaps`
- Optional v0.1.37 agent-contract suites: `cargo test --test cli_v0137_envelope --test cli_v0137_dominio --test cli_v0137_agent_native --test cli_v0137_artefato --test cli_v0137_etd --test cli_v0137_cancel`
- The status of a gap lives in the ADR that owns it; local audit notes are never a release gate
- Slim profile: `cargo test --no-default-features --features core` is green in 131 test targets, because every test that exercises `scope`, `transform`, `outline` or `query` carries `#[cfg(feature = "ast")]` and is skipped in that build
- To prove the slim build compiles, run `cargo xtask check-features` (it runs `cargo check --locked --no-default-features` for the `core` and `core,watch` sets and stops at the first failure), and run the test suite with the default features
- Optional v0.1.37 suites for the `config` verb and the source guards: `cargo test --test cli_v0137_config_verb --test cli_v0137_hardcode_guard --test cli_v0137_retry_guards --test cli_v0137_let_underscore_guard --test cli_v0137_terminal_event`
- Optional documentation suites: `cargo test --test docs_consistency --test docs_exit_codes`
- Format check: `cargo fmt -- --check`
- Docs lint: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- Supply-chain checks, run them before the version bump and not after the docs are written: `cargo audit` and `cargo deny check`

### Current release pin
- The release line in this tree is `0.1.37`, which is still in preparation, and crates.io carries `0.1.36` until `0.1.37` is published
- Pin consumers and agents to `^0.1.36` today and to `^0.1.37` when that version lands
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

### Pull request checklist
- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --lib --tests` passes with the default features
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` passes
- [ ] `cargo audit` and `cargo deny check` pass
- [ ] README, CHANGELOG and both `.pt-BR` mirrors are updated
- [ ] NDJSON schemas and the ADR are updated when the contract changes
- [ ] No unmeasured number was added to the docs


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

### Hermetic tests and source guards
- Every test that runs the binary goes through `common::hermetic(...)` (`tests/common/mod.rs`), which gives the child an empty `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME` and `APPDATA` and fixes `LANG` to `en_US.UTF-8`
- The XDG cache stays real on purpose, so the AST parsers are not downloaded again for every test
- A test that needs its own config redirects `HOME` and `XDG_*` after calling `hermetic`
- The source guards fail the build: `tests/cli_v0137_hardcode_guard.rs` forbids duration literals, the `.bak.` infix, sidecar names and time-unit numbers outside `constants*`, plus `env::var` and `Command::new` anywhere in production code, and forbids any `.sh` file and any `.github` directory
- `tests/cli_v0137_let_underscore_guard.rs` forbids `let _ =`, `.unwrap()`, `.expect(`, `unreachable!`, `panic!`, `todo!` and `unimplemented!` in production code
- Both guards keep their allowance lists empty, so new code must satisfy them instead of adding an exception


## Documentation
- Keep English as the canonical public language
- Mirror every public doc change in the matching `.pt-BR` file in the same delivery
- Open public docs with a cross-link to the opposite language
- Update `README.md` and `README.pt-BR.md` when commands or install guidance change
- Update `docs/AGENTS.md` and `docs/AGENTS.pt-BR.md` when the agent contract changes
- Update `CHANGELOG.md` and `CHANGELOG.pt-BR.md` for every user-visible change
- Translations: pt-BR is the only translation, written with all accents, and the UI strings live in `locales/en.toml` and `locales/pt-BR.toml` with key parity enforced by `tests/locale_parity.rs`
- Follow Keep a Changelog and Semantic Versioning
- Update skills under `skills/atomwrite-en/` and `skills/atomwrite-pt/` when operational contracts change
- Add or update JSON schemas under `docs/schemas/` for new NDJSON envelopes
- Add an ADR under `docs/decisions/` for non-trivial design choices
- Update `llms.txt`, `llms.pt-BR.txt`, and `llms-full.txt` when primary docs change
- When the CLI surface changes: bump `CANONICAL_SUBCOMMAND_COUNT`, the 47-row inventories in README/AGENTS/`llms*`, and `docs/schemas/README.md`
- When agent contract gaps close: update the status in the ADR that owns the gap, HOW_TO_USE, COOKBOOK, MIGRATION, TESTING, and both skills in the same delivery
- Note: the gap inventory is a local unversioned note, so the current status lives in the ADR series and never in that note
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
- Validate with the local DoD, every gate runs locally
- Suggested pre-publish checks:
  - `cargo test --lib --tests`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo check --target x86_64-pc-windows-gnu`
  - `cargo xtask check-features`
  - `cargo audit` and `cargo deny check`
  - `cargo package --list` / dry-run publish as needed
  - `cargo install --path . --force`
- Publish to crates.io only after local DoD is green, and crates.io still carries `0.1.36` until the `0.1.37` publish lands
- Current release line is `0.1.37`; consumers should pin `^0.1.36` today and `^0.1.37` once it is published


## Code of Conduct
- Every contributor follows the [Code of Conduct](CODE_OF_CONDUCT.md)
- Report violations as described in the Enforcement section of that document


## Recognition
- Contributors are credited in changelog entries and release notes when appropriate
- Significant contributions may be acknowledged in repository docs
- Security researchers are credited according to [SECURITY.md](SECURITY.md)


## Questions
- Open a GitHub Discussion for general questions when available
- Open an issue for concrete bugs and feature requests
- Contact the maintainer at daniloaguiarbr@proton.me for private contribution questions
- Be respectful and constructive in every interaction
