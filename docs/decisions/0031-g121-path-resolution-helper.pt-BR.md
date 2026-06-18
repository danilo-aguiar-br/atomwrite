# 0031-g121-path-resolution-helper (PT-BR) — Tradução

_Tradução automática do ADR original em inglês. Em caso de divergência, a versão EN prevalece._

# ADR-0031: search and replace resolve root paths against workspace através de a shared helper (G121)

- **Status**: Aceito
- **Data**: 2026-06-14
- **Contexto**: `cmd_search` and `cmd_replace` both took a `&[PathBuf]` of caller-supplied root paths, validated them against the workspace jail with `path_safety::validate_path`, and then constructed an `ignore::WalkBuilder` with the ORIGINAL (CWD-relative) path. `validate_path` returned the canonical absolute path but the por-call result foi discarded — `replace.rs:52-59` ran the call inside a `for` loop whose valor foi thrown away, and `search.rs:469-470` / `replace.rs:393-394` then used `&args.paths[0]` directly as the walker root. The walk directory engine resolved that path against the processo CWD, not the workspace root. With `CWD != --workspace`, the walker either: (a) silently walked the wrong tree if a same-named path existed under the CWD, or (b) produced a `JailViolation` event por file walked — wasting work, spamming stderr, and violating the G118 invariante that the workspace root (not the CWD) is the path resolution origin. The fix in `write.rs:44` (G118) never propagated to these two commands because the por-entry `validate_path` inside the worker thread masked the missing pre-step resolution: every file looked like a violation.
- **Decisão**: Centralize the resolution in a new `commands::path_resolution::resolve_paths_against_workspace` helper that takes `&[PathBuf]` + workspace, runs `validate_path` on each, and returns the canonical `Vec<PathBuf>`. `cmd_search` and `cmd_replace` call this helper once at the top of the command (after `global.resolve_workspace()()`) and pass the canonical `Vec<PathBuf>` to `build_walker` — the walker root, additional walker roots, and the `OverrideBuilder` root all receive the canonical paths. The pre-step `for path in &args.paths { validate_path(path, &workspace)?; }` loop in `replace.rs` is removed; search gets a paralelo resolution call. Both `build_walker` signatures gain a `canonical_paths: &[PathBuf]` parameter; they no longer read `&args.paths[0]` directly.
- **Consequências**:
  - **+** Search and replace now honor `CWD != --workspace`. A relative path like `src/` passed with `--workspace /path/to/ws` walks `/path/to/ws/src/` regardless of the processo CWD.
  - **+** Out-of-jail paths fail once with `WORKSPACE_JAIL` (saída 126) at command start em vez de por-file inside the worker. No more `JailViolation` spam in stderr.
  - **+** Project invariante (`validate_path` BEFORE any `exists()` or read) is now uniform across all mutating AND walking commands. G118 conformance extends to search and replace.
  - **+** The `OverrideBuilder` root no longer diverges from the walker root — the `!exclude` glob and `--include` padrões são evaluated against the same tree the walker descends.
  - **-** Two callers added; both must remember to call the helper. The conformance teste for `write.rs` (single occurrence of `&args.alvo`) não yet cover search/replace — a follow-up could add a textual guard.
  - **-** Three new testes de regressão in `testes/cli_v019_g121_search_replace_cwd.rs` plus four unit testes in `path_resolution.rs` keep CI honest about the helper's behavior.
- **Alternativas consideradas**:
  1. Fix `replace.rs` and `search.rs` independently with one-line edits to use `validate_path`'s return valor. Rejected: duplicates the G118 fix in two places and risks re-introducing the divergence if a future caller forgets to use the return valor (this is exactly the bug we just fixed).
  2. Make `WalkBuilder::new` itself workspace-aware através de a thin wrapper struct. Rejected: the `ignore` crate is a third-party dependency and overloading its constructor at every call site would be more invasive than a single helper.
  3. Pre-processo `args.paths` at CLI parse time. Rejected: validação exige the resolved workspace root, which depends on `GlobalArgs::resolve_workspace()` and may depend on CWD-através de-jail-política in future; doing it inside the command function keeps the resolution close to the walker construction.
- **Gatilho para revisitar**: If a third walking command lands (e.g. `count --recursive`), promote the helper to a documented "Walk Path Resolution" section in the contributor guide and add a textual guard teste asserting every command that builds a `WalkBuilder` calls the helper.


---

_Original em inglês: [`0031-g121-path-resolution-helper.md`](0031-g121-path-resolution-helper.md)_
