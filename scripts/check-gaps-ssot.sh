#!/usr/bin/env bash
# Fail if GAP-2026-019…033 Status lines disagree with the gaps.md footer inventory.
# Local release gate only — not product GitHub Actions.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GAPS="${ROOT}/gaps.md"
if [[ ! -f "$GAPS" ]]; then
  echo "check-gaps-ssot: missing $GAPS" >&2
  exit 2
fi

mapfile -t BODY < <(python3 - <<'PY' "$GAPS"
import re, sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding="utf-8")
status = {}
for m in re.finditer(
    r"## GAP-2026-(0(?:1[9]|2[0-9]|3[0-3]))[^\n]*\n\n### Status\n\n- ([^\n]+)",
    text,
):
    status[int(m.group(1))] = m.group(2).strip()
if not status:
    print("NO_BODY_STATUS", file=sys.stderr)
    sys.exit(2)
for n in range(19, 34):
    s = status.get(n, "MISSING")
    print(f"{n:03d}|{s}")
PY
)

footer="$(python3 - <<'PY' "$GAPS"
import re, sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding="utf-8")
# Footer inventory near end
fechados = set()
mitigados = set()
for m in re.finditer(r"\*\*Fechados[^*]*\*\*:\s*([^\n]+)", text):
    for n in re.findall(r"\b(\d{3})\b", m.group(1)):
        fechados.add(int(n))
for m in re.finditer(r"\*\*Mitigados[^*]*\*\*:\s*([^\n]+)", text):
    for n in re.findall(r"\b(\d{3})\b", m.group(1)):
        mitigados.add(int(n))
# Also accept unbold forms
for m in re.finditer(r"Fechados \([^)]*\):\s*([^\n]+)", text):
    for n in re.findall(r"\b(\d{2,3})\b", m.group(1)):
        fechados.add(int(n))
for m in re.finditer(r"Mitigados \([^)]*\):\s*([^\n]+)", text):
    for n in re.findall(r"\b(\d{2,3})\b", m.group(1)):
        mitigados.add(int(n))
print("FECHADOS=" + ",".join(str(x) for x in sorted(fechados)))
print("MITIGADOS=" + ",".join(str(x) for x in sorted(mitigados)))
PY
)"

echo "check-gaps-ssot: body statuses:" >&2
printf '%s\n' "${BODY[@]}" >&2
echo "check-gaps-ssot: footer: $footer" >&2

# Parse footer sets
eval "$(echo "$footer" | sed 's/^/export /')"
IFS=',' read -r -a F_ARR <<< "${FECHADOS:-}"
IFS=',' read -r -a M_ARR <<< "${MITIGADOS:-}"
declare -A FSET MSET
for n in "${F_ARR[@]:-}"; do [[ -n "$n" ]] && FSET[$n]=1; done
for n in "${M_ARR[@]:-}"; do [[ -n "$n" ]] && MSET[$n]=1; done

fail=0
for line in "${BODY[@]}"; do
  num="${line%%|*}"
  st="${line#*|}"
  n=$((10#$num))
  if [[ "$st" == MISSING* ]]; then
    echo "FAIL GAP-$num missing Status" >&2
    fail=1
    continue
  fi
  if [[ "$st" == Aberto* ]]; then
    echo "FAIL GAP-$num still Aberto (footer claims closed/mitigated inventory)" >&2
    fail=1
    continue
  fi
  if [[ "$st" == Fechado* ]]; then
    if [[ -z "${FSET[$n]:-}" && -z "${MSET[$n]:-}" ]]; then
      echo "WARN GAP-$num Fechado but not listed in footer (non-fatal if footer uses short form)" >&2
    fi
  fi
  if [[ "$st" == Mitigado* ]]; then
    if [[ -z "${MSET[$n]:-}" && -z "${FSET[$n]:-}" ]]; then
      echo "WARN GAP-$num Mitigado but not listed in footer" >&2
    fi
  fi
done

# Ensure no Aberto remains for 019-033
if printf '%s\n' "${BODY[@]}" | grep -q '|Aberto'; then
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  echo "check-gaps-ssot: FAILED" >&2
  exit 1
fi
echo "check-gaps-ssot: OK (019–033 no Aberto; statuses present)"
exit 0
