# ADR-0026: O edit com múltiplos pares ganha paridade fuzzy, relatório por par e --partial opt-in (G117)

- Status: Aceito
- Data: 2026-06-11
- Contexto: O `edit` aceita pares `--old`/`--new` repetidos, mas o caminho multi-par (`edit_old_new_multi`) usava apenas o casamento exato `find_str` e devolvia `InvalidInput` no primeiro erro, perdendo o lote inteiro e relatando nem o índice do par que falhou nem quais pares teriam sido aplicados. O caminho de par único tem uma cascata fuzzy de 9 estratégias, então o mesmo par com divergência de espaços em branco que o caminho único resgata matava o lote múltiplo. Além disso, o envelope de erro NDJSON vai para o stdout, então `edit ... | jaq '.edits'` mascarava o exit 65 como `{"edits": null}` com exit 0 do pipeline (G117, documentado no gaps.md em 2026-06-11).
- Decisão: Extrair a cascata para `match_pair(content, old, new, fuzzy_mode)`, compartilhada pelos dois caminhos. O caminho múltiplo agora roda a cascata completa por par, acumula `PairResult {index (base 1), matched, strategy, similarity}` e, na falha, levanta a nova variante `AtomwriteError::EditPairFailed {index, total, reason, pair_results}`, que reaproveita o código `INVALID_INPUT` e o exit 65 (sem um 26º código de erro) e enriquece o envelope de erro com `failed_pair_index`, `pairs_total` e `pair_results`. Os envelopes de sucesso ganham `pairs_total` e `pair_results`, e `mode` vira `fuzzy-multi(N)` quando algum par casou por fuzzy, permanecendo `exact-multi(N)` caso contrário. Uma nova flag opt-in `--partial` aplica os pares que casam (exit 0, `edits < pairs_total`) e relata o resto com `matched: false`, e zero pares aplicados vira `NO_MATCHES` (exit 1, igual ao `replace`) sem gravar nada.
- Consequências:
  - + A paridade fuzzy remove a assimetria não documentada entre par único e múltiplo.
  - + `failed_pair_index` torna o erro acionável sem bisseção manual, e `pair_results` é a verdade por item (princípio de falha parcial do Agent-First CLI).
  - + Um processo, um tempfile, um fsync e um rename para N pares de novo, e o contorno "1 par por invocação" morre.
  - + Tudo-ou-nada continua sendo o padrão: quando a atomicidade é possível, ela é estritamente melhor que relatar falha parcial, e o `--partial` é um opt-in explícito.
  - - No erro tudo-ou-nada, os pares depois do índice que falhou nunca são tentados e ficam fora de `pair_results` (a ausência mais o `failed_pair_index` distingue "falhou" de "não tentado").
  - - A ordem dos pares é comportamento definido: o conteúdo evolui entre os pares, então um par posterior pode casar com texto introduzido por um anterior (igual ao comportamento anterior ao G117).
- Alternativas consideradas:
  1. Novo código de erro `EDIT_PAIR_FAILED` com exit próprio. Rejeitado: um 26º código mexe em todas as tabelas de exit code da documentação e da skill sem benefício para o agente, e os campos estruturados carregam o diagnóstico.
  2. Enfiar o diagnóstico dos pares na string de mensagem do `InvalidInput`. Rejeitado: os agentes teriam de interpretar prosa, e o contrato são os campos estruturados.
  3. Tornar `--partial` o padrão. Rejeitado: a aplicação parcial silenciosa quebra a expectativa de atomicidade de todo chamador existente.
- Gatilho para revisitar: Se os agentes precisarem que `--partial` também continue após erros de *parse* (quantidades desiguais de `--old`/`--new`), ou precisarem de um exit code distinto para aplicação parcial (exit 2 pelo princípio Agent-First), revisitar o mapeamento de exit codes.
