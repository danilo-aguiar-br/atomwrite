#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
BIN="${ATOMWRITE_BIN:-target/debug/atomwrite}"
if [[ ! -x "$BIN" ]]; then
  cargo build -q
  BIN="target/debug/atomwrite"
fi
OUT=docs/schemas
mkdir -p "$OUT"
"$BIN" --json-schema write > "$OUT/write-output.schema.json"
"$BIN" --json-schema replace > "$OUT/replace-result.schema.json"
"$BIN" --json-schema error > "$OUT/error-output.schema.json"
"$BIN" --json-schema progress > "$OUT/progress-event.schema.json"
"$BIN" --json-schema recipe > "$OUT/recipe-result.schema.json"
"$BIN" --json-schema best-candidate > "$OUT/best-candidate.schema.json"
"$BIN" --json-schema cancelled > "$OUT/cancelled-event.schema.json"
echo "schemas regenerated in $OUT"
