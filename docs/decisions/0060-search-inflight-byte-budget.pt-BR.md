# ADR-0060 — Um orçamento de bytes para os eventos do `search` em voo

## Status
Aceito (2026-10-15)

## Contexto
- A memória residente do `search` crescia com o número de threads. Medido numa máquina com 72 CPUs e 160 GB de RAM, árvore de 1620 arquivos e 380,9 MiB, cache quente, saída para `/dev/null`: 142,6 MB no `-j` padrão (gaps.md W10).
- A causa era um custo de cerca de 1,5 MB por thread do walker (31 MB com 1 thread, 38 MB com 8, 142 MB com 72), e não o canal de eventos.
- A memória era limitada contando eventos por worker, o que não diz nada sobre o tamanho deles: com um `--max-columns` grande, as mesmas contagens guardam uma quantidade arbitrária de bytes.
- Uma primeira versão do orçamento, com mínimo de 1 MiB, deixou o `search` de 10 a 15 vezes mais lento com 16 ou mais workers (`-j 72`: de 39,99 a 59,18 s), porque cada liberação acordava todo worker dormindo (gaps.md W33).

## Decisão
- Um orçamento único de bytes, `SEARCH_MAX_INFLIGHT_BYTES` (8 MiB), é dividido entre todos os workers do `search`. Ele é a chave `search.max_inflight_bytes`, aceita de 4 MiB a 1 GiB.
- O orçamento efetivo é `max(configurado, workers x SEARCH_BUDGET_BYTES_PER_WORKER)`, em que o piso por worker é de 128 KiB (o dobro do buffer inline de 64 KiB).
- O stream que o consumidor está drenando (a cabeça) passa do limite só enquanto não tem nenhum evento em voo, então um consumidor lento não trava o pipeline. Quando a cabeça tem um evento em voo, ela espera como qualquer outro stream.
- A cobrança vive num `Charge` cujo `Drop` devolve os bytes, então um evento escrito e um evento descartado por uma execução cancelada devolvem o espaço.
- O caminho quente não usa lock (um add ou subtract atômico por evento). O mutex e o condvar só põem um worker bloqueado para dormir, e uma liberação acorda um worker, só quando o total cai abaixo do limite.
- O `search` mantém um canal limitado por arquivo em vez de um mapa global.

## Consequências
- A memória é limitada seja qual for o número de threads e de matches.
- O `search` no `-j` padrão foi de 142,6 MB para 45,3 MB na árvore de 1620 arquivos. O `read` no `-j` padrão foi de 44,3 MB para 17,9 MB porque o pool do rayon não é mais criado para ele (ADR-0064).
- Um `-j 72` explícito vale acima do teto e mediu 143,6 MB numa medição e de 71 a 80 MB em outra. A meta de 64 MB vale para o `-j` padrão.
- Um `.atomwrite.toml` que ainda diga 1 MiB para a chave é recusado na carga.
- A medição é manual. O teste que roda é o `cli_v0137_search_rss`, com teto de 120 MiB.
- O `search` denso (3,24 milhões de linhas casadas) foi de 10,4 s para 12,9 s numa amostra única com carga 50, e a diferença não foi investigada (gaps.md W27, P3).

## Alternativas
- Contar eventos por worker: rejeitada, o tamanho do evento não tem limite.
- Um `notify_all` global a cada liberação: rejeitada, causou uma tempestade de futex (423 s de tempo de sistema contra 83 s de usuário).
- Manter o mínimo de 1 MiB: rejeitada, deixou o `search` até 15 vezes mais lento com muitos workers.

## Evidência
- `CHANGELOG.md:68,137-141,146,215-216,218,267,280` e `gaps.md:177-181` (W10), `gaps.md:270-273` (W33).
- `src/commands/search_budget.inc.rs:1-60` (notas de projeto) e `:239-240` (orçamento efetivo), `src/constants_concurrency.inc.rs:58-72` (8 MiB, 64 KiB inline, 128 KiB por worker), `src/constants_config.inc.rs:50-65` (4 MiB a 1 GiB).
- `tests/cli_v0137_search_rss.rs`, `tests/cli_v0137_search_stream.rs`.
- Binário 0.1.37: `config get search.max_inflight_bytes` devolveu 8388608, e `config set search.max_inflight_bytes 1048576 --scope workspace` saiu com exit 65 e "must be 4194304..=1073741824".

## Gatilho para revisar
- Uma medição no `-j` padrão passar da meta de 64 MB.
- Outro verbo passar a emitir eventos sem limite e precisar do mesmo orçamento.
