# ADR-0074 — Leitura-modificação-escrita serializada e um `apply` estrito

## Status
Aceito (2026-10-15)

## Contexto
- Uma gravação calculada a partir do arquivo que ela substitui perdia atualizações sob concorrência. O `atomic_write` pegava o lock consultivo só na hora de gravar, depois de o verbo ter lido o arquivo, então dois escritores liam a mesma base e o segundo `rename` apagava o primeiro (gaps.md W31, achado V2 A1 da revisão).
- Medido antes da correção: 30 `edit` paralelos em linhas distintas de um arquivo saíam todos com exit 0 e `ok: true`, e só de 4 a 10 das 30 mudanças sobravam. 30 `set` paralelos mantinham 1 chave, 30 `replace` mantinham 1, e 19 `config set` paralelos mantinham 2.
- O `apply` com diff unificado usava só o número de linha do cabeçalho `@@`. Um patch `@@ -1 +1 @@ -zzz +yyy` sobre um arquivo que começava com `alpha` virou `yyy` com exit 0, e um hunk com contexto errado apagou três linhas. O `.bak` de um sucesso é removido, então não sobrava cópia (gaps.md W29, V2 C1, crítico, presente desde a 0.1.36).
- O `apply` com SEARCH/REPLACE engolia `Cancelled`, `InvalidInput` e `MatchAmbiguous` (um `else if let Ok(...)`), e um bloco ausente sumia com `ok: true` e `hunks_applied: 1`, deixando o arquivo meio aplicado (gaps.md W30, V2 A2).

## Decisão
- Um verbo de leitura-modificação-escrita pega o lock consultivo ANTES de ler e segura o guard até a gravação ser commitada: `acquire_for_update` em `src/lock.rs`, com o orçamento de espera `lock.acquire_timeout_ms`. Os verbos são `edit`, `edit-loop`, `set`, `del`, `config set`, `config unset`, `apply`, `write --append`, `write --prepend`, e as ops `replace` e `edit` do `batch`.
- O `edit-loop` lê o stdin antes de pegar o lock, então nunca segura o lock enquanto espera entrada.
- O `replace` lê cada arquivo sem o lock. Na hora de gravar ele pega o lock, compara o checksum e, se o arquivo mudou, refaz a substituição sobre o conteúdo novo, de modo que todo escritor tem sucesso e nenhuma atualização se perde.
- O lock é reentrante por thread (o `atomic_write` interno do mesmo verbo o compartilha), e o `LockGuard` deixou de ser `Send`. Uma segunda thread do mesmo processo pergunta ao SO e espera, como outro processo esperaria.
- O `atomic_write_based(target, content, opts, workspace, expected_base)` é a rede para os verbos que não seguram o lock durante a leitura. Sob o lock ele calcula o hash do arquivo atual e compara com o checksum do conteúdo de onde o conteúdo novo saiu. Um arquivo diferente ou ausente falha com `STATE_DRIFT` (exit 82, retryable) e não grava nada, antes de abrir qualquer journal. `case`, `transform`, `scope`, `replace` e `edit` o chamam. Quem perde refaz a execução.
- O `apply` com diff unificado compara o contexto e as linhas removidas de cada hunk com o arquivo (igualdade exata, espaço no fim da linha ignorado). Um hunk é achado na posição do seu cabeçalho ou no vizinho exato mais próximo até `PATCH_MAX_HUNK_SHIFT` (64) linhas. Todos os hunks são aplicados em memória e gravados uma vez, e as linhas de contexto mantêm o texto do arquivo.
- Uma divergência é `MATCH_FAILED` (exit 65) que nomeia o hunk e a primeira linha que discorda, com `best_candidate` (estratégia `unified-hunk`), e nada é gravado. Um cabeçalho `@@` ilegível e um patch sem hunk são `INVALID_INPUT`. Um hunk só de inserção insere DEPOIS da linha nomeada.
- O `apply` com SEARCH/REPLACE tenta primeiro o match exato e depois a cascata fuzzy. Só `MatchFailed` marca um bloco como ausente. Qualquer bloco ausente falha o lote inteiro com `MATCH_FAILED` que nomeia os blocos, com `best_candidate` quando o texto lembra alguma linha, e nada é gravado. `Cancelled`, `InvalidInput` e um match fuzzy ambíguo chegam ao chamador. Um patch sem bloco é `INVALID_INPUT`.
- Não existe `--partial` no `apply` e nenhum foi criado.

## Consequências
- 30 `edit`, 30 `replace`, 30 `set` e 16 `config set` paralelos saem todos com exit 0 e toda mudança está no arquivo (`cli_v0137_rmw_concurrent`).
- Resíduo declarado: onde não existe `flock` (NFS) e no Windows (o lock é um no-op, porque o `LockFileEx` exige `unsafe` e a feature `Win32_System_IO` do `windows-sys`) só `edit`, `replace`, `case`, `transform`, `scope` e `codemod` mantêm a rede do checksum (CHANGELOG).
- O `edit-loop` não passa checksum-base, e o `batch --transaction` trava por operação e não por lote.
- O `--expect-checksum` roda fora do lock, então resta uma janela de TOCTOU entre a checagem e a gravação (gaps.md, P3).
- Um nome de arquivo acima de cerca de 238 bytes pode estourar o limite de 255 bytes com o nome do sidecar `.<nome>.atomwrite.lock`, e uma gravação in-place num diretório onde o sidecar não pode ser criado roda sem o lock (gaps.md, P3).
- Um processo morto por SIGKILL pode deixar um sidecar de lock. O `flock` já foi liberado pelo kernel e o próximo escritor reaproveita e remove o arquivo (gaps.md W44).
- Mudanças de comportamento: um hunk só de inserção do `apply` inseria uma linha acima, e as linhas de enchimento em branco que o cabeçalho do hunk não conta são descartadas.

## Alternativas
- Manter o lock na hora de gravar e confiar no `--expect-checksum`: rejeitada, ele detecta a atualização perdida depois que ela aconteceu e não a impede, e só de 4 a 10 das 30 mudanças sobraram na medição.
- Deixar o `apply` gravar os hunks que casam: rejeitada, um bloco ausente não pode deixar o arquivo meio aplicado, e o `apply` não tem `--partial`.
- Um lock no Windows por `LockFileEx`: não feito, exige `unsafe` e uma feature que não está habilitada, e não havia host Windows para provar.

## Evidência
- `CHANGELOG.md:74,202-212,278` e `gaps.md:251-266` (W29, W30, W31 com o seu recorte P3), `gaps.md:321-324` (lista P3).
- `src/lock.rs:1-60` (o que o lock cobre e suas limitações), `:150-152` (`acquire_for_update`), `:104` (`HELD`, a tabela por thread) e `:176-230` (`acquire_exclusive`), `src/atomic/write.rs:54-100` (`atomic_write_based`, a comparação da base e o `StateDrift`).
- Chamadores de `acquire_for_update`: `src/commands/edit/single.inc.rs:33`, `src/commands/edit_loop.rs:287`, `src/commands/set.rs:63`, `src/commands/del.rs:53`, `src/commands/config_cmd_write.inc.rs:26`, `src/commands/apply.rs:62`, `src/commands/write.rs:74`, `src/commands/replace_run.inc.rs:282,386`, `src/commands/batch/ops.inc.rs:151,277`.
- Chamadores de `atomic_write_based`: `src/commands/case.rs:266`, `src/commands/transform.rs:290`, `src/commands/scope/mod.rs:320`, `src/commands/replace_run.inc.rs:417`, `src/commands/edit/single.inc.rs:185`, `src/commands/edit/multi.inc.rs:315`.
- `src/commands/apply_unified.inc.rs:49-100` (`apply_unified`), `:103-125` (`locate_hunk`), `:132-170` (`hunk_mismatch`), `src/commands/apply.rs:169-255` (`apply_search_replace`), `src/constants.rs:364-366` (`PATCH_MAX_HUNK_SHIFT` 64, `PATCH_ERROR_SNIPPET_CHARS` 120).
- `tests/cli_v0137_rmw_concurrent.rs` (5 testes, a base velha na linha 182), `tests/cli_v0137_apply_strict.rs` (8 testes), `tests/cli_v0137_lock.rs` (3 testes).
- Binário 0.1.37 numa bancada com `HOME` e `XDG_*` dentro do scratchpad: 30 `edit --fuzzy off` paralelos em 30 tokens distintos de um arquivo deram 30 eventos com `ok: true`, 30 linhas mudadas, 0 sobrando, e nenhum sidecar sobrou no workspace. O `apply --format unified` com contexto errado saiu com exit 65 `MATCH_FAILED` com `best_candidate` (estratégia `unified-hunk`) e o arquivo continuou `alpha`, `beta`, `gamma`. O `apply --format search-replace` com um bloco presente e um ausente saiu com exit 65 `MATCH_FAILED` e o arquivo continuou intacto. Um patch unificado correto saiu com exit 0 e `hunks_applied: 1`.

## Gatilho para revisar
- Um lock no Windows ser implementado, ou o Windows ser validado num host real.
- O `edit-loop` passar a mandar um checksum-base, ou o `batch --transaction` travar por lote.
- O `apply` ganhar um modo parcial por pedido explícito.
