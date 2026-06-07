# ADR-0024: get/del TOML dotted-path uses manual Table descent, not toml_edit::Table::get

- **Status**: Accepted
- **Date**: 2026-06-07
- **Context**: v0.1.12 introduces `atomwrite get <path> <key-path>` and `atomwrite del <path> <key-path>`. The key path can be dotted like `package.version` or nested like `package.dependencies.serde`. The naive `doc.get("package.version")` returns `None` because `toml_edit::Document::get` looks up the literal key `"package.version"`, not the nested key.
- **Decision**: We wrote `get_toml_path(doc, key_path)` and `remove_toml_path(doc, key_path)` helpers that manually split the key path on `.` and descend `Table` segments. For JSON we use `serde_json::Value::pointer` (which already supports dotted paths).
- **Consequences**:
  - **+** Behavior is consistent with JSON pointer semantics; users do not need to learn two navigation conventions.
  - **+** We avoid pulling in the `dotted-lexer` or `toml-path` crate (yet).
  - **+** Error messages are clear: `key "package.version" not found at segment 2: no such key "version"` instead of `Option::None`.
  - **-** Manual descent does not handle TOML quoted keys with literal `.` inside them. We document this as a known limitation.
  - **-** Re-allocates a `Vec<&str>` for each lookup. Microbenchmark shows <1 µs per call, acceptable.
- **Alternatives considered**:
  1. Use `toml_edit::Table::get_many` (if it existed). Rejected: toml_edit does not have this API; closest is `Item::as_table_mut()` + manual descent.
  2. Use the `toml` crate (not `toml_edit`). Rejected: `toml` does not preserve comments/formatting; we would lose the v14 Tier 3 promise of `comments_preserved: true`.
  3. Require users to pass TOML paths as arrays (`["package", "version"]`). Rejected: dotted string is the de-facto convention (matches JSON pointer, jq, kubectl, terraform).
- **Trigger to revisit**: If toml_edit adds `get_many` or a similar API, we can simplify. If users ask for quoted-key support, we add it.
