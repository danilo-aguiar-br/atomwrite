# ADR-0047: correção do modo somente leitura do scope e migração para find_all

- Status: Aceito
- Data: 2026-06-21
- Contexto: O subcomando `scope` informava `files_matched: 0` quando invocado sem uma ação (`--delete`, `--action` ou `--replace-with`). A investigação revelou que a causa raiz NÃO era o mecanismo de casamento (DFS + `match_node` funciona corretamente), mas a lógica de contagem do resultado. Quando nenhuma ação está configurada, o `apply_scope_action()` devolve o texto original inalterado. Depois de aplicar as "edições" (que são operações de identidade), `checksum_before == checksum_after`, fazendo o código contar o arquivo como "pulado" e nunca emitir um evento `ScopeResult`. Isso tornou o modo somente leitura/auditoria do scope completamente inoperante: os usuários não conseguiam listar quais nós da AST casavam com um padrão sem também especificar uma mutação. Além disso, o casamento usava `root.dfs().filter(|node| pattern.match_node(node.clone()).is_some())`, que contorna o pré-filtro `potential_kinds()` que o `Node::find_all()` oferece.

- Decisão: Duas mudanças aplicadas:
  1. Ramo do modo somente leitura: Depois de coletar os casamentos e antes do loop de edição, detectar `is_read_only = !delete && action.is_none() && replace_with.is_none()`. Quando verdadeiro, emitir `ScopeResult` com a contagem de casamentos, incrementar `files_matched` e retornar cedo, sem comparação de checksum nem tentativa de escrita.
  2. Migração de DFS para find_all: Substituir o manual `root.dfs().filter(...)` por `root.find_all(&pattern)`. O método `find_all` (ast-grep-core 0.43.0, `Node::find_all`) chama `pat.potential_kinds()` uma vez e usa o resultado como filtro de curto-circuito antes de invocar `match_node` em cada candidato. Isso reduz chamadas desnecessárias a `match_node` em nós cujo `kind_id` nunca pode casar com a raiz do padrão. O `find_all` devolve `Iterator<Item = NodeMatch>`, em que `NodeMatch: Deref<Target = Node>`, então `.range()` funciona sem mudanças a jusante.

- Consequências:
  - + O `scope` sem ação agora informa corretamente os casamentos (o modo somente leitura/auditoria funciona).
  - + Ganho de desempenho com o pré-filtro `potential_kinds()` do `find_all`.
  - + Consistência com o `transform`, que usa `replace_all` (internamente usa o mesmo mecanismo Visitor/find_all).
  - + Todas as 30+ consultas preparadas produzem resultados quando invocadas no modo somente leitura.
  - + Removido o import `MatcherExt` não utilizado.
  - - (nenhuma) O modo de mutação (`--delete`, `--action`, `--replace-with`) continua funcionando sem mudanças.

- Alternativas consideradas:
  1. Apenas adicionar o ramo somente leitura, mantendo o DFS manual. Rejeitado: o find_all é estritamente melhor (mesma semântica + desempenho do pré-filtro por kind) e é a API idiomática do ast-grep-core.
  2. **Usar o `Visitor` diretamente em vez do `find_all`.** Rejeitado: o `Visitor` é de mais baixo nível e exige mais código repetitivo, enquanto o `find_all` encapsula a mesma lógica em uma linha.
  3. Emitir um aviso quando nenhuma ação for especificada. Rejeitado: o modo somente leitura é um caso de uso legítimo (auditar, contar, listar casamentos) e deve funcionar em silêncio.

- Gatilho para revisitar: Se o ast-grep-core mudar a API do `find_all` ou se uma abordagem em streaming/sem coleta for necessária para arquivos muito grandes.
