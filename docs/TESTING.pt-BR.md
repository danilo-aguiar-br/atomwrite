# Guia de Testes do atomwrite


[Read in English](TESTING.md)


## Por Que Testes Categorizados
- Cada categoria de teste valida uma camada diferente do sistema
- Testes unitários verificam lógica pura em isolamento
- Testes de integração verificam comportamento da CLI de ponta a ponta
- Testes de snapshot travam a estrutura de saída JSON
- Testes property-based descobrem edge cases que humanos perdem
- Executar todas as categorias juntas dá confiança de que atomwrite funciona corretamente


## Estatísticas Atuais
- 54 arquivos Rust em `src/` e `tests/`
- 5923 linhas de código
- 102 testes no total
- 7 arquivos de snapshot em `tests/snapshots/`
- 2 arquivos de regressão proptest


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
- `cli_batch.rs` -- 7 testes para execução de operações em batch

### Testes de Snapshot (insta)
- Localizados em `tests/snapshot_write.rs` -- 7 testes
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
