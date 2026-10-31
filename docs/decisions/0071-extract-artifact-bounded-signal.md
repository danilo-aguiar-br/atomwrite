# ADR-0071 — `extract-artifact`: bounded signal, never raw bytes (E5)

## Status
Accepted (2026-10-15)

## Context
- An agent needs the signal of a PDF, an Office file or an image: page count, dimensions, sheet header, slide titles, document text. It never needs the raw bytes.
- Putting raw bytes on the NDJSON data channel breaks the contract (one JSON object per line) and the byte budget.
- The extract limits were literals in the command code.

## Decision
- Add the subcommand `extract-artifact` PATH. It reads the container, extracts the signal and emits one bounded NDJSON record of `type: "artifact"` per artifact. The raw stream is never forwarded.
- Coverage is deliberately narrow: containers whose decoder is pure Rust with no C toolchain and no system package. They are `pdf`, `docx`, `xlsx`, `pptx`, and header-only image probing for `png`, `jpeg`, `gif`, `bmp`, `webp`, `tiff` and `ico`. Type detection reads the magic bytes and never the file extension.
- The record is bounded. Every part read is capped by `--max-filesize`, and a crafted container cannot blow the payload because sheets and slides are capped.
- The four limits are config keys (ADR-0058): `extract.max_sample_rows` (up to 100000), `extract.max_text_chars` (up to 10000000), `extract.max_sheets` and `extract.max_slides` (up to 10000). The flags `--max-sample-rows` and `--max-text-chars` still win over the config.
- The command is CPU-bound on a single artifact, so the shared rayon pool is not built (ADR-0064).
- Dependencies: `zip` (9.0.5, `deflate-flate2` backend), `flate2` (Rust backend), `quick-xml` (0.42), `imagesize` and `lopdf`.

## Consequences
- `extract-artifact` is one of the 47 verbs. The count went from 45 to 46 with it and to 47 with `config` (CHANGELOG, Corrections). It needs its schema in the 55 `--print-schema` names (ADR-0067).
- `zip` moved to 9.0.5 and `quick-xml` to 0.42 to close advisories in the OOXML and artifact readers (RUSTSEC-2026-0367, -0368, -0369, -0194 and -0195).
- Anything outside the listed formats is not read, by design.

## Alternatives
- Stream the raw bytes and let the agent decode them: rejected, it breaks the NDJSON contract and the budget.
- Wrap a decoder that needs a C toolchain or a system package: rejected, the verb stays pure Rust.

## Evidence
- `CHANGELOG.md:16,57,111,315-316` and `gaps.md:69-70` (E5).
- `src/commands/extract_artifact/mod.rs:3-17` (rationale and coverage), `:35-46` (default limits), `:48-64` (`ExtractArtifactArgs`), `:179` (the flag wins over the tuning value), `src/commands/extract_artifact/office.inc.rs:4`, `image.inc.rs:19`, `pdf.inc.rs`, `Cargo.toml:173-195` (dependencies), `src/cli.rs:237`.
- `tests/cli_v0137_artefato.rs` (17 tests) and `tests/cli_v0137_config_verb.rs` (`extract_keys_bound_the_artifact_and_a_flag_still_wins`).
- Binary 0.1.37: a synthetic 2x3 PNG header gave `{"type":"artifact","format":"png","width":2,"height":3}` and exit 0.

## Trigger to revisit
- A format outside the list is needed and has a pure Rust decoder.
- A decoder in the tree gets an advisory.
