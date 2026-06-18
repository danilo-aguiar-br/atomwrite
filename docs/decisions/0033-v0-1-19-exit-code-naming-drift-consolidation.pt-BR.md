# 0033-v0-1-19-exit-code-naming-drift-consolidation (PT-BR) — Tradução

_Tradução automática do ADR original em inglês. Em caso de divergência, a versão EN prevalece._

# ADR-0033: v0.1.19 — Exit code documentation drift consolidation

- **Status**: Aceito
- **Data**: 2026-06-14
- **Contexto**: Phase D testing on 2026-06-14 ran 7 concrete binary-level probes against the v0.1.18 release and surfaced 7 places where the published docs (SKILL.md EN+PT, `error-saída.schema.json`, README, CHANGELOG) diverged from the actual binary behavior. Each drift is small individually, but together they create an environment where agents and CI gates não pode reliably interpret códigos de saída. The drifts são:
  1. `STATE_DRIFT` (82) vs `CHECKSUM_VERIFY_FAILED` (81) — the docs said `--verificar-checksum` returns `CHECKSUM_VERIFY_FAILED`; the binary returns `STATE_DRIFT`. The 81-code is reserved for the `read` path's BLAKE3 mismatch on the file content. The 82-code is the optimistic-locking failure that includes the `--expect-checksum` mismatch on writes/edits and the `--verificar-checksum` mismatch on reads.
  2. `SYNTAX_ERROR` vs `SYNTAX_ERROR_DETECTED` — the docs in v0.1.12 named the code `SYNTAX_ERROR`; the binary in v0.1.18 emits `SYNTAX_ERROR_DETECTED`. The rename happened in the v0.1.12 G72 tree-sitter rollout but the docs foram not updated.
  3. `ORPHAN_JOURNAL` (93) is consultive, NOT auto-detected — the docs implied that any stale sidecar is detected on every invocation. The actual gate is `ATOMWRITE_WAL=1` OR `--strict-atomic`; the padrão `write` não write a sidecar and therefore não pode detect orphans.
  4. `BROKEN_PIPE` (141) exige real SIGPIPE propagation — the docs implied a simple `head -1` pipe triggers it. The actual behavior is that the v0.1.4+ SIGPIPE restoration puts the padrão disposition back, so the signal is only raised when the downstream consumer actively closes the pipe mid-stream.
  5. Binary file reads return saída 0 with `kind=binary` metadata, NOT saída 65 — the v0.1.4 `BINARY_FILE` heuristic foi changed to emit a structured envelope and saída 0; the 65-code path now only fires for `read` without `--format raw` AND with the binary heuristic bypassed.
  6. Missing positional returns `ARGUMENT_PARSE_ERROR` (saída 2), NOT `INVALID_INPUT` (65) — clap-level argument errors são reported as saída 2. The 65-code is reserved for runtime content validação (e.g. malformed TOML, invalid regex, empty stdin padrão).
  7. Missing `--workspace` padrões to CWD, NOT an error — the docs implied `--workspace` is obrigatório; the actual behavior anchors relative paths to CWD and only fires `WORKSPACE_JAIL` (126) when an absolute path resolves outside the effective jail.

- **Decisão**: Accept the binary behavior as canonical. Consolidate the docs in v0.1.19 to match the binary. Especificamente,:
  1. **STATE_DRIFT absorbs CHECKSUM_VERIFY_FAILED for `--verificar-checksum`** — atualizar the `Exit Codes` table to note the absorption, atualizar the `Error Code List` to flag `CHECKSUM_VERIFY_FAILED` (81) as histórico, and add a drift note in both SKILL files.
  2. **Rename `SYNTAX_ERROR` to `SYNTAX_ERROR_DETECTED`** in both the `Exit Codes` and `Error Code List` sections. The histórico name is preserved only in prose for grep-ability.
  3. **Document the ORPHAN_JOURNAL gate** — add explícito text in both SKILL files stating that the consultive path exige `ATOMWRITE_WAL=1` or `--strict-atomic`. The current `WalPolicy::Auto` (v0.1.16 G119) means the sidecar is skipped for trivial writes, so padrão invocations never see this code.
  4. **Document the BROKEN_PIPE propagation requirement** — the contract is "saída 141 is raised when SIGPIPE is delivered", not "saída 141 is raised when the saída is short". The v0.1.4+ SIGPIPE restoration note belongs here.
  5. **Document the binary read envelope** — `kind=binary` is the canonical signal; the 65-code is the secondary, edge-case path.
  6. **Document the clap vs runtime split** — saída 2 is clap, saída 65 is runtime. The SKILL already separates them; the drift section reinforces the distinction.
  7. **Document the CWD fallback** — `--workspace` is documented as a flag with a CWD padrão, not a obrigatório argument. `WORKSPACE_JAIL` semantics são tied to the effective jail (CWD when `--workspace` is omitted).
- **Consequências**:
  - **+** All 7 drifts têm a one-line note in the v0.1.19 drift section of both SKILL files. Agents and CI gates can grep the drift section when an código de saída não match the legado table.
  - **+** CHANGELOG v0.1.19 entry documents the consolidation in a single bullet, indexed by Phase D testing date 2026-06-14.
  - **+** ADR-0033 captures the rationale so future maintainers não re-discover the drifts.
  - **+** No binary change obrigatório; the docs now match the binary em vez de the other way around.
  - **-** Operators that scripted against the legado `SYNTAX_ERROR` code name will see a runtime mismatch. Mitigated: the SKILL drift note is grep-discoverable and the schema (`error-saída.schema.json`) already uses `SYNTAX_ERROR_DETECTED`.
  - **-** The `CHECKSUM_VERIFY_FAILED` (81) code is now histórico; callers that matched on it must migrado to `STATE_DRIFT` (82). None observed in this repository's own teste suite.

- **Alternativas consideradas**:
  1. Change the binary to match the docs (rename `SYNTAX_ERROR_DETECTED` back to `SYNTAX_ERROR`, split `STATE_DRIFT` and `CHECKSUM_VERIFY_FAILED`, etc.). Rejected: would break the v0.1.12 G72 contract that testes already pin, force a major version bump, and the new behavior is strictly more correct (e.g. `STATE_DRIFT` already covers the locking semantics that `CHECKSUM_VERIFY_FAILED` foi trying to express).
  2. Add a `--legado-saída-codes` opt-in that restores the v0.1.12 names. Rejected: YAGNI. The drift notes são sufficient for migração; a runtime flag would invite permanent suporta burden.
  3. Document the drifts in a separate `docs/drifts/v0-1-18.md` file and link from CHANGELOG. Rejected: the SKILL files são the primary lookup surface for agents; a separate file is less discoverable.
  4. Skip the CWD-fallback note because the `--workspace REQUIRED` phrasing in the global flags section already implies it. Rejected: the v0.1.4 GAP 13 fix made the error envelope context-aware, and the CWD-fallback contract is the foundation of that fix; it deserves a dedicated note.

- **Gatilho para revisitar**: If a v0.2.0 release introduces a new código de saída, copy the drift section into a v0.2.0 migração chapter and convert the in-place drift notes to archived histórico links. If a binary teste fails because of an undocumented código de saída, the new drift gets its own bullet here.


---

_Original em inglês: [`0033-v0-1-19-exit-code-naming-drift-consolidation.md`](0033-v0-1-19-exit-code-naming-drift-consolidation.md)_
