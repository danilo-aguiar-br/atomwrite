# ADR-0022: G72 tree-sitter REAL substitui (não soma) a heurística de colchetes

- Status: Aceito
- Data: 2026-06-07
- Contexto: A v0.1.11 tinha uma heurística de balanceamento de colchetes no `atomic_write` para pegar erros de sintaxe óbvios antes de confirmar uma escrita. Era útil, mas gerava muitos falsos positivos (indentação do Python, template literals do JS) e falsos negativos (`import` do Python de um módulo inexistente). A v0.1.12 adiciona o G72, a verificação de sintaxe real com tree-sitter.
- Decisão: O G72 SUBSTITUI a heurística. `--syntax-check` no `write` invoca o tree-sitter para as linguagens conhecidas (24 cobertas) e recorre à heurística legada para extensões desconhecidas. O padrão do `write` é DESLIGADO (sem verificação de sintaxe) porque a verificação soma 50-200ms por escrita.
- Consequências:
  - + Quem ativa a verificação recebe erros de sintaxe precisos com linha/coluna/kind, não contagens de colchetes.
  - + A heurística de reserva continua funcionando em linguagens sem parser (por exemplo Dockerfile, .env).
  - - O padrão DESLIGADO faz a maioria dos usuários nunca ver o benefício. Adicionamos na v0.1.12 um ramo `cfg(debug_assertions)` que avisa a cada execução para incentivar o uso.
  - - O tree-sitter é permissivo: um arquivo que o tree-sitter processa sem erro não é necessariamente válido semanticamente (por exemplo, Rust que usa um nome indefinido ainda passa no parse). Documentamos isso.
- Alternativas consideradas:
  1. Manter a heurística e o tree-sitter lado a lado. Rejeitado: dois caminhos de código para manter, e os usuários teriam de escolher entre `--syntax-check=heuristic` e `--syntax-check=tree-sitter`.
  2. Rodar o tree-sitter sempre. Rejeitado: 50-200ms por escrita é inaceitável para operações em lote.
  3. Pré-aquecimento assíncrono dos parsers. Rejeitado: o design não tem daemon, e o custo do primeiro uso é amortizado pelo cache do tree-sitter-language-pack.
- Gatilho para revisitar: Se adicionarmos um daemon (`atomwrite daemon --syntax-warmup`), poderemos tornar isso o padrão e amortizar os 50-200ms.
