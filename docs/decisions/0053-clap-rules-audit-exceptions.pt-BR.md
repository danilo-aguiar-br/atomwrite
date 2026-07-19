# ADR-0053: Exceções da auditoria de rules Clap (v0.1.32)

## Status

Aceito

## Contexto

A auditoria contra `rules_rust_cli_com_clap` fechou a maioria dos gaps OBRIGATÓRIO. Duas desvios intencionais permanecem.

## Decisão

### 1. `BackupOpts.backup: Option<bool>` (tri-estado)

Necessário para: ausente → config/default; `--backup` → forçar on; `--no_backup` → forçar off.

### 2. `prescan_json_schema` com `std::env::args` antes do Clap

Necessário para `--json-schema` sem validar args obrigatórios do subcomando. Paths normais usam só `Cli::try_parse()`.

## Consequências

Tratar como exceções documentadas em auditorias futuras. Ver versão EN para detalhes.
