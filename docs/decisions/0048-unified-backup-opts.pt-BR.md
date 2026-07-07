# ADR-0048: BackupOpts unificado e resolver único resolve_backup()

- **Status**: Aceito
- **Date**: 2026-07-06
- **Context**: As flags relacionadas a backup (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) eram declaradas manualmente em separado em cada uma das 15 structs de subcomandos mutantes (`write`, `edit`, `edit-loop`, `replace`, `transform`, `scope`, `apply`, `set`, `del`, `case`, `batch`, `delete`, `move`, `copy`, `rollback`), com presença e defaults divergentes entre structs (GAP-CLI-SURFACE-DRIFT). Isso fazia `replace --retention` e `delete --no-backup` retornarem exit 2 `ARGUMENT_PARSE_ERROR` porque a flag estava ausente ou declarada com tipo incompatível nessas structs específicas. Passar `--backup` e `--no-backup` juntos deixava silenciosamente a última flag vencer, em vez de falhar rápido. `retention: 5` estava hardcoded em 9 call sites em vez de fluir de uma única fonte de verdade, e as chaves `backup`/`retention` do `[defaults]` do `.atomwrite.toml` eram parseadas mas nunca consultadas (GAP-CONFIG-DEFAULTS-DEAD) — ver ADR-0049.
- **Decision**: Extrair uma struct única `BackupOpts` (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) flattened via `#[command(flatten)]` nos 15 subcomandos mutantes. `--backup` e `--no-backup` agora são mutuamente exclusivos via `conflicts_with` do clap (exit 2) em vez de deixar a última flag vencer silenciosamente. Uma única função `resolve_backup(opts: &BackupOpts, defaults: &DefaultsSection) -> ResolvedBackup` centraliza a resolução com precedência: variável de ambiente `ATOMWRITE_BACKUP` > flags CLI (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > default embutido (`true` / `5`). Três exceções documentadas ao contrato unificado de default-true:
  1. **`rollback`** mantém seu snapshot de segurança pré-rollback opt-in via `--backup` explícito (já lê a partir de um backup existente, então um pré-passo default-true seria redundante).
  2. **`delete --keep-backup`** é redundante porque backups de deleção são sempre preservados e nunca auto-removidos; passá-la agora expõe um campo `warnings` no envelope NDJSON em vez de um no-op silencioso.
  3. **`move`/`copy`** sobrescrevendo um destino existente agora exigem `--force` OU um `--backup` explícito — anteriormente os dois subcomandos aplicavam essa regra de forma inconsistente.
- **Consequences**:
  - **+** `replace --retention` e `delete --no-backup` agora parseiam com sucesso em todos os 15 subcomandos.
  - **+** Conflito `--backup`/`--no-backup` falha rápido no parsing (exit 2) em vez de escolher silenciosamente a última flag.
  - **+** Zero literais `retention: 5` hardcoded restantes; os impls `Default` em `config.rs` e `atomic.rs` leem `constants::DEFAULT_BACKUP_RETENTION`.
  - **+** `batch --retention <N>` e `batch --backup` agora são efetivos ponta a ponta, incluindo o pré-passo transacional de backup e a operação `delete` dentro de um batch.
  - **-** `delete`/`move`/`copy` agora têm uma mudança BREAKING de comportamento: `delete` cria backup por padrão (era opt-in), e `move`/`copy` sobrescrever exige `--force` ou `--backup`.
- **Alternatives considered**:
  1. **Manter flags de backup por struct, corrigir cada divergência individualmente.** Rejeitado: a deriva recorreria toda vez que um novo subcomando mutante fosse adicionado; uma struct única flattened é a única forma de garantir paridade por construção.
  2. **Deixar `--backup`/`--no-backup` com última-flag-vence (status quo).** Rejeitado: precedência silenciosa é uma armadilha em pipelines de agentes que podem passar ambas as flags em linhas de comando compostas.
- **Trigger to revisit**: Se um 16º subcomando mutante for adicionado, confirmar que ele flattens `BackupOpts` em vez de redeclarar as flags manualmente.
