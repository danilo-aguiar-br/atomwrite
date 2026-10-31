# ADR-0073 — Um ponto terminal de cancelamento e o `deadline_exceeded` (A1, A3)

## Status
Aceito (2026-10-15). Emenda o ADR-0054, que dava o exit 143 como a saída do cancelamento cooperativo e descrevia o prazo de 124 como um envelope de erro simples.

## Contexto
- Cada verbo que varre escolhia sozinho entre um envelope, silêncio e um resumo parcial ao ser interrompido.
- Um consumidor que lia a última linha de um `search` cancelado a tomava como resultado final.
- Um consumidor não separava um orçamento esgotado de um sinal sem ler o texto humano.
- O motivo do cancelamento de `delete`, `copy`, `hash`, `backup` e do leitor de stdin era uma frase fixa, `cancelled by signal`, mesmo quando o prazo global disparava.
- O prazo interrompia uma varredura só entre arquivos. Um arquivo enorme sem match ia até o fim.

## Decisão
- Um ponto terminal de cancelamento no dispatch (`finish_run` em `src/lib_dispatch.inc.rs`): quando a execução foi cancelada, o writer descarta o que ainda guarda para o passe de cauda, escreve o evento `cancelled` sem moldagem e faz o flush. Todo verbo que varre emite o mesmo envelope, e o `cancelled` é a última linha da saída também com `--sort-by`, `--dedupe` e `--aggregate`.
- O registro terminal de cancelamento é escrito sem moldagem, então `--filter`, `--select` e `--limit` não conseguem descartá-lo. Uma flag de moldagem descreve como apresentar uma resposta completa e não pode apagar o aviso de que a resposta está incompleta.
- O `deadline_exceeded` é um booleano sempre serializado e nunca omitido. Ele vem de `shutdown.is_timeout()`.
- O status de saída segue a causa viva: SIGHUP 129 (`signal: "SIGHUP"`), SIGINT 130, SIGTERM 143, o prazo 124. O motivo vem do sinal vivo pelo `cancel_reason`.
- Uma primitiva única de cancelamento com granularidade de bloco, o `CancelCheck`, é dividida pelo laço de varredura. O `CancelRead` confere a bandeira a cada leitura, então o prazo e o SIGTERM são vistos dentro de um arquivo enorme (`search_file_cancellable`).
- O aviso de cancelamento fica fora do canal de dados e os diagnósticos viajam só no canal de diagnóstico.
- O `src/stdin_cancel.rs` é um leitor de stdin cancelável, então `write`, `edit`, `edit-loop`, `apply`, `hash --stdin`, `extract`, `diff`, `calc` e `regex` obedecem ao SIGTERM (143) e ao prazo (124) enquanto o stdin ainda está aberto.

## Consequências
- Mudança de contrato do canal de dados: `search`, `count` e os outros verbos que varrem emitem o envelope `cancelled` ao serem interrompidos, onde antes emitiam silêncio ou um resumo parcial.
- Docs que descrevem o exit 143 como "o cancelamento" precisam da tabela de quatro vias: 129, 130, 143 e 124. O caminho de falha com pipe fechado é um quinto caso, o exit 141 (ADR-0063).
- Num arquivo de 492 MB sem match, `--timeout-secs 1` sai com exit 124 em 1,01 s onde levava 1,63 s, e um SIGTERM em 0,3 s sai com exit 143 em 0,307 s onde levava 1,60 s (T3b).
- O `cancelled` carrega `deadline_exceeded`, mas não `ok: false` nem `code`, por desenho do `docs/schemas/README.md` (gaps.md W44).
- Um `kill -9` não é um cancelamento. Ele ainda deixa um journal, e o `wal-heal` é o caminho de recuperação (ADR-0062).

## Alternativas
- Deixar cada verbo decidir o que imprimir ao ser interrompido: rejeitada, foi o comportamento que deixava os consumidores adivinhando.
- Um motivo fixo `cancelled by signal`: rejeitada, escondia o caso do prazo.

## Evidência
- `CHANGELOG.md:18-22,91-92,128-130,239-240,261` e `gaps.md:165-168` (W4), `gaps.md:20-29` (A1 a A3).
- `src/lib_dispatch.inc.rs:85-110` (`finish_run`), `src/ndjson/part2.inc.rs:112-156` (`CancelledEvent`, `deadline_exceeded`), `src/signal_cancel.inc.rs:55-110` (`CancelCheck`), `src/signal.rs:179` (`cancel_reason`), `src/constants.rs:131-149` (`EXIT_TIMEOUT` 124, `EXIT_SIGHUP` 129, `EXIT_SIGINT` 130, `EXIT_SIGTERM` 143), `src/stdin_cancel.rs` (245 linhas), `docs/schemas/cancelled-event.schema.json:29-37`.
- `tests/cli_v0137_cancel.rs` (10 testes, `deadline_exceeded` verificado nas linhas 521, 612 e 828), `tests/cli_v0137_safety.rs:426` (`r22_search_sighup_exits_cleanly_with_cancelled_envelope`), `tests/cli_v0137_stdin_cancel.rs` (13 testes), `tests/cli_v0137_search_cancel_file.rs`.
- Binário 0.1.37: `write x.txt` com um stdin que nunca fecha e `--timeout-secs 1` saiu com exit 124 e imprimiu `{"type":"cancelled","deadline_exceeded":true,...}`.

## Gatilho para revisar
- Surgir uma nova causa de cancelamento que os quatro exit codes não cubram.
- O `cancelled` ganhar `ok: false` e `code`.
