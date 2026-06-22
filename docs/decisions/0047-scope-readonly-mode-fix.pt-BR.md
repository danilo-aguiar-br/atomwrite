# ADR-0047: Fix do modo read-only do scope e migração para find_all

- **Status**: Aceito
- **Date**: 2026-06-21
- **Context**: O subcomando `scope` reportava `files_matched: 0` quando invocado sem ação (`--delete`, `--action` ou `--replace-with`). A investigação revelou que a causa raiz NÃO era o mecanismo de matching (DFS + `match_node` funciona corretamente) mas sim a lógica de contagem de resultados. Quando nenhuma ação está configurada, `apply_scope_action()` retorna o texto original inalterado. Após aplicar as "edits" (que são operações identidade), `checksum_before == checksum_after`, fazendo o código contar o arquivo como "skipped" e nunca emitir evento `ScopeResult`. Isso tornava o modo read-only/auditoria do scope completamente inoperante — usuários não podiam listar quais nós AST casavam com um pattern sem também especificar uma mutação. Adicionalmente, o matching usava `root.dfs().filter(|node| pattern.match_node(node.clone()).is_some())` que ignora a pré-filtragem por `potential_kinds()` que `Node::find_all()` fornece.

- **Decision**: Duas mudanças aplicadas:
  1. **Branch de modo read-only**: Após coletar matches e antes do loop de edits, detectar `is_read_only = !delete && action.is_none() && replace_with.is_none()`. Quando true, emitir `ScopeResult` com contagem de matches, incrementar `files_matched`, e retornar early sem comparação de checksum ou tentativa de escrita.
  2. **Migração DFS → find_all**: Substituir o `root.dfs().filter(...)` manual por `root.find_all(&pattern)`. O método `find_all` (ast-grep-core 0.43.0, `Node::find_all`) chama `pat.potential_kinds()` uma vez e usa o resultado como filtro de curto-circuito antes de invocar `match_node` em cada candidato. Isso reduz chamadas desnecessárias a `match_node` em nós cujo `kind_id` nunca pode casar com a raiz do pattern. `find_all` retorna `Iterator<Item = NodeMatch>` onde `NodeMatch: Deref<Target = Node>`, então `.range()` funciona inalterado no código downstream.

- **Consequences**:
  - **+** `scope` sem ação agora reporta matches corretamente (modo read-only/auditoria funciona).
  - **+** Melhoria de performance pela pré-filtragem via `potential_kinds()` no `find_all`.
  - **+** Consistência com `transform` que usa `replace_all` (internamente usa o mesmo mecanismo Visitor/find_all).
  - **+** Todos os 30+ queries preparados produzem resultados em modo read-only.
  - **+** Import `MatcherExt` não utilizado removido.
  - **-** (nenhuma) Modo de mutação (`--delete`, `--action`, `--replace-with`) continua funcionando inalterado.

- **Alternatives considered**:
  1. **Apenas adicionar branch read-only, manter DFS manual.** Rejeitado: find_all é estritamente melhor (mesma semântica + performance da pré-filtragem por kind) e é a API idiomática do ast-grep-core.
  2. **Usar `Visitor` diretamente em vez de `find_all`.** Rejeitado: `Visitor` é mais baixo nível e requer mais boilerplate; `find_all` encapsula a mesma lógica em uma única linha.
  3. **Emitir warning quando nenhuma ação é especificada.** Rejeitado: modo read-only é um caso de uso legítimo (auditoria, contagem, listagem de matches) e deve funcionar silenciosamente.

- **Trigger to revisit**: Se ast-grep-core mudar a API do `find_all` ou se uma abordagem streaming/não-coletora for necessária para arquivos muito grandes.
