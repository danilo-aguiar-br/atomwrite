# ADR-0054 — v0.1.34 fuzzy one-shot + timeout padrão 120

## Status
- Aceito em 2026-07-19
- Publicação docs-complete do runtime one-shot da v0.1.33 (mesmo comportamento do binário)

## Contexto
- Edições de expand-section de agentes frequentemente produzem um replacement que **contém** o pattern (NEW embute OLD)
- Antes da 0.1.33, o multi-apply fuzzy reescaneava texto inserido e podia hangar por dias (crescimento infinito de CPU)
- O global `--timeout-secs` tinha padrão `0` (desabilitado), então jobs ilimitados de monorepo e hangs não tinham backstop de wall-clock
- O multi-apply padrão era efetivamente ilimitado quando `--max-replacements` era omitido
- gaps.md rastreia residual ONESHOT-*; o fix de código saiu como 0.1.33; a 0.1.34 alinha a documentação agent-facing

## Decisão
- Multi-apply fuzzy é **one-pass** esquerda→direita no conteúdo **original** (`apply_fuzzy_one_pass`); **nunca** reescaneia texto inserido
- Máximo de applies fuzzy padrão = **1** quando `--max-replacements` é omitido; teto rígido **10_000**
- Se o replacement **contém** o pattern (string fixa), força apply **único** mesmo com `--max-replacements` grande (previne crescimento infinito na expansão de seção)
- Global `--timeout-secs` (alias `--timeout`) **padrão 120**; `0` desabilita; prazo esgotado → exit **124**
- Cancel cooperativo é consultado no meio da cascata fuzzy
- **Cancel vs timeout (exits distintos):**
  - Cancel cooperativo no meio da cascata (SIGINT/SIGTERM ou flag de cancel) → evento NDJSON `cancelled` / exit **143**
  - Prazo wall-clock de `--timeout-secs` → envelope de erro / exit **124**
- Fatias densas de CPU podem ultrapassar levemente o prazo antes do próximo poll (honesto ONESHOT-017); ainda é cooperativo, não preempção dura
- Caps: pattern 64 KiB; lev 8192 chars; max windows 4096; growth max(4×, +16 MiB)
- Sem telemetria de conteúdo de arquivos ou patterns
- Modos fuzzy permanecem só `auto|aggressive` (`off` rejeitado exit 65 desde v0.1.30)
- Suíte de regressão: `tests/cli_v0133_oneshot_fuzzy.rs` (hang < 2s, embeds, timeout)
- 41 subcomandos inalterados

## Consequências
- Agentes podem expandir seções com segurança quando NEW embute OLD, sem risco de hang
- Scripts que dependiam de runtime ilimitado devem passar `--timeout-secs 0` (ou valor maior) de propósito
- Multi-hit fuzzy em massa exige `--max-replacements N` explícito (e ainda é one-pass)
- Documentação agent-facing (docs/, skill, llms, README) deve refletir 0.1.34
- Pin `^0.1.34` para docs-complete; runtime idêntico ao oneshot da 0.1.33

## Referências
- [gaps.md](../../gaps.md) (ONESHOT-*)
- [CHANGELOG](../../CHANGELOG.md)
- [MIGRATION.pt-BR.md](../MIGRATION.pt-BR.md#v0133-para-v0134-atual)
- [MIGRATION.pt-BR.md](../MIGRATION.pt-BR.md#v0132-para-v0133) (BREAKING de runtime)
- ADR-0052 (contrato residual de agente v0.1.30)
- `tests/cli_v0133_oneshot_fuzzy.rs`
