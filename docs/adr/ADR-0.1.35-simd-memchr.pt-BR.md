# ADR-0.1.35 — Localização fuzzy com SIMD: memchr primeiro (rejeitar triple_accel como padrão)

## Status
REJEITAR o `triple_accel` como dependência obrigatória/padrão na v0.1.35.

## Contexto
- As regras de fuzzy do GraphRAG recomendam o `triple_accel` para distância de edição em lote de alto volume.
- docsrs-cli: o `triple_accel` é apenas u8 (sem Unicode), com fallback AVX2→SSE→escalar.
- O fuzzy do atomwrite opera em janelas de linhas UTF-8 com `strsim::normalized_damerau_levenshtein`.

## Decisão
1. Manter o memchr para localização exata e o strsim para similaridade segura em Unicode.
2. A pontuação de janelas dentro do arquivo usa fan-out limitado do rayon (`windows::match_context_aware`).
3. NÃO adicionar o `triple_accel` a menos que um microbenchmark local mostre ≥15% nos caminhos de localização só ASCII e que um fast-path ASCII protegido seja isolado.

## Consequências
- Sem distorção semântica de Unicode causada por distância SIMD em u8.
- Uma futura feature opcional `simd-ascii` pode reavaliar, com emenda ao ADR + benchmarks.

## Alternativas consideradas
- Ligar o triple_accel em todas as janelas — rejeitado (Unicode).
- nucleo-matcher — rejeitado para one-shot offline (o ADR P3 já cobre).
