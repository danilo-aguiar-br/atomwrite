# ADR-0053: Exceções da auditoria de rules do Clap (v0.1.32)

## Status

Aceito

## Contexto

A auditoria contra `rules_rust_cli_com_clap` / memória GraphRAG `rules-rust-cli-com-clap` fechou a maioria dos gaps OBRIGATÓRIO (`main` enxuto, features explícitas do clap, `author`, `value_hint`, `ArgAction::SetTrue`, `help_heading` em Backup, teste dedicado de `debug_assert`, CONTRIBUTING/CODE_OF_CONDUCT).

Dois desvios intencionais permanecem.

## Decisão

### 1. `BackupOpts.backup: Option<bool>` (tri-estado)

O texto da rule proíbe `Option<bool>` para flags booleanas. O atomwrite precisa de três estados:

| CLI | Significado |
|-----|-------------|
| (ausente) | Usa `[defaults].backup` do `.atomwrite.toml` ou o padrão embutido `true` |
| `--backup` | Força ligado (`Some(true)` via `ArgAction::SetTrue`) |
| `--no-backup` | Força desligado (`conflicts_with = "backup"`) |

Um `bool` simples não consegue representar “seguir a config”. Documentado no campo e coberto por `tests/cli_v0128_backup_matrix.rs`.

### 2. `runtime::prescan_json_schema` usa `std::env::args` antes do Clap

O texto da rule proíbe parsing manual de argv antes do Clap. Exceção: os agentes precisam despejar o JSON Schema de um subcomando via `--json-schema` mesmo quando o resto do argv falharia na validação de argumentos obrigatórios. Os caminhos de comando normais continuam usando exclusivamente `Cli::try_parse()`. Documentado em `src/runtime.rs`.

## Consequências
- Auditorias futuras devem tratar esses desvios como exceções documentadas, não como violações silenciosas.
- Evitar ampliar a varredura de argv pré-Clap além do prescan de schema.
- Se o Clap ganhar um “despejo de schema sem argumentos obrigatórios” de primeira classe, remover o caminho do prescan.

## Relacionados
- `src/cli_args.rs` (`BackupOpts`)
- `src/runtime.rs` (`prescan_json_schema`)
- `tests/cli_clap_debug_assert.rs`
- Geração de man page pelo `xtask` via `clap_mangen`
