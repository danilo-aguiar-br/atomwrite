# ADR-0075 — Pacote de gramática 1.22.1, ast-grep 0.51, tree-sitter 0.27 e MSRV 1.90

## Status
- Aceita (2026-10-28)
- Emenda a ADR-0019 na versão do pack (1.8.1 para 1.22.1), na contagem de gramáticas (305 para 372) e no caminho do cache (`v1.8.1` para `v1.22.1`), então os números 1.8 daquele registro ficam como história datada

## Contexto
- O `cargo audit --deny warnings` reportou 4 vulnerabilidades e 10 avisos de unsound contra `zstd 0.13.3`, `zstd-safe 7.2.4` e `zstd-sys 2.0.16`, e o `cargo deny check` reprovava o portão `advisories` com o mesmo conjunto.
- Os identificadores reportados foram RUSTSEC-2026-0391 (um stream abandonado vaza para o seguinte), 0398 (`Seekable::decompress` põe o comprimento do `Vec` além da capacidade), 0403 (memória não inicializada numa moldura legada que falha), 0410 (o shim de alocador wasm corrompe memória), mais 0395, 0396, 0397, 0399, 0400, 0401, 0402, 0404, 0405 e 0406 como unsound.
- Nenhuma dessas três crates é dependência direta. Elas chegam só pelo `tree-sitter-language-pack 1.8.1`, que liga a feature `download` do pacote e com ela `ureq`, `rustls`, `ring` e o caminho de compressão `zstd`.
- O upstream corrigiu as falhas em `zstd 0.14.3`, `zstd-safe 8.2.0` e `zstd-sys 2.1.2`, então nenhuma versão da linha antiga é segura.

## Decisão
### 1. Ir para `tree-sitter-language-pack` 1.22.1
- É a versão publicada mais alta, e ela puxa `zstd 0.14.3`, que encadeia `zstd-safe ^8.2.0` e `zstd-sys ^2.1.2`, então o conjunto todo de avisos zera sem `[patch]` e sem fixar pacote antigo.
- O pacote mantém `get_parser`, `get_language`, `has_language`, `cache_dir`, `init`, `Error::LanguageNotFound` e os reexports `Parser`, `Node`, `TreeCursor` e `tree_sitter::Language`, então `src/grammar.rs`, `src/syntax_check.rs`, `src/commands/query.rs`, `src/commands/outline.rs` e `src/commands/doctor.rs` não mudam.
- O pacote acrescenta `fd-lock 4` e `dirs 7`. O `dirs 7` resolve pelo mesmo `dirs-sys 0.5` que o `directories 6` já usa, então o check de duplicata continua limpo.
- O cache de gramática é indexado pela versão do pacote, então a subida move o cache de `.../tree-sitter-language-pack/v1.8.1/libs` para `.../v1.22.1/libs` e as gramáticas baixam de novo no primeiro uso. Isso é o custo documentado da mudança, não defeito.

### 2. Ir para `ast-grep-core` e `ast-grep-language` 0.51
- O pacote 1.19 e acima exige `tree-sitter ^0.27`, e o `ast-grep` 0.43 exige `tree-sitter ^0.26`, então os três sobem juntos ou o grafo fica com dois majors de `tree-sitter` e o `multiple-versions = "deny"` reprova.
- O `ast-grep-core 0.51` mantém `AstGrep::new`, `replace_all`, `Pattern::try_new`, `Edit { position, deleted_length, inserted_text }` e `Language`, então `src/commands/transform.rs`, `src/commands/scope/mod.rs` e `src/lang_utils.rs` não mudam.

### 3. Ir para `tree-sitter` 0.27 e ajustar a única chamada que mudou
- `Query::new(&Language, &str)`, `Parser::set_language`, `QueryCursor::matches` e `StreamingIterator` mantêm a forma.
- A única quebra é `QueryMatch::captures`, que virou campo privado atrás do método `captures()`; o `src/commands/query.rs` agora chama o método.

### 4. Subir `rust-version` para 1.90
- `ast-grep-core 0.51` e `tree-sitter 0.27` declaram 1.90 como piso. Um `rust-version` abaixo disso afirmaria uma toolchain com que o grafo não compila.
- O `rust-toolchain.toml` mantém o canal 1.94, então a correção não muda a toolchain de desenvolvimento.

## Consequências
- `cargo audit --deny warnings` e `cargo deny check` passam sem entrada na lista de ignore, então o portão mantém o significado.
- Uma máquina que já baixou gramáticas paga um novo download das que faltam depois da subida, para o diretório versionado novo. O `atomwrite doctor` informa a contagem antes e depois.
- Qualquer subida futura do pacote repete esse download. É o preço de um cache que não pode misturar gramáticas compiladas contra majors diferentes de `tree-sitter`.
- A declaração de MSRV muda em todo documento que a carrega: os dois readmes, `CONTRIBUTING`, `SECURITY`, `ARCHITECTURE`, `docs/INSTALL`, `docs/CROSS_PLATFORM`, `docs/HOW_TO_USE`, `docs/COOKBOOK`, os dois pares `llms`, o par `llms-full` e `src/lib.rs`.
