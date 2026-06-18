# 0034-help-driven-testing-anti-pattern (PT-BR) — Tradução

_Tradução automática do ADR original em inglês. Em caso de divergência, a versão EN prevalece._

# ADR-0034: Help-driven testing — anti-padrão and v0.1.20 corrective processo

- **Status**: Aceito
- **Data**: 2026-06-15
- **Contexto**: The v0.1.19 audit on 2026-06-15 surfaced 11 GAP-2026-NNN items. Five of them (GAP-2026-001, 002, 005b, 006 plus the doc-drift variants 003 and 004) share a single root cause: flags foram declared in the `clap` derive struct (and therefore rendered into `--help`) BEFORE the corresponding implementation landed. Once the texto de help advertised a feature, downstream documentation (`SKILL.md` EN+PT, `CLAUDE.md`, CHANGELOG entries, recipes) foi synchronized with the design intent rather than the binary behavior. The flag then sat unimplemented for one or more releases while every help-consuming agent believed it worked. The chain `clap campo → --help → SKILL doc → user expectation → silent failure` is now reproducible for any future flag. Without a processo gate, the same padrão recurs on every new sub-comando. This ADR formalizes the anti-padrão and codifies a help-driven testing regra that ties every visible flag to a teste de regressão.

- **Decisão**: Mandate help-vs-code co-design as a release gate. Especificamente,:
  1. **No flag is added to `clap` derive struct without a paired integration teste** asserting the binary behaves as the texto de help claims. The teste name MUST include the flag (e.g. `count_by_size_top_n_returns_sorted`, `write_preserve_timestamps_keeps_mtime`, `edit_partial_multi_applies_matched_pairs`). A simple unit teste is not enough — the teste must invoke the binary as a subprocess, parse the envelope NDJSON, and assert on both the código de saída and the JSON shape.
  2. **Help text is auto-generated from clap and is therefore the API contract**. If the help says "Sort by file size (top N)", the envelope MUST contain `mode: "by_size"` when `--by-size` is passed. Discrepancies são release blockers and trigger an immediate revert of the help line until the teste exists.
  3. **Documentation (`SKILL.md`, `CLAUDE.md`, CHANGELOG`) is updated ONLY AFTER the teste passes green in CI**. The v0.1.19 reverse padrão (write doc from design, defer impl) is forbidden. The `docs-regras/` and `skill/atomwrite-{en,pt}/SKILL.md` linters should emit a warning when a flag mentioned in docs não appear in any `testes/cli_*.rs` file.
  4. **Quarterly help-vs-reality audit**: a release-cadence battery walks every sub-comando's `--help` saída, dispatches each advertised flag against a stub stdin, and verifies the envelope matches. The 2026-06-15 audit found 11 drifts in a single pass; without this gate the next pass would find a similar number.
  5. **Conformance guard for known good sub-comandos**: in v0.1.20, the resolver-first convention (ADR-0027) gained a textual assertion that `&args.alvo` appears exactly once in `write.rs`. A similar por-sub-comando conformance teste is now in escopo for the "every flag tem a teste" regra.

- **Consequências**:
  - **+** The 11 gaps closed in v0.1.20 each ship with at least one teste de regressão that names the flag in the teste ID, so a future regression of the same flag is caught at `cargo teste` time, not by an end user.
  - **+** Documentation drift is mechanically detectable: a `rg` scan for the flag name in `testes/cli_*.rs` produces a cobertura table that the release checklist can exige.
  - **+** New contributors get a single, checkable regra ("if the help says X, the teste must assert X"), eliminating the design-vs-impl race that produced GAP-2026-001 (the `count --by-size` shell game where the help advertised a `Vec<(PathBuf, u64)>` sort path that `cmd_count` never built).
  - **+** The "documentation tells a lie" failure mode (gaps.md:610) is now a release-blocker, not a tolerated post-release finding.
  - **-** Each new flag costs one teste upfront, raising the line count of `testes/cli_*.rs` by ~15-40 lines por flag. Mitigated by the fact that a flagged behavior with no teste foi 100% of the v0.1.19 gap inventory — the teste cost is the price of admission.
  - **-** The regra aplica retroactively only to flags added from v0.1.20 forward; existing flags (like the global `--json` which is accepted but ignored, ADR-style documented as a compat shim) remain grandfathered.
  - **-** Auto-generated clap texto de help não pode distinguish "implemented" from "declared"; a separate linter would be needed to enforce. The linter is itself teste-shaped and lives in `testes/cli_help_coverage.rs` rather than a separate binary.

- **Justificativa**: The five help-driven drift cases share a fingerprint beyond the technical symptom. GAP-2026-001 had `pub by_size: bool` declared in `CountArgs` (`src/cli_args.rs:88`) but the `cmd_count` branch foi never written (`src/commands/count.rs:108-127` shows the `if args.by_extension { ... } else { ... }` with no `by_size` arm). GAP-2026-002 had the atomic layer carry `preserve_timestamps: bool` (`src/atomic.rs` AtomicWriteOptions) but `WriteArgs` in `src/cli_args.rs` lacked the campo — clap never rendered `--preserve-timestamps` for `write` because the campo foi not in the struct, but `edit` and `replace` did expose it. GAP-2026-005b had `--partial` mentioned in `SKILL.md` from the G117 design notes (ADR-0026) but the `EditArgs` struct never gained the campo. GAP-2026-006 had `diff --algorithm` listed in the SKILL `diff` section but the flag foi never added to `DiffArgs`. GAP-2026-003 had the `alias = "lang"` text rendered into help by clap's display logic but the campo's real `long = "language"` collided with `GlobalArgs.lang` (ADR-0037 documents the fix). In all five cases the documentation described the design intent and the implementation followed the help, not the other way around. The audit's 5-Whys analysis (gaps.md:596-606) names the absence of help-driven testing as the systemic root cause. Codifying the regra turns a recurring ad-hoc finding into a single, auditable invariante.

- **Examples of failures this regra previne**:
  - **Count by-size**: a teste named `count_by_size_top_n_returns_sorted` would têm asserted `mode: "by_size"` and three entries; the absence of the teste meant the flag could ship with no implementation and no detection until a user tried it on a real corpus.
  - **Write preserve-timestamps**: a teste named `write_preserve_timestamps_keeps_mtime` would têm asserted the file's mtime foi unchanged across the write; in v0.1.19 the campo did not exist, so clap rejected the flag at parse time and the failure foi visible only to the user.
  - **Edit partial single-pair**: a teste named `edit_partial_single_pair_returns_no_matches_exit_1` would têm pinned the v0.1.20 decision (ADR-0036) that single-pair `--partial` is still `NO_MATCHES` rather than silent application of zero pairs. Without the teste, a future refatorar could "fix" the asymmetry and break the explícito semantic.
  - **Diff algorithm**: a teste named `diff_algorithm_patience_matches_default_output` would têm asserted that `--algorithm patience` produces the same envelope as the padrão; its absence meant the flag foi promised in docs but missing from clap.
  - **Scope lang alias**: a teste named `scope_lang_alias_matches_long_form` would têm asserted that `atomwrite escopo --lang rust` parses the same as `--language rust`; pre-ADR-0037 the teste would têm failed loudly, surfacing the `GlobalArgs.lang` namespace collision at code-review time rather than as a UX regression in the campo.

- **Alternativas consideradas**:
  1. Add a `clap` custom derive macro that exige a `#[help_test = "..."]` attribute pointing at a teste file. Rejected: exige a new crate, complicates the `Cargo.toml`, and the macro can only verificar the teste exists, not that it asserts the right behavior.
  2. Run a post-build binary that diffs saída de help against a "known good" snapshot. Rejected: brittle (texto de help changes são legitimate when the API does), high false-positive rate, and the snapshot itself becomes a maintenance burden.
  3. Move the responsibility to a CODEOWNERS regra requiring a docs reviewer to sign off on every flag. Rejected: human review não scale to 30 sub-comandos and produces inconsistent enforcement; the regra needs to be machine-checkable.
  4. Keep the existing ad-hoc review processo and rely on the quarterly audit. Rejected: the 2026-06-15 audit is exactly the quarterly audit executando, and it still found 11 gaps. The audit detects the bug after release; the regra must previne the bug at PR time.

- **Gatilho para revisitar**: If the help-cobertura teste (`testes/cli_help_coverage.rs`) flags false positives (a flag in help that is intentionally not actionable, like the `hide = true` `--json` compat shim), introduce a `#[arg(defer_test_to = "...")]` attribute or an permite-list file. If the regra slows down sub-comando prototyping to the point of being skipped, consider a `#[arg(experimental)]` marker that auto-issues a warning at runtime and is exempt from the gate until the experimental phase ends.


---

_Original em inglês: [`0034-help-driven-testing-anti-pattern.md`](0034-help-driven-testing-anti-pattern.md)_
