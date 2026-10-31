# ADR-0024: o caminho pontilhado TOML do get/del usa descida manual de Table, não toml_edit::Table::get

- Status: Aceito
- Data: 2026-06-07
- Contexto: A v0.1.12 introduz `atomwrite get <path> <key-path>` e `atomwrite del <path> <key-path>`. O caminho da chave pode ser pontilhado como `package.version` ou aninhado como `package.dependencies.serde`. O ingênuo `doc.get("package.version")` devolve `None` porque `toml_edit::Document::get` procura a chave literal `"package.version"`, não a chave aninhada.
- Decisão: Escrevemos os helpers `get_toml_path(doc, key_path)` e `remove_toml_path(doc, key_path)`, que dividem manualmente o caminho da chave em `.` e descem pelos segmentos de `Table`. Para JSON usamos `serde_json::Value::pointer` (que já suporta caminhos pontilhados).
- Consequências:
  - + O comportamento é consistente com a semântica do JSON pointer, e os usuários não precisam aprender duas convenções de navegação.
  - + Evitamos (por enquanto) puxar o crate `dotted-lexer` ou `toml-path`.
  - + As mensagens de erro são claras: `key "package.version" not found at segment 2: no such key "version"` em vez de `Option::None`.
  - - A descida manual não trata chaves TOML entre aspas com `.` literal dentro. Documentamos isso como limitação conhecida.
  - - Realoca um `Vec<&str>` a cada consulta. O microbenchmark mostra <1 µs por chamada, aceitável.
- Alternativas consideradas:
  1. Usar `toml_edit::Table::get_many` (se existisse). Rejeitado: o toml_edit não tem essa API, o mais próximo é `Item::as_table_mut()` mais a descida manual.
  2. Usar o crate `toml` (não o `toml_edit`). Rejeitado: o `toml` não preserva comentários/formatação, e perderíamos a promessa `comments_preserved: true` da v14 Tier 3.
  3. Exigir que os usuários passem caminhos TOML como arrays (`["package", "version"]`). Rejeitado: a string pontilhada é a convenção de fato (igual a JSON pointer, jq, kubectl, terraform).
- Gatilho para revisitar: Se o toml_edit adicionar `get_many` ou uma API parecida, podemos simplificar. Se os usuários pedirem suporte a chaves entre aspas, nós o adicionamos.
