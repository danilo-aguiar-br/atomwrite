# Guia de Testes do atomwrite


[Read in English](TESTING.md)


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças relevantes para testes em v0.1.12. A release adicionou 96 novos testes (eram 320 baseline) para um total de **445 testes em 43 suítes**. A v0.1.15 eleva o total para **542 testes** (+2 testes unitários na v0.1.14, +8 do G117 em `tests/cli_edit.rs`, +6 do G118 em `tests/cli_write.rs`).

### Novos Arquivos de Teste (10 Adicionados em v0.1.11+v0.1.12)

- `tests/cli_v012_regressions.rs` -- 11 testes para regressões v0.1.2 a v0.1.4
- `tests/cli_v012_audit_regressions.rs` -- 27 testes para auditoria G72/G114 de v0.1.12
- `tests/cli_v012_batch4_regressions.rs` -- 23 testes para o batch final de v0.1.12
- `tests/cli_v012_syntax_check.rs` -- 5 testes para validação G72 tree-sitter
- `tests/cli_v012_wal.rs` -- 8 testes para G114 sidecar WAL
- `tests/cli_v012_xattr_reflink.rs` -- 3 testes para G39 xattr + G64 reflink
- `tests/cli_set.rs` -- 6 testes para subcomando set v14 Tier 3
- `tests/cli_get_del.rs` -- 5 testes para subcomandos get/del v14 Tier 3
- `tests/cli_query.rs` -- 5 testes para subcomando query v14 Tier 3
- `tests/cli_outline.rs` -- 5 testes para subcomando outline v14 Tier 3
- `tests/cli_case.rs` -- 3 testes para subcomando case v14 Tier 3

### Novos Testes em Arquivos Existentes

- 12 testes em `src/binary_detect.rs::tests` (G41 content_inspector)
- 16 testes em `src/syntax_check.rs::tests` (G72 tree-sitter)
- 8 testes em `src/wal.rs::tests` (G114 WAL)
- 3 testes em `src/xattr_restore.rs::tests` (G39 xattr)
- 3 testes em `src/lock.rs::tests` (G54 advisory lock)
- 3 testes em `src/atomic.rs::tests` (WriteStrategy: rename/inplace/copyback)

### Organização das Suítes de Teste

- Total: 43 suítes (eram 34 em v0.1.10)
- Testes unitários em `src/`
- Testes de integração em `tests/cli_*.rs`
- Testes property-based em `tests/proptest_*.rs`
- Testes de snapshot via `insta`
- Testes de sinal (SIGINT, SIGTERM, SIGPIPE)

## O Que Há de Novo na v0.1.25 (Atual)

- 631 testes passando, 0 falhas, 3 ignorados (gate de cross-compile)
- 49 gaps adicionais resolvidos em 6 rodadas de auditoria e2e (GAP-071 a GAP-134)
- Testes property-based de fuzzy via proptest (5 propriedades em `tests/proptest_fuzzy.rs`)
- ~505 cenários e2e executados contra binário real

## O Que Há de Novo na v0.1.24

- 621 testes passando, 0 falhas, 3 ignorados (gate de cross-compile)
- 52 gaps resolvidos (GAP-2026-019 a GAP-2026-070)
- 20 `anyhow::bail!()` convertidos para `AtomwriteError` tipado (testes unitários inline por comando)
- 3 novos ADRs (0045-0047)

### Novos Arquivos de Teste (v0.1.24)

- `tests/cli_v0124_clap_suggestion.rs` — 5 testes para GAP-019 (sugestões acionáveis de erro clap)
- `tests/cli_gap020_diff_workspace.rs` — 3 testes para GAP-020 (diff resolve-first)
- `tests/cli_gap021_scope_readonly.rs` — 4 testes para GAP-021 (scope modo read-only)

### Correções Validadas por Suítes Existentes

- GAP-022 a GAP-070: correções validadas por testes de integração existentes nos arquivos `cli_*.rs`
- Cada conversão `bail!` → `AtomwriteError` inclui asserções `#[cfg(test)]` inline
- `cargo test` confirma 621 total: 609 (v0.1.23) + 5 (GAP-019) + 3 (GAP-020) + 4 (GAP-021)

## O Que Há de Novo na v0.1.23

- 609 testes passando (575 baseline v0.1.22 + 12 GAP-2026-015 + 7 GAP-2026-016 + 4 GAP-2026-017 + 8 GAP-2026-018 + 3 de [Unreleased] CI Windows)
- 33 subcomandos (32 da v0.1.22 + `verify` da v0.1.25)
- 26 ADRs em docs/decisions/ (0019-0044)
- GAP-2026-015 (allow_hyphen_values) fechado; 15 campos CLI em 8 structs agora aceitam valores iniciando com `-`
- GAP-2026-016 (backup-by-default) fechado; backup habilitado por padrão em 9 structs que mutam conteúdo
- GAP-2026-017 (guarda de shrink) fechado; writes que reduzem >50% bloqueados quando --expect-checksum ativo
- GAP-2026-018 (--old-file/--new-file) fechado; edit lê match/substituição de arquivos, contornando ARG_MAX
- Novos testes: `tests/cli_v0123_hyphen_values.rs` (12), `tests/cli_v0123_backup_default.rs` (7), `tests/cli_v0123_shrink_guard.rs` (4), `tests/cli_v0123_old_file.rs` (8)

## O Que Há de Novo na v0.1.22

- 631+ testes passando (542 baseline v0.1.18 + 33 na v0.1.21-v0.1.22 + 46 na v0.1.23-v0.1.24 + 10 na v0.1.25)
- 33 subcomandos (30 baseline v0.1.18 + `edit-loop` + `prune-backups` da v0.1.22 + `verify` da v0.1.25)
- 22 ADRs em docs/decisions/ (0019-0040)
- 2 novos schemas NDJSON: `edit-loop-output.schema.json`, `prune-backups-output.schema.json`

## O Que Há de Novo na v0.1.21

- GAP-2026-012 (flag `--allow-sequential-drift`) fechado; edits sequenciais não exigem mais re-capturar checksum entre iterações
- GAP-2026-013 Frente 4 (`--backup` exposto em `edit` e `rollback`) fechado; paridade com `write` e `replace`
- GAP-2026-014 v2 (backup é deletado por padrão após sucesso) fechado; flag opt-in `--keep-backup` adicionada
- ADR-0038 criado documentando o paradigma de backup que deleta após sucesso
- Novos testes: `tests/cli_v0121_sequential_drift.rs` (4), `tests/cli_v0121_backup_keep_flag.rs` (3), `tests/cli_v0121_edit_backup.rs` (3), `tests/cli_v0121_rollback_backup.rs` (2), `tests/cli_v0121_apply_keep.rs` (2), `tests/cli_v0121_batch_keep.rs` (2), `tests/proptest_v0121_backup_delete.rs` (2 property tests)

## O Que Há de Novo na v0.1.22

- GAP-2026-012 Frente 3 (sub-comando helper `edit-loop`) fechado; novo `edit-loop` aplica N pares `{old, new}` em 1 invocação via NDJSON no stdin
- GAP-2026-013 Frente 2 (sub-comando `prune-backups`) fechado; novo `prune-backups` oferece limpeza manual de arquivos `.bak.YYYYMMDD_HHMMSS` legados
- ADR-0039 criado documentando o design de `edit-loop`
- ADR-0040 criado documentando o design de `prune-backups`
- Novos testes: `tests/cli_v0121_edit_loop.rs` (4), `tests/cli_v0121_prune_backups.rs` (3)


### Como Executar

```bash
# Executar todos os 631+ testes
cargo test

# Executar apenas a suíte de regressão v0.1.12
cargo test --test cli_v012_regressions
cargo test --test cli_v012_audit_regressions
cargo test --test cli_v012_batch4_regressions

# Executar com output visível para debug
cargo test --test cli_v012_syntax_check -- --nocapture

# Executar o gate de cross-compile
cargo test --test cross_compile_check -- --ignored
```

### Cobertura

- 20.19% cobertura de linhas via `cargo tarpaulin` (935/4631 linhas cobertas)
- Menor que o ideal porque tarpaulin conta apenas testes unitários, não testes de integração CLI
- A suíte de testes de integração é a métrica primária de cobertura (631+ testes em 63+ suítes)

### Dependências Adicionadas

- `tree-sitter-language-pack = "1.8"` -- 305 linguagens para query/outline/syntax-check
- Todas as dependências de teste já estão na seção dev-dependencies

### ADRs e Schemas

- 22 novos ADRs em `docs/decisions/` (0019-0040) explicam as decisões arquiteturais por trás das features v0.1.12 a v0.1.22
- 29 schemas JSON em `docs/schemas/` (índice completo em `docs/schemas/README.md`); v0.1.22 adicionou `edit-loop-output` e `prune-backups-output`
- Veja [docs/decisions/README.md](README.md) para a lista completa de ADRs

## Por Que Testes Categorizados
- Cada categoria de teste valida uma camada diferente do sistema
- Testes unitários verificam lógica pura em isolamento
- Testes de integração verificam comportamento da CLI de ponta a ponta
- Testes de snapshot travam a estrutura de saída JSON
- Testes property-based descobrem edge cases que humanos perdem
- Executar todas as categorias juntas dá confiança de que atomwrite funciona corretamente


## Estatísticas Atuais
- 70+ arquivos Rust em `src/` e `tests/`
- **631+ testes no total em 63+ suítes** (unitários + integração + snapshot + property-based + sinal + tracing + NDJSON + regressão + cross-compile + concorrência)
- **96 novos testes adicionados em v0.1.11+v0.1.12**:
  - 11 testes em `tests/cli_v012_regressions.rs` (fixes GAP 13, GAP 14, GAP 18)
  - 27 testes em `tests/cli_v012_audit_regressions.rs` (auditoria v0.1.12 G72/G114)
  - 23 testes em `tests/cli_v012_batch4_regressions.rs` (batch final v0.1.12)
  - 5 testes em `tests/cli_v012_syntax_check.rs` (G72 tree-sitter)
  - 8 testes em `tests/cli_v012_wal.rs` (G114 WAL sidecar)
  - 3 testes em `tests/cli_v012_xattr_reflink.rs` (G39 xattr + G64 reflink)
  - 6 testes em `tests/cli_set.rs` (v14 Tier 3)
  - 5 testes em `tests/cli_get_del.rs` (v14 Tier 3)
  - 5 testes em `tests/cli_query.rs` (v14 Tier 3)
  - 5 testes em `tests/cli_outline.rs` (v14 Tier 3)
  - 3 testes em `tests/cli_case.rs` (v14 Tier 3)
  - 16 testes em `src/syntax_check.rs::tests` (G72 tree-sitter)
  - 8 testes em `src/wal.rs::tests` (G114 WAL)
  - 12 testes em `src/binary_detect.rs::tests` (G41 content_inspector)
  - 3 testes em `src/xattr_restore.rs::tests` (G39)
  - 3 testes em `src/lock.rs::tests` (G54 advisory lock)
- 2 novos testes de regressão de mtime em `src/atomic.rs::tests` (v0.1.3)
- 7 testes de sugestão de erro do GAP 13 em `src/error.rs::tests` (v0.1.4)
- 3 testes do gate de cross-compile em `tests/cross_compile_check.rs` (v0.1.4)
- 9 arquivos de snapshot em `tests/snapshots/`
- 2 arquivos de regressão proptest
- 2 alvos de fuzzing em `fuzz/fuzz_targets/`


## Categorias de Teste
### Testes Unitários (src/)
- Localizados dentro dos arquivos fonte via módulos `#[cfg(test)]`
- Testam funções puras sem interação com filesystem ou CLI
- `path_safety.rs` -- 5 testes para prevenção de path traversal e workspace jail
- `checksum.rs` -- corretura do hashing BLAKE3

### Testes de Integração (tests/cli_*.rs)
- Localizados no diretório `tests/`
- Cada arquivo testa um subcomando de ponta a ponta via binário compilado
- Usam `assert_cmd` para invocar atomwrite como subprocesso
- Usam a crate `tempfile` para fixtures isoladas de filesystem
- `cli_write.rs` -- 6 testes para operações de escrita atômica
- `cli_read.rs` -- 6 testes para leitura com metadados e checksums
- `cli_edit.rs` -- 5 testes para modos de edição cirúrgica
- `cli_search.rs` -- 6 testes para busca paralela
- `cli_replace.rs` -- 6 testes para substituição de texto em massa
- `cli_hash.rs` -- 4 testes para operações de checksum BLAKE3
- `cli_delete.rs` -- 4 testes para deleção de arquivo com backup
- `cli_count.rs` -- 2 testes para contagem de linhas e arquivos
- `cli_diff.rs` -- 4 testes para comparação de arquivos
- `cli_move.rs` -- 5 testes para move e rename atômico
- `cli_copy.rs` -- 4 testes para cópia com verificação de checksum
- `cli_list.rs` -- 4 testes para listagem de estrutura do projeto
- `cli_extract.rs` -- 4 testes para extração de campos NDJSON
- `cli_calc.rs` -- 5 testes para matemática e conversão de unidades
- `cli_regex.rs` -- 5 testes para geração de regex a partir de exemplos
- `cli_transform.rs` -- 5 testes para busca e reescrita AST
- `cli_scope.rs` -- operações de escopo: encontrar funções, deletar comentários, erro de linguagem desconhecida, padrões customizados
- `cli_backup.rs` -- criação de backup, dry-run, erro de arquivo não encontrado, múltiplos arquivos
- `cli_rollback.rs` -- restauração de backup, dry-run, erro sem backup disponível, flag de verificação
- `cli_apply.rs` -- substituição completa de arquivo, blocos SEARCH/REPLACE, diff unificado, dry-run
- `cli_batch.rs` -- 13 testes para execução de operações em batch (write, replace, delete, move, copy, alias path, rollback transacional)

### Testes de Snapshot (insta)
- Localizados em `tests/snapshot_write.rs` -- 9 testes
- Usam a crate `insta` para verificação de saída baseada em snapshot
- Travam a estrutura JSON exata de cada tipo de saída
- Arquivos de snapshot armazenados em `tests/snapshots/`
- Execute `cargo insta review` para aceitar ou rejeitar mudanças de snapshot
- Snapshots disponíveis:
- `snapshot_write__write_output_structure.snap`
- `snapshot_write__read_output_structure.snap`
- `snapshot_write__edit_output_structure.snap`
- `snapshot_write__search_match_structure.snap`
- `snapshot_write__replace_result_structure.snap`
- `snapshot_write__error_not_found_structure.snap`
- `snapshot_write__batch_summary_structure.snap`
- `snapshot_write__error_invalid_input_structure.snap`

### Testes de Regressão
- `tests/cli_v012_regressions.rs` -- 11 testes para regressões de v0.1.2 a v0.1.4
- `jail_suggestion_mentions_workspace_flag` -- atualizado na v0.1.4 (GAP 13): afirma que a sugestão menciona `--workspace` quando nenhum workspace é fornecido
- `gap13_jail_suggestion_when_workspace_supplied_says_inside` -- adicionado na v0.1.4 (GAP 13): afirma que a sugestão diz "inside the workspace" quando `--workspace` É fornecido
- Outras regressões cobrem scope, batch path/source aliases, fuzzy edit, search files dedup, edit multi NDJSON, clap JSON errors, json-schema, validate path jail, dead flags, mtime

### Gate de Cross-Compile (v0.1.4)
- `tests/cross_compile_check.rs` -- 3 testes protegendo compilação Windows-only
- `cross_compile_windows_gnu_x64_succeeds` -- afirma que `cargo check --target x86_64-pc-windows-gnu` sucede e não emite E0433, E0308, ou E0507
- `cross_compile_windows_gnu_i686_succeeds` -- afirma o mesmo para `i686-pc-windows-gnu` (Windows 32-bit)
- `cross_compile_windows_msvc_succeeds` -- afirma o mesmo para `x86_64-pc-windows-msvc` (toolchain Microsoft Visual C++)
- Todos os testes são `#[ignore]` por padrão para pular em hosts sem os targets Windows instalados
- Execute com `cargo test --test cross_compile_check -- --ignored`
- Pula graciosamente quando o linker necessário (lib.exe para MSVC, i686-w64-mingw32-gcc para 32-bit) está ausente
- Obrigatório antes de qualquer release que toque código `#[cfg(windows)]` (veja `docs/INSTALL.md` e `docs/CROSS_PLATFORM.md`)
- `snapshot_write__error_workspace_jail_structure.snap`

### Testes Property-Based (proptest)
- Localizados em `tests/proptest_checksum.rs` -- 2 testes
- Localizados em `tests/proptest_backup.rs` -- 2 testes
- Usam a crate `proptest` para gerar inputs aleatórios
- Verificam que checksums BLAKE3 são determinísticos para qualquer input
- Verificam que criação de backup funciona para conteúdos arbitrários de arquivo
- Arquivos de regressão armazenados ao lado dos arquivos de teste

### Testes de Sinal
- Localizados em `tests/signal_test.rs` -- 1 teste
- Verificam comportamento de shutdown gracioso na recepção de sinais

### Testes de Validação NDJSON
- Localizados em `tests/ndjson_valid_test.rs` -- 17 testes
- Validam estrutura de saída NDJSON para 20 de 21 comandos
- Incluem testes de interoperabilidade `jaq` verificando parsing de JSON via pipe
- Incluem teste i18n confirmando que `--lang` não altera saída JSON

### Testes de Concorrência
- Localizados em `tests/cli_concurrency.rs`
- Testam operações paralelas de arquivo e condições de corrida

### Testes de Tamanho Máximo de Arquivo
- Localizados em `tests/cli_max_filesize.rs`
- Testam enforcement de limites de tamanho de arquivo

### Testes de Tracing
- Localizados em `tests/tracing_test.rs`
- Verificam comportamento da infraestrutura de tracing/logging

### Alvos de Fuzzing
- Localizados em `fuzz/fuzz_targets/batch_parse.rs` -- fuzzing do parser NDJSON de batch
- Localizados em `fuzz/fuzz_targets/extract_json.rs` -- fuzzing do parser JSON de extract
- Requerem `cargo +nightly fuzz run <alvo>`
- Executar com `--max_total_time=30` para validação rápida


## Como Executar
### Executar Todos os Testes

```bash
cargo test
```

### Executar um Arquivo de Teste Específico

```bash
cargo test --test cli_write
cargo test --test cli_search
cargo test --test snapshot_write
cargo test --test proptest_checksum
```

### Executar um Teste Específico por Nome

```bash
cargo test --test cli_write test_write_atomic
cargo test --test cli_edit test_edit_old_new
```

### Executar Apenas Testes Unitários

```bash
cargo test --lib
```

### Executar Apenas Testes de Integração

```bash
cargo test --tests
```

### Executar Com Saída Visível

```bash
cargo test -- --nocapture
```

### Executar Revisão de Snapshots

```bash
cargo insta review
```

### Atualizar Todos os Snapshots

```bash
cargo insta test
cargo insta review
```


## Perfis de CI
### Rápido (Desenvolvimento)
- Execute `cargo test` com configurações padrão
- Pule testes property longos com `PROPTEST_CASES=10`
- Adequado para hooks de pre-commit
- Tempo total: abaixo de 30 segundos

### Completo (Pipeline CI)
- Execute `cargo test` com todas as features
- Execute `cargo clippy -- -D warnings` para linting
- Execute `cargo fmt -- --check` para formatação
- Execute `cargo insta test` para verificação de snapshots
- Execute testes property com contagem padrão de cases
- Adequado para validação de pull request
- Tempo total: abaixo de 2 minutos

### Release (Pré-Publicação)
- Execute `cargo test --release` para testes com build otimizado
- Execute `cargo test --test proptest_checksum` com `PROPTEST_CASES=1000`
- Execute `cargo test --test proptest_backup` com `PROPTEST_CASES=1000`
- Verifique que todos os snapshots estão atualizados com `cargo insta test`


## Variáveis de Ambiente
- `PROPTEST_CASES` -- número de cases de testes property (padrão: 256)
- `RUST_LOG` -- nível de tracing para saída debug durante testes
- `ATOMWRITE_LOG` -- define nível de tracing para debug de testes (ex: `debug`, `trace`)
- `INSTA_UPDATE` -- defina como `always` para atualizar snapshots automaticamente
- `RUST_TEST_THREADS` -- limita execução paralela de testes (útil para testes de I/O)
- `CARGO_TARGET_DIR` -- sobrescreve diretório target para builds de teste


## Solução de Problemas
### Mismatch de Snapshot
- Execute `cargo insta review` para ver o diff
- Aceite mudanças com `cargo insta accept`
- Rejeite e corrija com `cargo insta reject`
- Snapshots ficam em `tests/snapshots/`

### Regressão de Proptest
- Verifique arquivos `tests/*.proptest-regressions` para falhas registradas
- Estes arquivos contêm inputs mínimos de reprodução
- Deletar arquivos de regressão força re-descoberta (não recomendado)
- Corrija a causa raiz

### Testes Flakey
- Verifique problemas de timing do filesystem em testes de integração
- Use `--test-threads 1` para serializar execução de testes
- Verifique se `/tmp` tem espaço suficiente para arquivos temporários

### Erros de Permissão
- Testes de integração criam diretórios temporários
- Verifique acesso de escrita a `TMPDIR` ou `/tmp`
- Alguns testes verificam comportamento de permissão negada e precisam de diretórios pai graváveis
- Use `tempfile::tempdir()` para todas as fixtures de filesystem

### Teste Trava ou Expira
- Verifique loops infinitos no código sendo testado
- Use `RUST_TEST_THREADS=1` para executar sequencialmente
- Verifique que nenhum teste escreve em path que outro teste lê

### Gate de Cross-Compile Falha
- Confirme que o target Windows está instalado: `rustup target list --installed`
- Instale targets ausentes: `rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu x86_64-pc-windows-msvc`
- No Linux, instale o toolchain necessário: `mingw64-gcc` (Fedora) ou `mingw-w64` (Ubuntu)
- Para Windows 32-bit: instale `mingw32-gcc` separadamente
- Para MSVC: instale Visual Studio 2019+ Build Tools com workload C++ (lib.exe deve estar no PATH)
- Reexecute após corrigir: `cargo test --test cross_compile_check -- --ignored`
- O gate reporta linker ausente com snippet de stderr claro como "lib.exe" ou "i686-w64-mingw32-gcc"; case essa string com o toolchain ausente


## v0.1.20 — Novidades

Esta release introduz uma nova camada de segurança chamada **intention guards** e renomeia a flag global `--lang` para `--locale` para desambiguar do seletor tree-sitter `--lang` usado por `scope` e `transform`.

### Intention Guards (5 flags OPT-IN)

- `--require-backup <N>` — recusa a operação quando menos de `N` backups retidos existem para o alvo
- `--confirm` — emite um prompt de confirmação listando a mutação planejada em NDJSON antes de executar
- `--auto-rotate <N>` — rotaciona automaticamente o anel de backups para `N` entradas após uma escrita bem-sucedida
- `--risk-threshold <LOW|MEDIUM|HIGH>` — bloqueia operações cujo risco classificado atinge ou excede o threshold
- `--locale <en|pt-BR>` — renomeado de `--lang` para desambiguar do `--lang` tree-sitter

### Outras Adições

- `count --by-size` — lista os maiores arquivos da árvore com tamanhos e contagem de linhas
- `read --mode raw|envelope` — seleciona entre saída byte-stream e envelope NDJSON estruturado
- `search --no-begin-end` — desabilita a decoração implícita de âncoras `^` e `$` na saída regex
- `write --preserve-timestamps` — preserva o mtime do arquivo fonte ao sobrescrever
- `scope --lang rust` — alias explícito aceito para simetria ergonômica com `transform --lang`

### Estatísticas

- 542 testes passando em 47 suites de integração, 0 falhas
- 11 GAP-2026 fechados
- 3 targets de cross-compile Windows verdes
- 19 ADRs em `docs/decisions/` (0019-0037)

### Migração `--lang` para `--locale`

```bash
# Descobrir todos os arquivos com --lang
rg -l -- '--lang\b' .

# Substituir em massa preservando outros matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Ou via ruplacer
ruplacer --subvert --lang --locale
```
