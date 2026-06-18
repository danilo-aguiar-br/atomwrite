# Guia de Migração: v0.1.21 para v0.1.22

- **Público alvo**: operadores e scripts CI que invocam `atomwrite` via shell, Make ou outras camadas de automação
- **Escopo**: 2 novos sub-comandos (`edit-loop`, `prune-backups`) e 2 novos ADRs (0039, 0040)
- **Tempo de leitura**: 5 minutos
- **Ação necessária**: nenhuma para scripts existentes; adoção opcional dos novos sub-comandos para casos de uso de edição sequencial e limpeza de backups

## O Que Há de Novo

Esta release adiciona 2 novos sub-comandos à v0.1.22. Ambos são aditivos: scripts existentes que não invocam os novos sub-comandos continuam se comportando exatamente como na v0.1.21.

### `edit-loop` — N pares em uma invocação

- **Caso de uso**: aplicar um lote de transformações textuais em um arquivo (renomear identificador em 7 lugares, varrer aliases obsoletos, refatorar paths de imports) onde hoje você invocaria `edit` N vezes em loop shell.
- **Antes (v0.1.21)**: loop shell com N chamadas `edit`; cada invocação paga o custo completo de startup do subprocesso (clap parse, leitura BLAKE3, re-validação `--expect-checksum`, pipeline de escrita, emissão do envelope NDJSON).
- **Depois (v0.1.22)**: 1 invocação `edit-loop` lê o arquivo uma vez, aplica todos os N pares em memória e escreve uma vez.

```bash
# v0.1.21 — 5 chamadas edit, 5 spawns de processo, 5 re-leituras de checksum
for par in "$@"; do
  CS=$(atomwrite --workspace . read src/foo.rs | jaq -r '.checksum')
  echo "${par#*=}" | atomwrite --workspace . edit \
    --expect-checksum "$CS" \
    --old "${par%%=*}" --new "${par#*=}" src/foo.rs
done

# v0.1.22 — 1 invocação, 1 leitura, 1 escrita
printf '%s\n' \
  '{"old":"foo","new":"bar"}' \
  '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace . edit-loop src/foo.rs
```

- **Formato NDJSON de entrada**: um `{"old":"...","new":"..."}` por linha
- **Saída NDJSON**: uma linha `pair_result` por linha de entrada (`matched: true|false`) mais uma linha `summary` final com `pairs_total`, `pairs_matched`, `pairs_unmatched`
- **Flags**: `--workspace`, `--expect-checksum`, `--partial`, `--fuzzy`, `--line-ending`, `--preserve-timestamps`, `--backup`, `--keep-backup`, `--retention`
- **Códigos de saída**: 0 se todos casaram (ou `--partial` com ≥1 casado), 1 se zero matches (NO_MATCHES), 65 se qualquer precondição falhou

### `prune-backups` — limpeza manual de anéis de backup legados

- **Caso de uso**: operadores que atualizaram de v0.1.20 herdam siblings `.bak.<timestamp>` de cada escrita `--backup` emitida sob v0.1.20 (a auditoria classificou esses como lixo transitório que o operador pode limpar no seu tempo).
- **Antes (v0.1.21)**: nenhum comando para limpar arquivos `.bak.*` legados. O ciclo de vida da v0.1.21 lida apenas com deleção pós-sucesso do backup recém-criado (ADR-0038).
- **Depois (v0.1.22)**: sub-comando `prune-backups` explícito com default `--dry-run true` para segurança.

```bash
# v0.1.22 — listar o que seria removido (seguro; default dry-run true)
atomwrite --workspace . prune-backups --max-age 86400 .

# v0.1.22 — remover de fato backups mais antigos que 24 horas
atomwrite --workspace . prune-backups --max-age 86400 --dry-run false .

# v0.1.22 — manter apenas os 3 backups mais recentes por diretório
atomwrite --workspace . prune-backups --max-count 3 --dry-run false .
```

- **Flags**: `--max-age <SECONDS>` (deleta backups mais antigos que N; default 0 = todos), `--max-count <N>` (mantém no máximo N mais recentes por diretório; default 0 = ilimitado), `--dry-run` (default `true`; passe `false` para deletar de fato)
- **Saída NDJSON**: uma linha por backup inspecionado (`path`, `age_secs`, `size_bytes`, `action: deleted|kept|would_delete`) mais uma linha `summary` com `scanned`, `deleted`, `kept`, `elapsed_ms`, `dry_run`
- **Códigos de saída**: 0 se o scan completou, 1 se nenhum backup encontrado (NO_MATCHES), 65 se precondição falhou
- **Segurança**: recusa rodar sem `--max-age` ou `--max-count` (aborta com `InvalidInput`); `--dry-run` default `true` torna o comando seguro para colar de uma thread de chat

## Notas Não-Quebrando

- Todos os 32 sub-comandos da v0.1.21 permanecem disponíveis com assinaturas e comportamento idênticos.
- Todas as semânticas de flags são preservadas. Nenhuma flag foi renomeada, removida ou teve seu default alterado.
- Schemas do envelope NDJSON para sub-comandos existentes permanecem inalterados. Os dois novos schemas (`edit-loop-output.schema.json`, `prune-backups-output.schema.json`) são aditivos.
- Comportamento default `keep_backup: false` da v0.1.21 é preservado (backups deletados após sucesso a menos que `--keep-backup` seja passado).
- `--allow-sequential-drift` opt-in da v0.1.21 permanece o padrão recomendado para edits sequenciais que preferem loops shell a `edit-loop`.

## Referência de Campos

- `EditLoopArgs { path: PathBuf }` — nova struct em `src/cli_args.rs`
- `PruneBackupsArgs { paths: Vec<PathBuf>, max_age: Option<u32>, max_count: Option<u8>, dry_run: bool }` — nova struct em `src/cli_args.rs`
- `EditLoopSummary` — nova struct em `src/ndjson_types.rs`
- `PruneBackupSummary` — nova struct em `src/ndjson_types.rs`

## Verificação

Após atualizar, rode o smoke test em um diretório de scratch para confirmar que os novos sub-comandos funcionam como esperado:

```bash
# Setup — criar um arquivo alvo
echo "hello world" > /tmp/v0122-test.txt

# v0.1.22 — edit-loop aplica 2 pares em 1 invocação
printf '%s\n' '{"old":"hello","new":"Olá"}' '{"old":"world","new":"Rust"}' \
  | atomwrite --workspace /tmp edit-loop /tmp/v0122-test.txt

# Esperado "Olá Rust" no arquivo
cat /tmp/v0122-test.txt

# v0.1.22 — prune-backups com default dry-run
echo "original" > /tmp/v0122-prune.txt
echo "novo" | atomwrite --workspace /tmp write --backup --keep-backup /tmp/v0122-prune.txt
atomwrite --workspace /tmp prune-backups --max-age 0 /tmp
# Esperado: summary mostra dry_run=true, action="would_delete", sem deleção real

# v0.1.22 — prune-backups com --dry-run false para deletar de fato
atomwrite --workspace /tmp prune-backups --max-age 0 --dry-run false /tmp
fd '*.bak.*' /tmp | wc -l
# Esperado: 0
```

## Veja Também

- `docs/decisions/0039-edit-loop-helper.md` — rationale ADR completo, alternativas consideradas e trigger para revisitar
- `docs/decisions/0040-prune-backups-subcommand.md` — rationale ADR completo, alternativas consideradas e trigger para revisitar
- `CHANGELOG.pt-BR.md` — notas da release v0.1.22
- `skill/atomwrite-en/SKILL.md` e `skill/atomwrite-pt/SKILL.md` — seções Padrão Correto para edits sequenciais com edit-loop e limpeza de backups
- `docs/HOW_TO_USE.md` — exemplos de uso para ambos os sub-comandos
