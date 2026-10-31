# ADR-0048: BackupOpts unificado e um único resolvedor resolve_backup()

> Histórico (pré-0.1.35): os knobs de produto `ATOMWRITE_*` / env descritos abaixo foram superados. A configuração de runtime é apenas flags de CLI + `config.toml` XDG / `atomwrite set|get`.


- Status: Aceito
- Data: 2026-07-06
- Contexto: As flags relacionadas a backup (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) eram declaradas à mão, separadamente, em cada uma das 15 structs de subcomandos mutadores (`write`, `edit`, `edit-loop`, `replace`, `transform`, `scope`, `apply`, `set`, `del`, `case`, `batch`, `delete`, `move`, `copy`, `rollback`), com presença e padrões divergentes entre as structs (GAP-CLI-SURFACE-DRIFT). Isso fazia `replace --retention` e `delete --no-backup` devolverem exit 2 `ARGUMENT_PARSE_ERROR`, porque a flag estava ausente ou declarada com tipo incompatível nessas structs específicas. Passar `--backup` e `--no-backup` juntos deixava a última flag vencer em silêncio, em vez de falhar rápido. `retention: 5` estava fixo no código em 9 pontos de chamada, em vez de fluir de uma única fonte da verdade, e as chaves `backup`/`retention` de `[defaults]` do `.atomwrite.toml` eram lidas mas nunca consultadas (GAP-CONFIG-DEFAULTS-DEAD), ver ADR-0049.
- Decisão: Extrair uma única struct `BackupOpts` (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) achatada via `#[command(flatten)]` nos 15 subcomandos mutadores. `--backup` e `--no-backup` agora são mutuamente exclusivas via `conflicts_with` do clap (exit 2), em vez de deixar a última flag vencer em silêncio. Uma única função `resolve_backup(opts: &BackupOpts, defaults: &DefaultsSection) -> ResolvedBackup` centraliza a resolução com a precedência: variável de ambiente `ATOMWRITE_BACKUP` > flags de CLI (`--backup`/`--no-backup`/`--retention`) > `[defaults]` do `.atomwrite.toml` > padrão embutido (`true` / `5`). Três exceções documentadas ao contrato unificado de padrão true:
  1. **`rollback`** mantém o snapshot de segurança pré-rollback como opt-in via `--backup` explícito (ele já lê de um backup existente, então uma etapa prévia com padrão true seria redundante).
  2. **`delete --keep-backup`** é redundante porque os backups de exclusão são sempre preservados e nunca removidos automaticamente, e passá-la agora faz surgir um campo `warnings` no envelope NDJSON, em vez de um no-op silencioso.
  3. **`move`/`copy`** que sobrescrevem um destino existente agora exigem `--force` OU um `--backup` explícito, pois antes os dois subcomandos impunham isso de forma inconsistente.
- Consequências:
  - + `replace --retention` e `delete --no-backup` agora são interpretados com sucesso nos 15 subcomandos.
  - + O conflito `--backup`/`--no-backup` falha rápido no parse (exit 2), em vez de escolher a última flag em silêncio.
  - + Não restam literais `retention: 5` fixos no código, e as implementações de `Default` em `config.rs` e `atomic.rs` leem `constants::DEFAULT_BACKUP_RETENTION`.
  - + `batch --retention <N>` e `batch --backup` agora são efetivos de ponta a ponta, incluindo a etapa transacional de pré-backup e a operação `delete` dentro de um batch.
  - - `delete`/`move`/`copy` agora têm uma MUDANÇA INCOMPATÍVEL de comportamento: o `delete` cria um backup por padrão (era opt-in), e o `move`/`copy` que sobrescreve exige `--force` ou `--backup`.
- Alternativas consideradas:
  1. Manter as flags de backup por struct e corrigir cada divergência individualmente. Rejeitado: o desvio se repetiria toda vez que um novo subcomando mutador fosse adicionado, e uma única struct achatada é a única forma de garantir paridade por construção.
  2. **Deixar `--backup`/`--no-backup` com a última flag vencendo (status quo).** Rejeitado: a precedência silenciosa é uma armadilha em pipelines de agentes que podem passar as duas flags em linhas de comando compostas.
- Gatilho para revisitar: Se um 16º subcomando mutador for adicionado, confirmar que ele achata o `BackupOpts` em vez de redeclarar as flags à mão.
