# ADR-0021: query/outline da v14 aceitam apenas nomes de kind, não S-expressions

- Status: Aceito
- Data: 2026-06-07
- Contexto: O tree-sitter expõe dois estilos de consulta: casamento por nome de kind (por exemplo `function_item`) e consulta S-expression (por exemplo `(function_item name: (identifier) @func)`). O `tree-sitter-language-pack` reexporta `Node` e `TreeCursor`, mas NÃO reexporta `tree_sitter::Query` (o parser de S-expression está em outro recurso de crate).
- Decisão: O `query` e o `outline` da v0.1.12 aceitam apenas nomes de kind. `query --kinds` enumera os kinds com contagens. `query --query <KIND>` emite os nós desse kind. `query --tree` despeja todos os nós nomeados. `outline --kind <KIND>` filtra itens estruturais.
- Consequências:
  - + Não é preciso um parser de S-expression, a API do language pack basta.
  - + Os usuários ganham um subconjunto útil de navegação por AST sem aprender a sintaxe de consulta do tree-sitter.
  - + A saída é estável entre versões menores do tree-sitter-language-pack.
  - - Usuários avançados não podem escrever `(call_expression function: (identifier) @fn)` para achar chamadas de função. Precisam rodar `query --kinds` antes e depois filtrar com `rg`.
  - - Expomos uma API por nome de kind que pode parecer "limitada" para quem domina AST.
- Alternativas consideradas:
  1. Adicionar `tree-sitter` como dependência direta ao lado do language pack. Rejeitado na v0.1.12 (adiciona uma dependência, mais tempo de compilação e mais superfície de teste). Considerar na v0.1.13.
  2. Usar o `tree-sitter-cli` para pré-compilar consultas na compilação. Rejeitado: exige `cc` na compilação e falha no `cargo install` a partir do tarball de código-fonte.
  3. Adiar o subcomando `query` inteiro para a v0.1.13. Rejeitado: o casamento por nome de kind já é útil sozinho, e perderíamos o ritmo esperando o suporte a S-expression.
- Gatilho para revisitar: Quando usuários perguntarem "como acho todas as chamadas de função?" ou quando o suporte a S-expression da v0.1.13+ for viável.
