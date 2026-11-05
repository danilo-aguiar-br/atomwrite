# ADR-0078 — One schema per verb, describing the whole stream

## Status
Accepted (2026-10-31), supersedes the name list of ADR-0067

## Context
- The emitter was written under the rule "one name describes the terminal event", and the verbs grew records before and after that terminal, so the document fell behind the stream.
- `recipe` served `RecipeResult` while emitting `begin`, `match`, `end`, `plan`, `replace`, `progress`, `hash`, `cancelled`, `recipe_step` and `recipe_result`, and each reemitted child record carried `step`.
- `codemod` served `CodemodSummaryEvent` while emitting `codemod`, `rule_begin`, `rule_error` and `transformed` with `rule_id`, and on failure it published the child `summary` as its own terminal.
- `batch`, `semantic-search`, `sparse`, `semantic-merge`, `case` and `watch` served only their summary, and `search`, `replace`, `list` and `backup` split the stream between the verb name and a `*-summary` name.
- No union declared the failure envelope, so a run that died before the first record validated against no schema of its own verb.
- `prune-backups` carried `additionalProperties: false` and `required: ["ok"]` beside the root `anyOf`, so a validator refused every per-entry record.

## Decision
### 1. One name per verb
- The verb name is the only document for its stream, and the `*-summary` names stop being served.
- `search`, `replace`, `list` and `backup` absorb the content of their summary name as the terminal branch of their own union.
- The served list drops from 59 to 55 names, backed by 54 files, because four summary files are removed with their names.

### 2. The error envelope is a branch of every union
- `union_of` appends the `ErrorJson` branch to every union it builds, including the ones that already existed.
- `diff` keeps its `oneOf` and gains the envelope as a fifth branch, exclusive because no record of the verb carries `ok: false` with a `code`.

### 3. The terminal branch stays closed
- `close_terminal_schema` returns early when the root holds `anyOf` or `oneOf`, because a union root declares no keys of its own.
- Each builder closes its own terminal branch with `additionalProperties: false`, the always-present keys in `required` and the failure detail `publish_failure` stamps.

### 4. Child tags are declared
- `tagged_record` declares the key the parent verb adds, which is `step` for `recipe` and `rule_id` for `codemod`.
- `terminal_record` adds the verdict keys the writer stamps on the last line, and the failure detail when the run failed.

### 5. Three copies of the `$defs` loop become one helper
- `union_of` hoists the `$defs` of every branch, strips `$schema` from each one and returns the document, so `extract`, `scope` and `record_with_own_summary` call it instead of repeating the loop.

## Consequences
- A consumer validates a whole stream with one schema instead of assembling a union by hand from two documents.
- A record shape the verb stops writing is a branch the schema still lists, and that is deliberate because the verb may write it in another mode.
- `docs/schemas/README.md`, `docs/AGENTS.md` and the two `CHANGELOG` files state the new count and the union of each verb.
- macOS and Windows remain unvalidated, because the proof of this round ran on Linux only.
