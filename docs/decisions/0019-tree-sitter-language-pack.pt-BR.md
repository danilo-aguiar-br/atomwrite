# ADR-0019: tree-sitter-language-pack em vez de dependência direta do tree-sitter

- Status: Aceito
- Data: 2026-06-07
- Contexto: A v0.1.12 introduz os subcomandos `query` e `outline` (v14 Tier 3) e o `--syntax-check` do `write` (G72). Os três recursos precisam de um parser tree-sitter. Era preciso decidir de qual crate depender.
- Decisão: Adotar `tree-sitter-language-pack = "1.8"` com `default-features = false` e `features = ["download", "dynamic-loading"]`. O recurso `download` permite que o pack baixe cada arquivo de parser `.so`/`.dylib`/`.dll` no primeiro uso. O recurso `dynamic-loading` mantém os parsers fora do tamanho estático do binário. As 305 gramáticas NÃO são embutidas.
- Consequências:
  - + O binário continua pequeno (~5-10 MB no Linux, contra ~50+ MB se os 305 parsers fossem embutidos pelo recurso `bundled`).
  - + Uma única dependência expõe 305 linguagens, sem declarar 5-10 crates `tree-sitter-{rust,python,js,...}` individuais.
  - + O download do primeiro uso fica em cache em `~/.cache/tree-sitter-language-pack/parsers/`, então a segunda execução é offline.
  - - A primeira chamada de cada linguagem paga uma ida e volta de rede mais o tempo de compilar/instalar o parser (~50-200ms por parser).
  - - Sistemas isolados da rede (air-gapped) precisam popular o cache manualmente.
  - - O `tree-sitter-language-pack` NÃO reexporta `tree_sitter::Query` (suporte a S-expression). O `query` da v14 aceita apenas nomes de kind, não S-expressions. Consultas S-expression reais ficam adiadas para a v0.1.13, se adicionarmos uma dependência direta do `tree-sitter`.
- Alternativas consideradas:
  1. `tree-sitter = "0.25"` + 5 crates `tree-sitter-{rust,python,javascript,typescript,go,toml,json,css,bash,markdown,yaml,html,...}` individuais. Rejeitado: 8+ dependências, gerenciamento manual de feature flags, sem solução de embutir parsers.
  2. `tree-sitter-language-pack` com o recurso `bundled`. Rejeitado: conjunto de parsers de 1+ GB, inviabiliza o cargo install.
  3. Lexer artesanal por linguagem. Rejeitado: reinventar 305 rodas é absurdo.
- Gatilho para revisitar: Se um usuário reportar problemas de latência no primeiro uso de cada linguagem, ou se um ambiente corporativo bloquear o endpoint de download, talvez seja preciso distribuir um pacote de parsers pré-populado como um crate separado `atomwrite-parsers`.
