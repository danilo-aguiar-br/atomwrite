# 0032-query-sexp-real-implementation (PT-BR) — Tradução

_Tradução automática do ADR original em inglês. Em caso de divergência, a versão EN prevalece._

# 0032 — G122: Real S-expression matching in `query` sub-comando

- Status: Accepted
- Date: 2026-06-14
- Scope: `src/commands/query.rs`, `Cargo.toml`, `testes/cli_v019_query_sexpr.rs`

## Contexto

O subcomando `query` (v14 Tier 3, introduzido na v0.1.12) sempre prometeu
suporte a S-expression. A documentação em `COOKBOOK.md`, `HOW_TO_USE.md`
e nas skills bilíngues (`skill/atomwrite-{en,pt}/SKILL.md`) mostra
exemplos como:

```bash
atomwrite --workspace . query src/main.rs \
  --query "(function_item name: (identifier) @name)"
```

Na prática, `cmd_query` chamava `walk_kind_filter`, que faz
`wanted.iter().any(|w| w == &kind)` — comparação literal de STRING com
`node.kind()`. O kind inteiro
`"(function_item name: (identifier) @name)"` nunca casava com nenhum
`node.kind()` real, então a feature S-expression nunca funcionou.

`tree-sitter` 0.26.9 já estava disponível através de `tree-sitter-language-pack`
1.8. Faltava apenas rotear padrões S-expression para
`tree_sitter::Query::new` + `QueryCursor`.

## Decisão

Adicionar detecção automática de S-expression no padrão, e rotear para o
caminho real do `tree_sitter` quando detectado:

1. Novo enum `QueryType { KindFilter, SExpression }` em
   `src/commands/query.rs`.
2. Nova função `classify_pattern(padrão) -> QueryType` que detecta S-expression
   pela presença de `(`, `)`, ou `@`.
3. Nova função `walk_sexpr` que compila o padrão através de
   `tree_sitter::Query::new`, executa através de `QueryCursor::matches`, e emite
   `query_match` NDJSON com o campo extra `capture_name` para cada
   `@capture` do padrão.
4. `cmd_query` ramifica em `classify_pattern` para chamar
   `walk_kind_filter` (legado) ou `walk_sexpr` (novo).
5. Adicionar `tree-sitter = "0.26"` como dependência direta em
   `Cargo.toml` (a language-pack reexporta `Language` mas
   `Query`/`QueryCursor`/`StreamingIterator` exigem o crate `tree-sitter`
   próprio).
6. O `walk_sexpr` re-parseia a fonte através de um `tree_sitter::Parser` direto
   em vez do wrapper da language-pack, porque o `QueryCursor::matches`
   API exige `tree_sitter::Node<'_>` (não `tree_sitter_language_pack::Node`).

## Alternativas Consideradas

- **Manter apenas kind-filter** (alinhava com o código): rejeitado porque
  quebra a promessa documentada há 6 releases e é a feature mais útil
  do `query` para agentes LLM (captura de nomes de funções/structs).
- **Flag separada `--sexp`**: rejeitado porque adiciona uma segunda
  flag sem benefício real, e a auto-detecção por caracteres
  estruturais é trivial e inequívoca.
- **Forçar kind-filter para `SExpression` ser padrão**: rejeitado
  porque quebraria o padrão v0.1.12 `atomwrite --query function_item`
  que é usado em pipelines existentes.

## Consequências

- Pattern com `(`, `)`, ou `@` agora é roteado para `tree_sitter::Query::new`.
  Erros de parsing S-expression (e.g. `(unclosed`) retornam saída 1 com
  mensagem `invalid S-expression padrão: ...` (através de `anyhow::Context`).
- Resposta NDJSON inclui o campo `capture_name` para cada captura
  (e.g. `name` para `@name`).
- O caminho kind-filter legado é preservado bit-a-bit: usuários que
  passavam `--query function_item` continuam recebendo os mesmos
  resultados de antes.
- Re-parse dentro de `walk_sexpr` adiciona um overhead mínimo por
  chamada (parser em memória, fonte pequena), aceitável para
  query em arquivo único. Para batch de arquivos, considerar
  cache do `Query` em iteração futura (YAGNI por agora).

## Validação

- 4 novos testes em `testes/cli_v019_query_sexpr.rs`:
  - `sexpr_function_item_returns_main` — padrão
    `(function_item name: (identifier) @name)` casa em arquivo com
    `fn main`.
  - `sexpr_with_capture_returns_captured_text` — campo `text` da
    captura é exatamente `"main"`.
  - `sexpr_struct_item` — padrão
    `(struct_item name: (type_identifier) @name)` extrai `"Point"`.
  - `kind_filter_still_works` — padrão `function_item` (sem S-expression)
    continua usando o caminho legado e não emite `capture_name`.
- `cargo check` e `cargo build` passam com `tree-sitter = "0.26"`
  adicionado como dependência.


---

_Original em inglês: [`0032-query-sexp-real-implementation.md`](0032-query-sexp-real-implementation.md)_
