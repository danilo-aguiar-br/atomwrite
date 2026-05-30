[Read in English](CHANGELOG.md)


# Changelog

- Todas as mudanças notáveis deste projeto são documentadas neste arquivo
- O formato segue [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
- O versionamento segue [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)


## [Unreleased]

### Fixed
- `search --include`/`--exclude` agora filtram arquivos corretamente via OverrideBuilder (antes era silenciosamente ignorado)
- `replace --include`/`--exclude` agora filtram arquivos corretamente via OverrideBuilder
- `transform --include`/`--exclude` agora filtram arquivos corretamente via OverrideBuilder
- `search --context` agora emite linhas de contexto via SearchSink customizado
- `search --max-count` agora limita matches por arquivo via SearcherBuilder.max_matches()
- `search --invert` agora mostra linhas sem correspondência via SearcherBuilder.invert_match()
- `search --sort` agora ordena resultados por caminho de arquivo
- `transform` agora processa arquivos em paralelo via WalkParallel + crossbeam channel
- `read` timestamp de modificação agora retorna formato ISO 8601 em vez de epoch seconds
- `batch delete` backup agora usa create_backup() atômico com fsync
- `create_backup` agora usa `fs::copy` em vez de `fs::hard_link` para prevenir corrupção de backup quando original é sobrescrito in-place
- Códigos de saída em `output.rs`, `read.rs`, `batch.rs` e `hash.rs` movidos de números mágicos para constantes nomeadas em `constants.rs`
- `DETECTION_SIZE` em `binary_detect.rs` centralizado para `BINARY_DETECT_SIZE` em `constants.rs`
- Seis chamadas `unwrap()` em `edit.rs` modo multi-edit substituídas por `ok_or_else` para tratamento de erro mais seguro
- Join de thread em `scope.rs` alterado de `unwrap()` para `let _ = join()` para prevenir propagação de panic
- `unwrap()` em `rollback.rs` substituído por `expect` com mensagem descritiva
- Documentação `# Errors` adicionada em três funções públicas de `output.rs` que retornam `Result`
- Dados de teste em português em `file_io.rs` substituídos por inglês

### Added
- `batch` suporta 7 operações: write, replace, delete, edit, hash, move, copy
- `batch --transaction` flag para execução tudo-ou-nada com rollback automático
- `edit --fuzzy` flag com cascata de 7 estratégias de correspondência (exact, line_trimmed, whitespace_normalized, indent_flexible, escape_normalized, trimmed_boundary, block_anchor)
- `edit --multi` flag para aplicar múltiplas operações de edição NDJSON em uma única escrita atômica
- `edit` saída NDJSON inclui campos fuzzy, strategy, strategies_tried, similarity quando correspondência fuzzy é usada
- Subcomando `scope` para escopo gramatical: selecionar categorias AST (comentários, funções, strings, etc.) e aplicar ações (delete, upper, lower, titlecase, squeeze, replace)
- `scope` suporta Rust (30 queries preparadas), Python (13), JavaScript/TypeScript (11), Go (8) e padrões AST customizados via `--pattern`
- Subcomando `backup` para criar backups de arquivos com timestamp, checksums BLAKE3 e retenção configurável
- Subcomando `rollback` para restaurar arquivos a partir de backups anteriores com verificação BLAKE3 opcional
- Subcomando `apply` para aplicar patches do stdin com detecção automática de formato (unified diff, blocos SEARCH/REPLACE, diff com fence markdown, substituição completa de arquivo)
- Flag `--line-ending lf|crlf|cr|auto` em `write` e `edit` para normalização de terminadores de linha
- Detecção de FIFO e arquivos de dispositivo na validação de caminho (códigos de saída 85 e 86)
- Detecção de hardlink antes do rename atômico com `tracing::warn` quando nlink > 1
- Detecção de mesmo arquivo em `copy` e `move` para prevenir perda de dados quando origem=destino
- Módulo de detecção e normalização de terminadores de linha (`line_endings.rs`)
- 60 testes unitários em 8 módulos (eram 5 testes em 1 módulo)
- Testes de integração para `backup`, `rollback`, `apply` e `scope`
- `deny.toml` para auditoria de licenças e advisories via cargo-deny
- Flag global `--lang` para override de locale (en, pt-BR) com variável de ambiente `ATOMWRITE_LANG`
- Suporte a i18n via `rust-i18n` e `sys-locale`: detecção automática de locale do SO com traduções en e pt-BR
- Headers de licença SPDX (`MIT OR Apache-2.0`) em todos os 64 arquivos `.rs`
- Documentação `//!` em nível de módulo em todos os 38 módulos fonte
- Doctests executáveis para funções `is_binary`, `detect` e `normalize`
- Documentação bilíngue `ARCHITECTURE.md` (en + pt-BR) descrevendo mapa de módulos, fluxo de dados e decisões chave
- Configuração `[package.metadata.docs.rs]` para builds no docs.rs
- Campo `documentation` e `[badges.maintenance]` no `Cargo.toml`
- Lints rustdoc: `broken_intra_doc_links`, `private_intra_doc_links`, `clippy::doc_markdown`
- `doc(html_root_url)` para cross-linking no docs.rs


## [0.1.0] - 2026-05-29
### Added
- 22 subcomandos: `read`, `write`, `edit`, `search`, `replace`, `hash`, `delete`, `count`, `diff`, `move`, `copy`, `list`, `extract`, `calc`, `regex`, `transform`, `batch`, `completions`, `scope`, `backup`, `rollback`, `apply`
- Pipeline de escrita atômica: tempfile + fsync + rename + fsync do diretório em toda operação de escrita
- Checksums BLAKE3 em toda resposta de `read` e `write`
- Locking otimista via flag `--expect-checksum`
- Contrato de saída NDJSON: toda linha do stdout é um objeto JSON com discriminador `"type"`
- Respostas de erro estruturadas no stdout com `error: true`, incluindo campos `error_class`, `retryable` e `suggestion`
- Busca paralela de arquivos com motor ripgrep (`grep-regex`, `grep-searcher`, `grep-matcher`)
- Travessia respeitando `.gitignore` via crate `ignore`
- Busca e reescrita estrutural por AST via ast-grep cobrindo 306 linguagens
- Geração de regex a partir de exemplos via grex
- Avaliação de expressões matemáticas e conversões de unidade via fend-core
- Saída de diff unificado via crate `similar`
- Leitura de arquivos via memory-map com `memmap2` para arquivos grandes
- Jail de workspace via `--workspace` para prevenir escape de caminhos
- Bloqueio de symlinks para prevenir travessia fora dos limites do workspace
- Operações em lote a partir de manifestos NDJSON suportando write, replace e delete
- Geração de completions de shell para bash, zsh, fish, elvish e PowerShell
- Tratamento de sinais para SIGINT, SIGPIPE e SIGTERM com shutdown limpo
- Suporte cross-platform: Unix (nix, libc) e Windows (windows-sys)
- 20 códigos de saída distintos para classificação precisa de erros
- Suporte a variável de ambiente `NO_COLOR`
- Variável de ambiente `RUST_LOG` para controle de verbosidade de logs
- Perfil release com LTO, codegen unit único, stripping de símbolos e panic=abort


[Unreleased]: https://github.com/comandoaguiar/atomwrite/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/comandoaguiar/atomwrite/releases/tag/v0.1.0
