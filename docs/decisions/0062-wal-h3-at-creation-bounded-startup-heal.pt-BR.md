# ADR-0062 — Limite de taxa do WAL na criação do journal e um `startup_wal_heal` limitado

## Status
Aceito (2026-10-15)

## Contexto
- O limite de taxa H3 (`h3_rate_limit`) era contado no `Drop` do journal. Depois de 10 journals em 60 s, todo journal seguinte era preservado após o commit: um `replace foo bar` em 120 arquivos de um diretório deixava 110 arquivos `.atomwrite.journal.*` residuais (gaps.md W1).
- O `startup_wal_heal` varria o workspace inteiro em toda execução, `target/` e `.git` incluídos, ignorava `--threads` e conferia o orçamento de 100 ms só depois da varredura (gaps.md W9). Um `read` de um arquivo num workspace cujo `target/` tem 40203 entradas usava de 73 a 90 MB.
- Um SIGTERM ou SIGINT cooperativo no meio de um `replace` deixou um journal `started` em 16 de 100 execuções, embora o alvo estivesse intacto (gaps.md W46).
- O `inplace.rs` removia todo arquivo `.atomwrite-*` do diretório, então podia apagar o tempfile de outro processo (gaps.md W3).

## Decisão
- O H3 saiu do `heuristics_should_preserve`. Ele vale só na criação do journal (`creation_throttled`), na política `Auto`: depois de 10 journals abertos em 60 s, as próximas gravações `Auto` não abrem nenhum e continuam atômicas por tempfile e rename. O `--wal-policy always` nunca é limitado.
- Um journal que commitou é sempre removido.
- O `startup_wal_heal` roda só em verbo que muta e só nos diretórios citados no argv (a subárvore para um alvo diretório, só o pai para um alvo arquivo). Ele pula `.git`, `target` e `node_modules`, respeita `.gitignore` e `.ignore`, e confere o orçamento de 100 ms dentro da varredura (`walk_journal_paths_bounded`).
- O `wal-heal` mantém a varredura completa. O `--no-auto-heal` desliga o heal da partida.
- O `JournalGuard::discard()` remove o journal quando o braço do rename falha com `Cancelled`, então uma execução cancelada não deixa nenhum. Uma falha real e um crash mantêm o journal.
- O `inplace.rs` remove só o tempfile da sua própria operação.

## Consequências
- 120 arquivos com `replace foo bar`: 110 journals residuais antes, 0 depois.
- Num `replace` em 120 arquivos, só os 10 primeiros ganham um journal de recuperação de crash.
- O `semantic-merge` e o `batch` não estão na lista de alvos explícitos, então não ganham heal automático, só o `wal-heal` explícito (gaps.md W28, P3).
- Correção, 2026-10-28: só o `batch` continua fora da lista, porque o `semantic-merge` passou a estar em `explicit_target_roots` como `a.output` e agora ganha o heal automático.
- Um `kill -9` ainda deixa um journal (46 de 100 execuções), por desenho. O `wal-heal` é o caminho de recuperação, e um journal `started` de um processo morto não é removido por ele (`removed:0, preserved:1`, gaps.md W44, P3).
- Séries de SIGTERM depois da correção: 0 de 100 e 0 de 100 execuções deixaram um journal, medido no binário debug e sem repetição em release (gaps.md W46).
- A sentinela H4 é `.atomwrite_keep_wal`, o nome que diz o que ela faz: um arquivo no diretório do alvo preserva o journal (gaps.md W15, fechado na 0.1.37). O nome antigo `.atomwrite_no_wal` continua honrado, porque diretório que o tem mantém o comportamento que sempre teve.

## Alternativas
- Manter o H3 no `Drop`: rejeitada, ele mantinha o journal de uma gravação que já tinha commitado.
- Varrer o workspace inteiro na partida: rejeitada, o custo crescia com o tamanho do `target/`.

## Evidência
- `CHANGELOG.md:161-167,262-263,286` e `gaps.md:158-173` (W1, W3, W9, W28), `gaps.md:296-301` (W46).
- `src/wal/policy.rs:84-96` (`creation_throttled`), `src/atomic/write.rs:106` (o único chamador), `src/wal/heuristics.rs:46-62` (contador do H3) e `:146-147`, `src/wal/journal.rs:227` (comentário de que o H3 não é consultado) e `:329`.
- `src/lib_dispatch.inc.rs:8` (`startup_wal_heal`), `:124` (`explicit_target_roots`), `src/lib.rs:218`, `src/wal/stats.rs:203` (`walk_journal_paths_bounded`), `src/wal/heal.rs:61-69`.
- `tests/cli_v0137_wal_journals.rs` (2 testes), `tests/cli_v0137_wal_startup_heal.rs` (4 testes), `tests/cli_v0137_wal_cancel.rs` (1 teste).

## Gatilho para revisar
- O `batch` ganhar uma lista de alvos explícitos, porque o `semantic-merge` já a tem.
- A sentinela H4 ser renomeada ou mudar de significado.
