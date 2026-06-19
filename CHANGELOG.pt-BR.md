[Read in English](CHANGELOG.md)


# Changelog

- Todas as mudanças notáveis deste projeto são documentadas neste arquivo
- O formato segue [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
- O versionamento segue [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)


## [Unreleased]

### Corrigido (Falhas de CI - GAP 23 barra invertida Windows em manifestos JSON)
- **11 testes do `cli_batch` deixam de falhar no `windows-2025-vs2026`** — Os testes construíam o manifesto NDJSON via `format!` + `Path::display()`. No Windows o path nativo da plataforma usa barras invertidas (`C:\Users\...\Temp\.tmpXXXX\file.txt`), e o `format!` não as escapa. O resultado é uma string JSON com sequências de escape inválidas (`\U`, `\r`, `\A`, `\L`, `\T`), que o `serde_path_to_error::deserialize` rejeita com `invalid escape`. O `cmd_batch` então retorna `bail!` e sai com código não-zero, falhando o `assert!(output.status.success())`. O teste passava no Linux/macOS apenas porque os paths usam barras normais, que são válidas em strings JSON sem escape.
  - Adicionado helper `common::manifest(&[serde_json::Value]) -> String` em `tests/common/mod.rs` que serializa cada op via `serde_json::to_string`, garantindo escape JSON correto de barras invertidas, aspas, caracteres de controle e Unicode.
  - Refatorados todos os 11 testes afetados em `tests/cli_batch.rs` para usar o novo helper via macro `serde_json::json!`.
  - Refatorados 2 testes adicionais com o mesmo padrão de bug: `tests/snapshot_write.rs::batch_summary_ndjson_structure_snapshot` e `tests/ndjson_valid_test.rs::ndjson_batch_output_valid` (também adicionado o `mod common;` faltante neste último).
  - Adicionado teste de regressão `batch_write_escapes_backslash_in_target_path` que constrói uma string de path com barra invertida forçada em qualquer plataforma, para que o bug seja capturado em toda execução de CI, não apenas no Windows.
- **Total de testes: 303/303 PASSAM** (eram 302; +1 do novo teste de regressão).


## [0.1.23] - 2026-06-19

#### GAP-2026-015 — `allow_hyphen_values` ausente em 15 campos CLI de texto livre em 8 structs
- **Bug** — O Clap v4 trata qualquer token iniciando com `-` como flag CLI por padrão. 15 campos em 8 structs de argumentos (EditArgs, SearchArgs, ReplaceArgs, CalcArgs, RegexArgs, TransformArgs, ReadArgs, QueryArgs) aceitam conteúdo texto livre mas não tinham `allow_hyphen_values = true`. Passar `--old "- bullet point"` ou `search "-deprecated"` ou `calc "-5 + 3"` causava `ARGUMENT_PARSE_ERROR` (exit 2). Em pipelines de agentes LLM, isso disparava falhas em cascata: exit 2 mascarado por pipe jaq, workaround do agente via write truncante, perda catastrófica de dados.
- **Correção** — Adicionado `allow_hyphen_values = true` a todos os 15 atributos `#[arg]` afetados em `src/cli_args.rs`. Tier 1 (EditArgs): `old`, `new`, `after_match`, `before_match`, `between`. Tier 2 (posicionais): `SearchArgs.pattern`, `ReplaceArgs.pattern`/`replacement`, `CalcArgs.expression`, `RegexArgs.examples`. Tier 3 (nomeados): `TransformArgs.pattern`/`rewrite`/`inline_rules`, `ReadArgs.grep`, `QueryArgs.query`. Excluído: `CaseArgs.subvert` (incompatível com `num_args = 2..` — parser guloso consome flags seguintes).

#### ADR
- ADR-0041 — allow-hyphen-values-edit: 15 campos em 8 structs ganham `allow_hyphen_values = true` para aceitar conteúdo Markdown/YAML/diff e números negativos com hífens iniciais

#### Validação
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 12 novos testes de regressão em `tests/cli_v0123_hyphen_values.rs`
- 1 novo ADR: 0041 (allow-hyphen-values-edit)
- 1 fechamento GAP-2026 (015)

### GAP-2026-016 — backup-by-default para comandos que mutam conteúdo

- Alterado default de `backup` de `false` para `true` em 9 structs que mutam conteúdo: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`
- Alterado `AtomicWriteOptions::default().backup` de `false` para `true`
- Adicionada flag `--no-backup` a todas as 9 structs para opt-out explícito
- Adicionada variável de ambiente `ATOMWRITE_BACKUP=0` para opt-out global
- Adicionado helper `resolve_backup()` em `src/commands/mod.rs`
- Default existente `keep_backup: false` inalterado — backup auto-deletado após sucesso
- 4 structs não-conteúdo inalteradas: `DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`
- ADR: `docs/decisions/0042-backup-by-default.md`
- 7 novos testes de regressão em `tests/cli_v0123_backup_default.rs`

### GAP-2026-017 — guarda de shrink com --expect-checksum

- Adicionada flag `--allow-shrink` em `WriteArgs`
- Writes que reduzem arquivo em >50% agora são BLOQUEADOS quando `--expect-checksum` está ativo
- Retorna exit 65 (INVALID_INPUT) com sugestão de passar `--allow-shrink`
- Tornado `risk_assessment` (L1) bloqueante quando `--expect-checksum` está ativo e arquivo encolhe
- Sem `--expect-checksum`, comportamento inalterado (compatível retroativamente)
- ADR: `docs/decisions/0043-shrink-guard.md`
- 4 novos testes de regressão em `tests/cli_v0123_shrink_guard.rs`

### GAP-2026-018 — --old-file/--new-file para o comando edit

- Adicionadas flags `--old-file <PATH>` e `--new-file <PATH>` em `EditArgs` como alternativas a `--old`/`--new`
- Conteúdo lido de arquivos dentro do processo atomwrite, contornando shell expansion e limite ARG_MAX do kernel (~131 KB)
- `conflicts_with` impede mistura de `--old` com `--old-file` (exit 2 em conflito)
- Caminhos validados contra jail do workspace (`validate_path`)
- Adicionado campo `source` em `PairResult` ("arg" ou "file") para rastreabilidade da origem do conteúdo
- Trailing newline stripping: `strip_file_trailing_newline()` remove exatamente uma quebra de linha final (`\n` ou `\r\n`) do conteúdo de arquivo para paridade com comportamento argv (arquivos criados por `echo` têm trailing newline que valores argv não têm)
- Validação de cross-mixing: guarda runtime rejeita `--old` + `--new-file` e `--old-file` + `--new` com exit 65 (`INVALID_INPUT`) e mensagem "cannot mix --old with --new-file or --old-file with --new"
- ADR: `docs/decisions/0044-edit-old-file-new-file.md`
- 8 novos testes de regressão em `tests/cli_v0123_old_file.rs`

### Validação
- `cargo test` — todos os testes passam (609 total, 31 novos para v0.1.23)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diffs
- 4 novos ADRs: 0041 (allow-hyphen-values), 0042 (backup-by-default), 0043 (shrink-guard), 0044 (edit-old-file-new-file)
- 4 GAP-2026 fechados (015, 016, 017, 018)


## [0.1.22] - 2026-06-17

### Adicionado

- Sub-comando `prune-backups` para limpeza manual de backups legados (flags `--max-age`, `--max-count`, `--dry-run`)
- Sub-comando `edit-loop` para N edições em 1 invocação via NDJSON no stdin
- ADR-0039 (`docs/decisions/0039-edit-loop-helper.md`)
- ADR-0040 (`docs/decisions/0040-prune-backups-subcommand.md`)
- 2 schemas NDJSON (`prune-backups-output.schema.json`, `edit-loop-output.schema.json`)

### Testes

- 16 novos testes de regressão (3+4+3+3+2+2)
- 2 novos property tests sob feature `slow-tests`
- Cobertura ≥ 80% em código novo

### Documentação

- Marcadores `[FECHADO v0.1.21]` em `gaps.md` para 3 gaps
- Seção "Padrão Correto — Edits Sequenciais com Re-captura de Checksum" em SKILLs EN/PT
- Exemplo copy-paste de loop `while` em `docs/HOW_TO_USE.md`
- Seções v0.1.21 em `docs/AGENTS.pt-BR.md`


## [0.1.21] - 2026-06-17

#### GAP-2026-012 — `--allow-sequential-drift` para pipelines sequenciais de `edit`
- **Contexto** — Agentes que encadeiam múltiplas chamadas `edit` no mesmo arquivo sem re-capturar `checksum_after` entre invocações recebem `STATE_DRIFT` (exit 82) em toda chamada após a primeira. A documentação cobria o cenário paralelo mas não o sequencial (agente único, arquivo único, edições em lock-step).
- **Correção — nova flag opt-in `--allow-sequential-drift` em `edit`** — quando setada, `cmd_edit` emite `tracing::warn!` nomeando o drift e prossegue com a edição (exit 0 em sucesso). O comportamento default permanece inalterado: `STATE_DRIFT` (exit 82) ainda dispara em mismatch de checksum quando a flag está ausente. Dois padrões válidos para pipelines sequenciais: (a) re-capturar `checksum_after` após cada `edit` e passar para a próxima chamada; (b) passar `--allow-sequential-drift` uma vez em cada chamada e deixar o pré-estado de cada chamada diferir do original. Veja `SKILL.md` para a receita de loop `while` e `docs/HOW_TO_USE.md` para o exemplo copy-paste.

#### GAP-2026-013 Problema C — `--backup` e `--keep-backup` expostos em `edit`, `rollback`, `replace`, `apply`, `batch`
- **Bug (violação de paridade de API)** — `edit` e `rollback` hardcodavam `backup: false` em `AtomicWriteOptions` enquanto `write` e `replace` expunham `--backup`. Usuários que tentavam `--backup` em `edit`/`rollback` eram silenciosamente ignorados. As structs `ReplaceArgs`, `ApplyArgs` e `BatchArgs` tinham o mesmo buraco.
- **Correção — `--backup`, `--retention` e `--keep-backup` propagados em 6 subcomandos** — `edit` ganha `backup`, `retention`, `keep_backup`; `rollback` ganha `backup`, `keep_backup`; `replace`, `apply`, `batch` ganham `keep_backup`. Os 3 sites hardcoded `backup: false` em `src/commands/edit.rs:139`, `src/commands/edit.rs:393` e `src/commands/rollback.rs:108` são substituídos por `args.backup`. Paridade de subcomando para `--backup` agora é 4/4 (write, edit, replace, rollback); 6/6 subcomandos honram `--keep-backup` (write, edit, replace, rollback, apply, batch).

#### GAP-2026-014 v2 — backups são deletados após escritas bem-sucedidas por default
- **Contexto** — `cleanup_old_backups_in` podava por contagem, deixando backups vivos indefinidamente até que 5 mais novos tomassem seu lugar. Toda operação bem-sucedida com `--backup` deixava lixo persistente em disco; scripts de CI que rodavam `fd '*.bak.*' . | wc -l` viam contagens crescentes proporcionais ao volume de escrita.
- **Correção — `keep_backup: bool` em `AtomicWriteOptions`, default `false`** — novo helper `delete_backup_quietly(path)` remove o backup após `atomic_write_inner` retornar sucesso. `ErrorKind::NotFound` é mapeado para `Ok(())` (idempotência). Em erros não-NotFound, `tracing::warn!` é emitido e a operação prossegue (cleanup é logado, não propagado). Em caminhos de falha o backup é preservado como antes. `keep_backup: true` é o opt-in explícito para preservar o backup; o comportamento prévio de `--backup` de deixar backups em disco agora só é acessível via `--keep-backup`. 6 subcomandos aceitam a flag: `write`, `edit`, `replace`, `rollback`, `apply`, `batch`. Veja `docs/decisions/0038-backup-cumprido-deleta.md`.

#### Paridade — `apply` e `batch` agora honram `--keep-backup`
- `apply` propaga `args.keep_backup` para a chamada interna de `atomic_write` para que um patch bem-sucedido não deixe um `.bak` sibling para trás por default.
- `batch` propaga `--keep-backup` para toda op `write`/`edit`/`replace` no manifesto NDJSON. `keep_backup` por op no NDJSON sobrescreve o default em nível de batch.

#### ADR
- ADR-0038 — backup cumprido deleta: justificativa para `keep_backup` default `false` + helper `delete_backup_quietly`; alternativas rejeitadas são scheduler, subcomando `prune-backups` e cleanup por idade (todas subsumidas por deleção-após-sucesso).

#### Migration Notes
- **Breaking change** — `write --backup` e `replace --backup` não deixam mais um sibling `.bak` em disco após uma escrita bem-sucedida. O comportamento pré-v0.1.21 de backup vive para sempre acabou. Adicione `--keep-backup` a qualquer script que dependa do backup persistindo através da operação, ou reescreva para ler o backup antes da escrita completar.
- **Breaking change** — `edit` e `rollback` agora aceitam `--backup` mas o ignoram sem reclamação se as pré-condições da camada atômica rejeitarem. O novo opt-in é a flag explícita `--backup`; scripts antigos que chamavam `edit` com a suposição de sem backup ainda recebem sem backup por default.
- **Não-breaking** — `apply --keep-backup` e `batch --keep-backup` são aditivos. Comportamento default (sem backup) permanece inalterado.

#### Validation
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 555+ testes passando (542 baseline v0.1.20 + 13 novos: 6 em `cli_v0121_backup_keep_flag`, 2 em `cli_v0121_edit_backup`, 3 em `cli_v0121_sequential_drift`, 1 em `cli_v0121_rollback_backup`, 1 em `cli_v0121_apply_keep`, 1 em `cli_v0121_batch_keep`, 1 em `proptest_v0121_backup_delete`)
- 1 novo ADR: 0038 (backup cumprido deleta)
- 3 novos GAP-2026 fechados (012, 013 Problema C, 014 v2)
- Cross-compile verificado em 3 targets Windows: x86_64-gnu, i686-gnu, x86_64-msvc
- Smoke test de migração: `fd '*.bak.*' . | wc -l` reporta 0 em uma execução pós-sucesso; reporta 1 quando `--keep-backup` está setado


## [0.1.20] - 2026-06-15

> **NOTA — Renomeação de flag (ADR-0037)** — a flag global `--lang` foi renomeada para `--locale`. **A env var `ATOMWRITE_LANG` permanece inalterada** (a renomeação foi apenas no flag CLI long-form; o env var e o campo programático `args.global.lang` seguem estáveis). Os valores aceitos permanecem `en` e `pt-BR`. O alias `--lang` foi REMOVIDO desta flag global e LIBERADO para subcomandos — agora passe `--lang` para subcomandos que o aceitam como alias (ex.: `atomwrite scope --lang rust`). Veja `docs/decisions/0037-global-locale-rename.md` para a justificativa completa.

#### GAP-2026-001 — `count --by-size` finalmente implementa a flag de help
- **Bug** — `cmd_count` ignorava `args.by_size` apesar do campo existir em `src/cli_args.rs`. A help prometia top-N por tamanho; o código sempre retornava `mode: "lines"`. HELP-FIRST-DRIFT (flag anunciada na help antes da implementação existir).
- **Correção** — Nova struct `CountBySizeOutput` em `ndjson_types.rs`. `cmd_count` coleta `Vec<(PathBuf, u64)>` durante o walk, ordena descendente por tamanho, trunca para `args.top` (default 10) e emite o output do top-N.

#### GAP-2026-002 — `write --preserve-timestamps` (paridade com edit/replace)
- **Bug** — `cmd_write` hardcodava `preserve_timestamps: false` em `AtomicWriteOptions` enquanto 5 de 6 subcomandos mutantes expunham a flag.
- **Correção** — Adicionado `preserve_timestamps: bool` a `WriteArgs`; `cmd_write` agora passa `args.preserve_timestamps` para a camada atômica.

#### GAP-2026-003 — alias `scope --lang`
- **Bug** — `ScopeArgs.language` foi declarado com `short='l' long="language"` apenas, sem `alias = "lang"`. Usuários seguindo os docs da SKILL que mencionam `--lang` recebiam `ARGUMENT_PARSE_ERROR`.
- **Correção** — Adicionado `alias = "lang"` ao campo `language`.

#### GAP-2026-004 — `write --line-ending crlf` (aceita ambas as formas)
- **Bug** — `LineEnding::CrLf` era renderizado pelo clap como `cr-lf` (kebab-case) enquanto os docs e expectativas prévias de usuários escreviam `crlf` (sem hífen).
- **Correção** — Adicionados `#[value(name = "cr-lf", alias = "crlf")]` e atributos `value` equivalentes a todas as 4 variantes para superfície CLI explícita e estável.

#### GAP-2026-005b — semântica de `edit --partial` documentada
- **Apenas doc** — `--partial` é implementado para multi-par; o caminho de falha single-par agora retorna `NoMatches` (exit 1) sem escrita. Documentado em skill EN/PT.

#### GAP-2026-006 — testes de regressão para `diff --algorithm`
- **Doc + testes** — `algorithm: DiffAlgorithm` já estava implementado e despachado para `similar::Algorithm::{Myers,Patience,Lcs}`. Adicionados testes de regressão para `myers` vs `patience` vs `lcs`.

#### GAP-2026-007 — `count --by-extension` filtra timestamps de backup
- **Bug** — `validated.extension()` retornava o último componente `.`, então `foo.txt.bak.20260615_035515` era categorizado como `20260615_035515`.
- **Correção** — Nova regex `BACKUP_RE` `\.bak\.\d{8}_\d{6}$` casa nomes de arquivo de backup e os roteia para uma categoria `"backup"` dedicada.

#### GAP-2026-008 — `read --head/--line/--lines` reporta contagem de linhas filtrada
- **Bug** — `ReadOutput.lines` reportava o total do arquivo não-filtrado mesmo com `--head`, `--line` ou `--lines`.
- **Correção** — `line_count` agora é computado a partir do `content_str` filtrado; `lines_total` (novo campo opcional) preserva o total original do arquivo para consumidores downstream.

#### GAP-2026-009 — `read` emite discriminador `mode`
- **API** — `ReadOutput` ganha um campo `mode`: `"full" | "head" | "tail" | "line" | "lines" | "grep" | "stat"`. Consumidores downstream não precisam mais parsear o conteúdo para discriminar leituras parciais de leituras completas.

#### GAP-2026-010 — `search --no-begin-end` para output de walk vazio mais limpo
- **API** — Nova flag `no_begin_end: bool` em `SearchArgs`. Default (off) preserva o comportamento pré-v0.1.20. Com a flag, eventos NDJSON `begin`/`end` são suprimidos para arquivos com zero matches.

#### GAP-2026-011 — guardas de intenção em `write` (defesa em profundidade após incidente de 2026-06-15)
- **Incidente** — Durante uma auditoria em 2026-06-15, `atomwrite write` foi chamado sem `--append` sobre `c24-framework34.html` (491827 bytes). O arquivo foi truncado para poucos bytes; ~127 linhas de trabalho de 15-jun (~9 KB) foram perdidas.
- **Correção — 5 novas flags em `WriteArgs`** (todas opt-in, default off para preservar compat):
  - `--require-backup` (L2): aborta com `InvalidInput` (exit 65) se o alvo existe e `--backup` não está setado.
  - `--confirm` (L3): quando o alvo existe e é maior que 100KB, prompt `Overwrite <path> (<N> bytes)? [y/N]` e lê do stdin. Aborta em qualquer resposta diferente de `y`/`yes`.
  - `--auto-rotate` (L5): quando `--backup` está ativo, força um backup rotativo se o alvo foi modificado nas últimas 24 horas.
  - `--risk-threshold <PERCENT>` (default 50): threshold L1 do guarda de tamanho; emite warning no stderr (`low`/`medium`/`high`) quando o delta de tamanho excede o threshold.
  - Telemetria: `WriteOutput.risk_assessment` (opcional, GAP-2026-011 L6) carrega bytes original/novo, percentual de delta, nível de risco e qual guarda disparou.

#### ADR
- ADR-0034 — help-driven testing anti-pattern: clap `--help` nunca mais declarado antes da implementação (5 dos 11 GAP-2026 tinham esse anti-pattern)
- ADR-0035 — write intention guards: 4 flags defense-in-depth (--require-backup, --confirm, --auto-rotate, --risk-threshold) + `risk_assessment` no envelope, motivadas pelo incident c24-framework34.html de 2026-06-15
- ADR-0036 — `edit --partial`: single-pair com zero matches retorna NO_MATCHES (exit 1); multi-pair aplica matched e relata unmatched em `pair_results`
- ADR-0037 — rename `--lang` global para `--locale` (env var `ATOMWRITE_LANG` permanece, campo `args.global.lang` permanece, namespace `--lang` liberado para subcomandos como alias de `--language`)

#### Migration Notes
- **Breaking change na CLI surface**: scripts que passavam `--lang <locale>` devem migrar para `--locale <locale>` (one-liner: `rg -l '\-\-lang\b' bin/ scripts/ && sd -- '\-\-lang\b' '--locale' bin/ scripts/`)
- Env var `ATOMWRITE_LANG` e campo programático `args.global.lang` permanecem estáveis — CI matrices, container wrappers e consumidores Rust não precisam de mudança
- Subcomandos que tinham `--language` agora também aceitam `--lang` como alias (ex.: `atomwrite scope --lang rust`)

#### Validation
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 542 testes passando em 47 suites (up from 515 in 46 suites in v0.1.19, +27 new: 11 GAP-2026 closures + 16 intention-guard tests)
- 4 novos ADRs: 0034 (help-driven testing), 0035 (write intention guards), 0036 (edit partial), 0037 (locale rename)
- 11 GAP-2026 fechados (001-011), 100% cobertura dos gaps de auditoria local
- Cross-compile verificado em 3 targets Windows: x86_64-gnu, i686-gnu, x86_64-msvc


## [0.1.19] - 2026-06-14

#### G121 — `search` e `replace` resolvem caminhos raiz contra o workspace
- **Bug (CWE-367 — TOCTOU/confusão de caminho)** — `cmd_search` e `cmd_replace` recebiam os caminhos raiz do chamador, validavam-nos com `path_safety::validate_path` e em seguida alimentavam o `ignore::WalkBuilder` com o caminho ORIGINAL (relativo ao CWD). `validate_path` retornava o caminho absoluto canônico, mas o resultado da chamada por entrada era descartado. Quando `CWD != --workspace`, o walker ou (a) caminhava silenciosamente pela árvore errada se existisse um caminho com mesmo nome sob o CWD, ou (b) emitia um evento `JailViolation` por arquivo caminhado. A correção G118 em `write.rs:44` (ADR-0027) nunca foi propagada para estes dois comandos porque a `validate_path` por entrada dentro da thread de trabalho mascarava a falta da resolução pré-passo.
- **Correção — novo helper `path_resolution::resolve_paths_against_workspace`** — `cmd_search` e `cmd_replace` agora chamam este helper uma vez no início do comando (após `global.resolve_workspace()()`) e passam o `Vec<PathBuf>` canônico para `build_walker`. O loop pré-passo `for path in &args.paths { validate_path(path, &workspace)?; }` em `replace.rs` é removido; o `search` ganha uma chamada de resolução paralela. Ambas as assinaturas de `build_walker` ganham o parâmetro `canonical_paths: &[PathBuf]`; elas não leem mais `&args.paths[0]` diretamente.
- **Consequência** — Search e replace agora honram `CWD != --workspace`. Um caminho relativo como `src/` passado com `--workspace /path/to/ws` caminha por `/path/to/ws/src/` independentemente do CWD do processo. Caminhos fora do jail falham uma única vez com `WORKSPACE_JAIL` (exit 126) no início do comando em vez de por arquivo dentro do worker. Veja `docs/decisions/0031-g121-path-resolution-helper.md`.

#### G122 — Matching real de S-expression no subcomando `query`
- **Bug (feature silenciosa, documentada mas nunca implementada)** — O subcomando `query` (v14 Tier 3, introduzido na v0.1.12) sempre prometeu suporte a S-expression. Os documentos em `COOKBOOK.md`, `HOW_TO_USE.md` e nos dois SKILLs bilíngues mostram exemplos como `atomwrite --workspace . query src/main.rs --query "(function_item name: (identifier) @name)"`. Na prática, `cmd_query` chamava `walk_kind_filter` que faz `wanted.iter().any(|w| w == &kind)` — comparação literal de STRING com `node.kind()`. A string inteira `"(function_item name: (identifier) @name)"` nunca casou com nenhum `node.kind()` real, então a feature S-expression nunca funcionou.
- **Correção — novo enum `QueryType` + auto-classificação** — `classify_pattern(pattern) -> QueryType` detecta S-expression pela presença de `(`, `)`, ou `@`. `walk_sexpr` compila o pattern via `tree_sitter::Query::new`, executa via `QueryCursor::matches`, e emite NDJSON `query_match` com o novo campo `capture_name` para cada `@capture`. `cmd_query` ramifica na classificação.
- **Nova dependência direta `tree-sitter = "0.26"`** — a language-pack reexporta `Language`, mas `Query`/`QueryCursor`/`StreamingIterator` exigem o crate `tree-sitter` em si.
- **Consequência** — Patterns com `(`, `)`, ou `@` são roteados para `tree_sitter::Query::new`. Erros de parse (ex.: `(unclosed`) retornam exit 1 com mensagem `invalid S-expression pattern: ...` (via `anyhow::Context`). O caminho kind-filter é preservado bit-a-bit: usuários que passavam `--query function_item` (sem caracteres de S-expression) continuam recebendo os mesmos resultados. Veja `docs/decisions/0032-query-sexp-real-implementation.md`.

#### Consolidação de drift na documentação de exit codes (ADR-0033)
- **Contexto** — Testes de Fase D em 2026-06-14 rodaram 7 probes binários concretos contra a release v0.1.18 e expuseram 7 pontos onde a documentação publicada divergia do comportamento real do binário. Os 7 drifts são:
  1. `STATE_DRIFT` (82) absorve `CHECKSUM_VERIFY_FAILED` (81) para `--verify-checksum` — ambos são classe conflict, retentáveis. O code 81 é agora histórico (preservado apenas para o mismatch BLAKE3 do caminho `read` no conteúdo do arquivo).
  2. `--syntax-check` retorna `SYNTAX_ERROR_DETECTED`, NÃO `SYNTAX_ERROR` — o rename aconteceu no rollout do G72 tree-sitter da v0.1.12 mas a documentação não foi atualizada.
  3. `ORPHAN_JOURNAL` (93) é consultivo, NÃO autodetectado — o portão é `ATOMWRITE_WAL=1` OU `--strict-atomic`. O `write` padrão (v0.1.16 G119 `WalPolicy::Auto`) não escreve sidecar e portanto não pode detectar órfãos.
  4. `BROKEN_PIPE` (141) exige propagação real de SIGPIPE — um pipe simples `head -1` NÃO o dispara. A restauração de SIGPIPE da v0.1.4+ recoloca a disposição default, então o sinal só é levantado quando o consumidor downstream fecha ativamente o pipe no meio do stream.
  5. Leituras de arquivo binário retornam exit 0 com metadados `kind: binary`, NÃO exit 65 — a heurística `BINARY_FILE` da v0.1.4 foi alterada para emitir envelope estruturado e exit 0. O caminho do code 65 agora só dispara para `read` sem `--format raw` E com a heurística binária bypassada.
  6. Argumento posicional ausente retorna `ARGUMENT_PARSE_ERROR` (exit 2), NÃO `INVALID_INPUT` (65) — erros de argumento no nível clap são reportados como exit 2. O code 65 é reservado para validação de conteúdo em runtime (TOML malformado, regex inválida, stdin vazio padrão).
  7. Falta de `--workspace` cai para CWD, NÃO é erro — `--workspace` é documentado como flag com default CWD, não argumento obrigatório. `WORKSPACE_JAIL` (126) só dispara quando um caminho absoluto resolve fora do jail efetivo.
- **Decisão** — Aceitar o comportamento do binário como canônico. Consolidar a documentação na v0.1.19 para casar. Veja `docs/decisions/0033-v0-1-19-exit-code-naming-drift-consolidation.md`.
- **Nota sobre o nome legado `SYNTAX_ERROR`** — a documentação da v0.1.12 usava `SYNTAX_ERROR`; o binário na v0.1.18 emite `SYNTAX_ERROR_DETECTED`. O nome histórico é preservado apenas em prosa para grep-ability.

#### Validação
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 515 testes passando em 46 suítes (acima de 502 em 44 suítes na v0.1.18, +13 novos)
- 3 novos ADRs: 0031 (helper de resolução de caminho), 0032 (S-expression de query), 0033 (consolidação de drift de exit code)

## [0.1.18] - 2026-06-14

#### G118 — `replace` pré-valida caminhos raiz contra o jail do workspace
- **`cmd_replace` resolve-first para todas as raízes** — após `global.resolve_workspace()`, o comando itera sobre `args.paths` e chama `path_safety::validate_path(path, &workspace)?` para CADA raiz ANTES de construir o `WalkBuilder`. Falha rápida com `WORKSPACE_JAIL` (exit 126) na primeira violação. Comportamento legado per-entry (v0.1.12-v0.1.17) emitia um evento `JailViolation` por arquivo caminhado e o usuário via o diagnóstico enterrado sob N eventos. Agora `replace /etc/passwd` aborta em microssegundos com um único envelope de erro estruturado.
- **Convenção resolve-first agora universal** — `write` (ADR-0027), `edit`, `copy`, `apply`, `move`, `rollback`, `set`, `del`, `case` e agora `replace` todos validam o alvo contra o jail workspace ANTES de qualquer `exists()` ou read. Um único modelo mental para todos os comandos mutantes.
- **Teste de regressão atualizado** — `replace_jail_violation_does_not_inflate_counter` em `tests/cli_v012_regressions.rs` agora afirma exit 126 + envelope `WORKSPACE_JAIL` + arquivos inside/outside inalterados. O nome é preservado (a invariante subjacente — "jail violation não pode inflar o counter de substituições" — é mantida) mas o corpo da asserção mudou.
- **2 novos testes integrados** em `tests/cli_replace.rs`: `replace_root_path_outside_workspace_exits_126` (caminho absoluto `/etc/passwd`) e `replace_relative_dotdot_root_outside_workspace_exits_126` (caminho relativo `../escape`).

#### G120 L3 — cobertura de teste para cross-validação
- **2 novos testes integrados** em `tests/cli_write.rs`:
  - `g120_l3_append_empty_stdin_with_matching_checksum_succeeds` — escreve arquivo seed, hasheia, roda `write --append --allow-empty-stdin --expect-checksum <HASH> < /dev/null`, afirma exit 0 + `stdin_bytes_read: 0` + arquivo inalterado (no-op append preserva checksum).
  - `g120_l3_append_empty_stdin_without_opt_in_rejects_at_l1` — sem `--allow-empty-stdin`, a guarda L1 dispara primeiro (exit 65) e o arquivo é preservado. Documenta que L3 é inalcançável sem opt-in explícita.

#### G117 follow-up — cobertura de edge cases
- **3 novos testes integrados** em `tests/cli_edit.rs`:
  - `edit_unicode_old_new_exact_match` — diacríticos UTF-8 (`ção` → `AÇÃO`) com casamento exato byte-a-byte. Documenta o contrato de single-pair (substitui apenas a PRIMEIRA ocorrência; multi-pair para múltiplas).
  - `edit_crlf_line_endings_preserve_eol_after_replace` — input com `\r\n`, replace, afirma preservação byte-a-byte (sem colapso para `\n`).
  - `edit_multi_pair_same_old_appears_twice_applies_both` — multi-par onde ambos os pares referenciam o mesmo token `--old`. Garante que o segundo par consome a versão pós-primeiro-substituição (proteção contra off-by-one).

#### ADR
- **`docs/decisions/0030-v0-1-18-g118-replace-pre-validation-g120-l3-tests-g117-edge-cases.md`** — registra as 3 decisões, alternativas consideradas e gatilhos para revisitar.

#### Validation
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- Suíte completa de testes: 502 testes passando, 0 falhas, 0 regressões introduzidas pelas 3 mudanças
- 2 flakes pré-existentes confirmados como não relacionados (`signal_test::batch_interrupted_by_signal`, `tracing_test::span_captures_path_field` + `tracing_test::debug_level_includes_filter_info`) — falhas idênticas no baseline `git stash` antes das mudanças


## [0.1.17] - 2026-06-13

#### G119 — fiação de L3 startup auto-heal e L4 no Drop guard
- **L3 startup pass** — cada invocação agora chama `auto_heal_on_startup(&workspace, threshold_secs=3600, max_duration_ms=100)` uma vez, ANTES de despachar o subcomando. Threshold default é 1h, orçamento é 100ms, e órfãos `Started` NUNCA são reaped (apenas `Committed`/`Aborted` mais antigos que o threshold). Em um workspace com 60 sidecars stale, o pass reapa todos em <5ms; em um workspace com 10k, o orçamento ainda limita o custo.
- **Nova flag global `--no-auto-heal`** — desabilita o pass L3 para loops apertados de CI, benchmarks e forense. Também vinculado à env `ATOMWRITE_WAL_NO_AUTO_HEAL=1`. Existe para manter o overhead por invocação previsível quando o workspace tem 0 sidecars (evita o custo de walk+parse em cada comando).
- **L4 fiação em `JournalGuard::drop`** — o drop agora consulta `heuristics_should_preserve` antes de remover. `h1_ttl`, `h3_rate_limit`, `h4_sentinel`, `h5_archive` todos votam; `h2_lru_within_cap` é intencionalmente bypassado (não há forma barata de saber a contagem global a partir de um drop por arquivo) passando `u64::MAX` tanto para `workspace_committed_count` quanto para `age_rank`. Composição OR: qualquer voto de preservação vence.
- **Novo campo `committed_at_unix: Option<u64>` em `JournalGuard`** — `release()` carimba o timestamp Unix atual para que as heurísticas L4 que raciocinam sobre idade pós-commit (h1_ttl, h5_archive) tenham dados. O guard inerte e o caminho `keep()` o deixam `None` (nenhuma entrada Committed existia).
- **Correções de teste** — 2 testes em `tests/cli_v012_wal.rs` (`wal_heal_reaps_stale_committed_journals`, `wal_stats_counts_committed_orphans_malformed`) e 2 em `tests/cli_wal.rs` (`g119_l5_wal_stats_reports_journal_state`, `g119_l5_wal_stats_reports_zeros_on_empty_workspace`) agora passam `--no-auto-heal` para impedir que o pass L3 de startup reapa sidecars stale pré-semeados antes da asserção rodar.
- **Atualização de ADR** — `docs/decisions/0028-g119-wal-cleanup-intelligent.md` ganhou uma seção "Atualização v0.1.17 — Fiação de L3 startup + L4 no Drop guard" documentando o truque `u64::MAX`, o campo `committed_at_unix`, e a justificativa de isolamento de teste.

#### Validação
- `cargo fmt --check` limpo
- `cargo clippy --bin atomwrite --lib --all-targets -- -D warnings` limpo (0 warnings)
- 6 novos testes unitários em `src/wal.rs::tests` para L3 (`l3_auto_heal_on_empty_workspace_reports_zero`, `l3_auto_heal_reaps_old_committed_preserves_started`, `l3_auto_heal_respects_budget`) e L4 (`l4_release_records_committed_at_unix`, `l4_drop_preserves_sidecar_when_h4_sentinel_votes`, `l4_drop_removes_sidecar_when_no_heuristic_preserves`)
- 3 novos testes integrados em `tests/cli_wal.rs` (`g119_l3_startup_auto_heal_reaps_stale_committed`, `g119_l3_no_auto_heal_preserves_stale_committed`, `g119_l4_sentinel_preserves_sidecar_on_successful_write`)
- Suíte completa: 474 testes passando, 0 falhas, 0 regressões


## [0.1.16] - 2026-06-13

#### G119 — fecha a limpeza autônoma de 5 camadas (L1 prevention + L4 heuristics)
- **L1 — enum `WalPolicy` + flag `--wal-policy`** — `Auto` (default) pula o sidecar para escritas triviais (arquivo ≤ 1 MiB AND não Edit/Replace AND diretório pai sob git AND tamanho do arquivo ≤ 4 KiB). `Always` força o sidecar (semântica legada, equivalente a `--strict-atomic`). `Never` suprime criação de sidecar mesmo quando `--strict-atomic` está setado. A decisão acontece dentro de `atomic_write` ANTES de `journal_started_with_guard`; custo é O(0) quando a política vota "sem sidecar". Redução esperada: 60-80% dos sidecars para cargas típicas de LLM agent.
- **L4 — `HeuristicsEngine` com 5 regras composíveis** — `h1_ttl` (preserva por N segundos após `Committed`, default 0), `h2_lru_within_cap` (preserva dentro do cap de contagem, default 100), `h3_rate_limit` (estrangula quando >K sidecars/min, default 10), `h4_sentinel` (arquivo `.atomwrite_no_wal` desabilita por diretório), `h5_archive` (flag para arquivamento quando mais antigo que 7 dias). Env vars: `ATOMWRITE_WAL_KEEP_SECS`, `ATOMWRITE_WAL_MAX_COUNT`, `ATOMWRITE_WAL_RATE_LIMIT`, `ATOMWRITE_WAL_ARCHIVE_DAYS`. H3 usa `AtomicU64` lock-free para a janela de 60s. `heuristics_should_preserve(target, committed_at_unix, count, rank)` compõe via OR.
- **`AtomicWriteOptions.wal_policy: WalPolicy`** — novo campo defaultando para `Auto`. Todos os 16 call-sites em `src/commands/` (write, edit, set, get, del, case, copy, replace, transform, scope, apply, rollback, batch×4) atualizados para passar a política adiante.
- **Telemetria** — envelope NDJSON `WriteOutput` ganha `wal_policy: "auto" | "always" | "never"` para que chamadores possam auditar qual política foi aplicada.
- **ADR** — `docs/decisions/0028-g119-wal-cleanup-intelligent.md` documenta a arquitetura de 5 camadas e a semântica de composição OR do engine L4.
- **Novos testes unitários** — 12 testes em `src/wal.rs::tests`: `l1_never_policy_always_returns_false`, `l1_always_policy_always_returns_true`, `l1_auto_policy_returns_true_for_large_file`, `l1_auto_policy_returns_true_for_edit_op`, `l1_auto_policy_skips_trivial_file`, `l4_h1_ttl_default_zero_returns_false`, `l4_h2_lru_within_cap_returns_true_when_count_low`, `l4_h2_lru_returns_true_when_count_at_or_below_default_cap`, `l4_h3_rate_limit_returns_false_below_threshold`, `l4_h4_sentinel_returns_true_when_file_exists`, `l4_h4_sentinel_returns_false_when_absent`, `l4_h5_archive_returns_false_for_recent_journal_under_default`, `l4_h5_archive_returns_true_for_journal_older_than_7_days`, `l4_engine_returns_false_when_all_heuristics_disabled`.

#### G120 — fecha a validação de conteúdo de 4 camadas (L3 cross-validation)
- **L3 — `--append`/`--prepend` + `--expect-checksum` + stdin vazio emite warning estruturado** — quando o chamador combina flag append/prepend com `--expect-checksum` E o stdin é vazio (o caso de bypass de L1 via `--allow-empty-stdin`), `cmd_write` emite um `tracing::info!` que nomeia a combinação de flags cruzadas e prossegue para `verify_checksum` (que ainda valida o estado pré-mutação). Operadores monitorando stderr recebem um sinal explícito sem mudar o exit code.
- **L3 opt-out — `--no-checksum-when-empty`** — chamador que INTENDE a combinação empty-stdin + checksum (no-op append com garantia de locking) pode passar esta flag para pular `verify_checksum` inteiramente. Emite `tracing::warn!` registrando a decisão.
- **L1+L2 inalterados** — `read_stdin_content` ainda rejeita stdin vazio por default (exit 65); `handle_append_prepend` ainda rejeita stdin vazio quando `--append`/`--prepend` está setado. L3 é a terceira camada que roda apenas quando L1+L2 são explicitamente bypassadas.
- **ADR** — `docs/decisions/0029-g120-empty-stdin-guard.md` documenta a semântica de warning L3 e a flag de opt-out.

#### Validação
- `cargo fmt --check` limpo
- `cargo clippy --bin atomwrite --lib` limpo (0 warnings)
- Suíte completa: 487 testes passando (469 baseline v0.1.15 + 18 novos em `src/wal.rs::tests`), 0 falhas, 0 regressões
- Novo `WalPolicy` exportado de `crate::wal` e registrado no derive `ValueEnum` do clap
- Schema `WriteOutput` regenerado para incluir campo `wal_policy`; `tests/snapshots/snapshot_write__write_output_structure.snap` atualizado


## [0.1.15] - 2026-06-11

#### G117 — `edit --old/--new` multi-par: paridade fuzzy, relato por par e `--partial` opt-in
- **Paridade fuzzy** — o caminho multi-par usava apenas busca exata `find_str`, então um par com whitespace divergente que a cascata fuzzy do par único resgata derrubava o lote inteiro. A cascata foi extraída para `match_pair` e compartilhada pelos dois caminhos: cada par roda a cascata completa de 9 estratégias (`exact`, `line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `escape_normalized`, `trimmed_boundary`, `block_anchor`, `context_aware`). `--fuzzy off` preserva o comportamento exato pré-G117.
- **Relato por par** — envelopes de sucesso ganham `pairs_total` e `pair_results` (array de `{index (1-based), matched, strategy, similarity}`); `mode` vira `fuzzy-multi(N)` quando algum par casou via fuzzy e permanece `exact-multi(N)` caso contrário. Envelopes de erro ganham `failed_pair_index`, `pairs_total` e `pair_results` via a nova variante `AtomwriteError::EditPairFailed`, que reutiliza o code `INVALID_INPUT` e o exit 65 (nenhum code novo). Pares após o índice falho nunca foram tentados e ficam ausentes do array.
- **Nova flag `--partial` (opt-in)** — aplica os pares que casam em uma escrita atômica (exit 0, `edits < pairs_total`) e relata os ausentes com `matched: false`. Zero pares aplicados sai com 1 (`NO_MATCHES`) sem escrever, igual à semântica do `replace`. O padrão continua all-or-nothing: quando a atomicidade é possível, ela é estritamente melhor que relato de falha parcial.
- **Orientação anti-mascaramento** — o envelope de erro NDJSON vai para o stdout por contrato, então `edit ... | jaq '.edits'` mascarava o exit 65 como `{"edits": null}` com exit 0 no pipeline. README, SKILL, COOKBOOK e HOW_TO_USE (ambos os idiomas) agora documentam a receita `jaq -e '.edits'` / `${PIPESTATUS[0]}`.
- **Schemas regenerados** — `docs/schemas/edit-output.schema.json` foi regenerado de `edit --json-schema` (agora inclui `mtime_preserved`, `pairs_total`, `pair_results`); `docs/schemas/error-output.schema.json` ganha os três campos do G117 mais os cinco error codes introduzidos na v0.1.12 que faltavam no enum (`LOCK_TIMEOUT`, `SYNTAX_ERROR_DETECTED`, `EXDEV_FALLBACK_DISABLED`, `COPY_BACK_BLAKE3_FAILED`, `ORPHAN_JOURNAL`).
- **Nota de escopo** — o modo `--multi` (NDJSON via stdin) permanece inalterado; o G117 cobre apenas pares repetidos `--old`/`--new`. Veja `docs/decisions/0026-g117-edit-multi-pair-fuzzy-partial.md`.

#### G118 — `write` resolve o alvo contra o workspace antes de todos os pré-passos
- **Bug (dupla identidade de caminho, CWE-367)** — `cmd_write` entregava o caminho CRU da CLI (relativo ao CWD) a `handle_append_prepend`, `normalize_line_endings` (auto) e `verify_checksum`, enquanto só `atomic_write` resolvia via `validate_path`. Com alvo relativo e CWD diferente do `--workspace`, append/prepend TRUNCAVA silenciosamente o arquivo, a detecção automática de line ending era pulada e o `--expect-checksum` era pulado por inteiro (qualquer hash aceito, exit 0). Detectado em produção: o `gaps.md` deste repositório foi truncado e recuperado via `rollback --latest --verify`.
- **Correção** — o alvo é resolvido UMA vez no início de `cmd_write` e o caminho resolvido alimenta os 3 pré-passos e o `atomic_write`. Drift de checksum com CWD divergente agora falha com `STATE_DRIFT` (exit 82); alvo fora do jail falha cedo com `WORKSPACE_JAIL` (exit 126). O campo `path` do NDJSON continua ecoando o caminho do usuário. Ver `docs/decisions/0027-g118-write-path-resolution.md`.
- **Por que os testes nunca pegaram** — a suíte só usava alvos ABSOLUTOS, imunes ao CWD. Cinco testes de regressão agora usam alvo RELATIVO com `current_dir` fora do workspace (append, prepend, drift exit 82, checksum correto, detecção CRLF), mais um teste-guarda de conformidade garantindo que `&args.target` aparece exatamente uma vez em `write.rs`.

#### GAP 18 — CI Windows verde de novo
- `tests/snapshot_write.rs` agora redige `platform.dir_fsync` como `[platform_dir_fsync]`, a mesma técnica já usada para `platform.fsync`. O snapshot fixava `"dir_fsync": "sync_all"`, que o Windows reporta como `best_effort`, mantendo o job `windows-2025-vs2026` vermelho desde a v0.1.12.

#### Job de MSRV alinhado ao manifesto
- O job de CI chamado `MSRV 1.85` (toolchain pinado em 1.85) agora testa o MSRV documentado: `MSRV 1.88` com `dtolnay/rust-toolchain@1.88`, casando com `rust-version = "1.88"` do `Cargo.toml`.

#### Validação
- 8 novos testes de integração em `tests/cli_edit.rs` (21 no total na suíte): par fuzzy no multi, compat do mode `exact-multi(N)`, `pair_results` no sucesso, `failed_pair_index`/`pairs_total`/`pair_results` no erro com arquivo intacto, compat de `--fuzzy off`, caminho feliz do `--partial`, `--partial` com zero matches sai com 1, `--partial --dry-run` não escreve
- `cargo test --lib` 152 passando (cobertura de suggestion por variante estendida a `EditPairFailed`); `cargo test --test snapshot_write` 9 passando
- Reprodução determinística do `gaps.md` re-executada contra o binário novo: lote misto reporta `failed_pair_index: 2` com arquivo intacto; `--partial` aplica o par 1 com `edits: 1`; `| jaq -e '.edits'` sai com 1 no envelope de erro
- G118: 6 testes de integração novos em `tests/cli_write.rs` (14 no total); reprodução determinística re-executada com CWD divergente: append preserva todas as linhas, o checksum de 64 zeros agora sai com 82 e o arquivo permanece intacto
- Suíte completa após o G118: 461 testes passando (445 na v0.1.12 + 2 na v0.1.14 + 8 G117 + 6 G118), 0 falhas; `fmt`/`clippy -D warnings`/`doc`/`deny`/`audit` verdes; cross-check Windows `x86_64-pc-windows-gnu` com `RUSTFLAGS=-Dwarnings` limpo


## [0.1.14] - 2026-06-07

### Corrigido (Falha de CI - `windows-2025-vs2026` no teste `write_creates_file_with_ndjson_output`)
- **Causa raiz** — O teste escreve 12 bytes de input (`"hello world\n"`) e espera `bytes_written == 12`. No Windows o v0.1.13 retornava 13 bytes porque o branch de fallback em `normalize_line_endings` retornava `LineEnding::CrLf` quando o arquivo alvo não existia e o SO host era Windows. A chamada subsequente `line_endings::normalize(..., CrLf)` inseria um `\r` antes de cada `\n`. Linux e macOS não eram afetados (o fallback delas retornava `LineEnding::Lf`, que preservava a contagem de bytes do input).
- **Solução** — Os branches de fallback que retornavam `LineEnding::CrLf` no Windows (quando o arquivo alvo não existia ou não podia ser lido) foram removidos. `Auto` agora cumpre a sua docstring (`Preserve the dominant ending of the original file`): quando não há original, os bytes do input passam adiante sem modificação. Isso torna o CLI determinístico entre Linux, macOS e Windows para o mesmo conteúdo de stdin.
- **Paridade de round-trip para input CRLF** — Um bug adicional foi descoberto ao escrever os testes de regressão: o fallback antigo não apenas convertia LF para CRLF, ele também *removia* o `\r` de input CRLF ao resolver `Auto` → `Lf` e depois executar a etapa de canonicalização de `normalize`. Com o novo comportamento, input CRLF em arquivo novo agora permanece CRLF byte a byte. Isso preserva o round-trip de `--expect-checksum` quando o usuário fornece conteúdo CRLF.
- **Dois novos testes de regressão em `src/commands/write.rs::tests`** — `auto_on_new_file_preserves_lf_input` e `auto_on_new_file_preserves_crlf_input`. Eles exercitam o branch `Auto` com input LF e CRLF e afirmam que a saída é igual ao input byte a byte. Os testes são agnósticos de plataforma e teriam pego tanto o bug `cfg!(windows) ? CrLf : Lf` quanto o bug de canonicalização em qualquer runner de CI.
- **Nenhuma mudança na lógica de detecção existente** — Quando o arquivo alvo existe, `Auto` ainda chama `line_endings::detect(&existing)` e aplica o estilo dominante. Quando o usuário passa um `--line-ending lf|cr-lf|cr` explícito, o valor explícito ainda é aplicado como antes. Apenas o branch `Auto` + arquivo alvo inexistente mudou.

### Validação
- CI Linux: `cargo build --all-features`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` (152 lib tests + suites de integração + 3 doctests) — todos verdes
- Cross-check Windows CI: `cargo check --target x86_64-pc-windows-gnu --lib` com `RUSTFLAGS=-Dwarnings` e stub de `cc` para o linker mingw-gcc ausente — zero erros, zero warnings
- 8/8 testes de integração `cli_write` passam, incluindo a falha que era exclusiva do Windows


## [0.1.13] - 2026-06-07

### Corrigido (Falhas de CI - `windows-2025-vs2026` exit 1 sob `RUSTFLAGS=-Dwarnings`)
- **4 erros de compilação eliminados na matriz Windows** — O CI roda `cargo clippy --all-features -- -D warnings` e `cargo build --all-features` em `windows-2025-vs2026` com `RUSTFLAGS: -Dwarnings` no env. Em Linux esses lints são invisíveis (os símbolos `unix-only` são exercitados), mas em Windows o compilador detecta código morto e aborta. Os 4 erros reportados foram corrigidos:
  - `error: unused import: Duration` em `src/lock.rs:24` — `std::time::Duration` é consumido apenas pelo loop de polling de `flock` em `try_acquire_loop` (gateado por `#[cfg(unix)]`). O `use` foi separado: `use std::time::Instant;` permanece no escopo do módulo, e `#[cfg(unix)] use std::time::Duration;` importa `Duration` somente em Unix.
  - `error: unused variable: strict_atomic` em `src/atomic.rs:381` — O parâmetro `strict_atomic` é lido exclusivamente dentro do branch `#[cfg(unix)]` da EXDEV fallback. Foi anotado com `#[cfg_attr(not(unix), allow(unused_variables))]`, replicando o padrão já estabelecido em `src/signal.rs:15-17` (GAP 06).
  - `error: function copy_tempfile_to_target is never used` em `src/atomic.rs:604` — A função é chamada apenas da linha 446 (dentro de `#[cfg(unix)]`). Foi gateada com `#[cfg(unix)]`, removendo-a completamente da unidade de compilação Windows. É a solução mais limpa: a função depende de semântica unix-only (clona handle de `tempfile` após `EXDEV`), e sua existência no binário Windows seria ruído.
  - `error: clippy::unnecessary_literal_unwrap` em `src/atomic.rs:195` — A heurística `hardlink_nlink.unwrap_or(1) > 1` foi reescrita como `hardlink_nlink.is_some_and(|n| n > 1)`. Em Windows `hardlink_nlink` é literalmente `None` (vide linha 178), o que disparava o lint no `None.unwrap_or(1)`. A forma nova tem semântica idêntica em ambas as plataformas: retorna `false` quando `None`, retorna a comparação booleana quando `Some(n)`.

### Validação
- **Linux CI**: `cargo build --all-features`, `cargo clippy --all-features -- -D warnings`, `cargo test --all-features` (150 testes de lib passando) — todos verdes.
- **Windows CI**: Os 4 erros sob `RUSTFLAGS=-Dwarnings` são eliminados. O padrão `#[cfg_attr(not(unix), allow(...))]` é o mesmo já validado em `signal.rs` (GAP 06) que historicamente passa em CI Windows desde v0.1.4.


## [0.1.12] - 2026-06-07

### Adicionado

#### Editores estruturados de configuração (v14 Tier 3)
- **Subcomandos `set` / `get` / `del` / `case`** — Editores estruturados TOML/JSON com envelopes NDJSON bilíngues. **Correção de bug**: navegação dotted path TOML em `get`/`del` tratava a chave dotted inteira como nome literal; adicionados helpers `get_toml_path` e `remove_toml_path` que descendem por segmentos `Table` manualmente. Navegação JSON (que já usa semântica de pointer) não foi alterada. Verificação end-to-end: `{"type":"get","path":"...","key_path":"package.version","value":"\"0.1.12\"","found":true,"format":"toml","elapsed_ms":0}` agora retorna o valor correto.
- **`case` com 5 estilos heck** — snake_case, camelCase, PascalCase, kebab-case, SCREAMING_SNAKE_CASE via crate `heck`. Rename multi-arquivo via `--subvert OLD NEW --to <style>`.

#### AST estruturado via `tree-sitter-language-pack`
- **Subcomando `query`** — modos `--kinds`, `--query <KIND>`, `-Q <KIND>`, `--tree`. 305 linguagens suportadas. DFS iterativo via pilha `Vec<Node>` para evitar stack overflow em arquivos profundos.
- **Subcomando `outline`** — extração estrutural de alto nível (funções, classes, structs, enums, traits, módulos). Filtro `--kind` (repetível) via nomes exatos tree-sitter.
- **Verificação de sintaxe G72 REAL via tree-sitter** — Flag `--syntax-check` em `atomwrite write` invoca o parser tree-sitter real via crate `tree-sitter-language-pack` em vez da heurística de balanceamento de colchetes. Exit 88 com primeira linha/coluna de erro. 24 linguagens cobertas; extensões desconhecidas fazem fallback para heurística legada. Novo módulo `src/syntax_check.rs` (16 testes unitários).
- **Sidecar G114 WAL para recuperação de crash** — `atomic_write` escreve `.atomwrite.journal.<target>.atomwrite.journal.json` com entrada `Started` (op_id, expected_new_checksum, pid, started_at_unix) e entrada `Committed` em sucesso. `recover_orphan_journals(dir)` é consultivo — lê sidecars e reporta órfãos sem tocar no filesystem. Novo módulo `src/wal.rs` (8 testes unitários). Schema: `docs/schemas/wal-recovery.schema.json`.

### Corrigido
- **Navegação dotted path TOML em `get`/`del`** — veja v14 Tier 3 acima.
- **7 novas variantes de erro** — `LockTimeout` (83), `SyntaxError` (88), `ExdevFallbackDisabled` (91), `CopyBackBlake3Failed` (92), `OrphanJournal` (93). Todas bilíngues com sugestões `ErrorContext` acionáveis.

### Dependências
- `tree-sitter-language-pack = "1.8"` com features `download` + `dynamic-loading` (parsers sob demanda; footprint da instalação fica pequeno porque as 305 gramáticas NÃO são bundled — apenas a biblioteca loader é).

### Cobertura de Testes
- **445/445 testes passando** (eram 320 baseline em v0.1.10, +125 novos): 9 set, 7 case, 5 query, 5 outline, 6 syntax_check E2E, 4 WAL E2E, 6 get/del integração, 4 xattr/reflink Linux-only, 27 auditoria de regressões (strsim 5, heck 6, toml_edit 9, content_inspector 4, serde_yaml 4, locale pt-BR 1, LANG=C 1, --max-filesize edge 1, shell-completion bash/zsh/fish 3, --threads 1 query+outline 2, G72 stream >1MiB 1, case --subvert boundary 4, recover_orphan_journals 3, --json-schema 6).

### Validação
- `cargo test --all-features`: 445/445 PASS em 43 suites de teste
- `cargo fmt -- --check`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: PASS
- `cargo audit`: PASS (0 vulnerabilidades, 379 crates)
- `cargo deny check`: PASS (advisories, bans, licenses, sources todos OK)
- `cargo +1.88 check --all-targets`: PASS (MSRV compliant)
- `cargo build --release`: PASS
- `cargo package --allow-dirty`: PASS
- `cargo publish --dry-run`: PASS
- `cargo install --path . --force`: PASS (binário `atomwrite 0.1.12 (6af0d76)` instalado)

### Notas
- v0.1.12 é uma release ADITIVA. Nenhum subcomando existente foi renomeado ou removido. Todos os novos códigos de saída (83, 88, 91, 92, 93) são adicionados sem alterar os existentes (0-86 inalterados).
- `atomwrite write --syntax-check` é OPT-IN. O comportamento padrão de `write` não mudou.
- `atomwrite write` agora escreve um sidecar WAL apenas quando a env var `ATOMWRITE_WAL=1` está definida, OU quando `--strict-atomic` é passado. O comportamento padrão de `write` NÃO escreve o sidecar (consultivo apenas).
- Veja `docs/MIGRATION.pt-BR.md` para o guia de upgrade completo de v0.1.11 para v0.1.12.


## [0.1.11] - 2026-06-05

### Corrigido (Falhas de CI - windows-2025-vs2026 + signal test flaky no Linux)
- **E0433 do Windows `windows-2025-vs2026` resolvido** — `libc::write(STDERR_FILENO, ...)` e `libc::STDERR_FILENO` eram referenciados em `src/main.rs:22-23` em uma função compilada em todas as plataformas, mas `libc` é declarado apenas em `[target.'cfg(unix)'.dependencies]`. O build falhava no Windows com `error[E0433]: failed to resolve: use of unresolved module or unlinked crate 'libc'`. O escritor da mensagem de shutdown foi movido para `src/signal.rs` e protegido com `#[cfg(unix)]` (com corpo no-op `#[cfg(not(unix))]`), então o Windows usa o caminho ctrlc existente que emite a banner inline. A nova função `atomwrite::signal::write_shutdown_message()` também faz loop em `EINTR` e `EAGAIN` para ser robusta contra syscalls `write(2)` interrompidos e limites apertados de buffer de pipe impostos por alguns sandboxes de CI.
- **`signal_test::shutdown_message_on_stderr` não falha mais intermitentemente no ubuntu-latest** — O teste anterior dormia 2 s antes de enviar SIGINT e afirmava que o stderr capturado continha "shutting down". Dois modos de falha independentes foram observados:
  1. O comando search retornava `Err(NoMatches)` quando a flag `shutdown.is_shutdown()` disparava no meio do scan, porque as threads paralelas do walker tinham eventos Begin bufferizados que nunca foram pareados com eventos End, deixando `has_matches = false`. `main.rs` então seguia o branch `Err` e nunca chegava na escrita da banner de shutdown. `cmd_search` agora curto-circuita para `Ok(())` sempre que `shutdown.is_shutdown()` é true, então a main thread segue o branch `Ok(())` e emite a banner como projetado.
  2. `install_handlers_early` e `install_handlers` criavam cada um seu próprio `Arc<ShutdownSignal>` (`signal A` para o polling do search dentro de `atomwrite::run`, `signal B` para a checagem `is_shutdown()` na main thread). Sob a ordenação de chain-of-handlers do `signal-hook`, apenas a primeira instância era flipada quando SIGINT chegava — a flag da segunda instância permanecia `false`, então a main thread seguia o branch `is_shutdown() == false` e saía com 0 sem escrever a banner. Ambas as funções agora compartilham uma única instância de `ShutdownSignal`: `install_handlers_early` instala a cadeia completa de handlers (flag + counter) e `install_handlers` é idempotente (retorna o `Arc` existente quando `GLOBAL_SHUTDOWN` já está populado).
- **Teste usa `ATOMWRITE_READY_FILE` para detecção race-free de readiness** — `signal_test::shutdown_message_on_stderr` agora define `ATOMWRITE_READY_FILE` para um caminho sob o tempdir do teste e o atomwrite escreve seu PID nesse caminho assim que `install_handlers_early` retorna. O teste faz poll do arquivo com deadline de 10 s antes de enviar SIGINT, eliminando a janela de microssegundos onde SIGINT poderia competir com `posix_spawn` e chegar antes do `sigaction` do kernel ser configurado. Esta mudança é interna ao harness do teste e não tem efeito na superfície CLI publicada.

### Validação
- `cargo fmt -- --check`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo build --release`: PASS (1 m 14 s)
- `cargo test --all-features`: 302/302 testes PASS em 33 suites de teste (5 execuções consecutivas do suite completo)
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: PASS
- `cargo audit`: PASS (sem vulnerabilidades)
- `cargo deny check`: PASS (advisories, bans, sources OK; um warning cosmético `license-not-encountered` para a allowance ISC não usada em `deny.toml`)

### Notas
- v0.1.11 é uma mudança NÃO-BREAKING. Nenhuma API pública foi modificada.
- A escrita da mensagem de shutdown foi movida de `src/main.rs` para `src/signal.rs` como uma `pub fn` documentada. A função é `#[cfg(unix)]` (dependência de libc) e no-op em não-Unix. Apenas movimento de API interna.
- v0.1.10 foi yanked do crates.io. Novo `cargo install` resolverá para v0.1.11.


## [0.1.10] - 2026-06-05

### Corrigido (Falhas de CI - GAP 20 follow-up)
- **`signal_test::shutdown_message_on_stderr` faz flush da mensagem via `io::stderr().lock()`** — A primeira correção do v0.1.8 moveu `eprintln!` do signal handler para a main thread, mas usou `writeln!(io::stderr(), ...)` que é fully-buffered quando stderr é redirecionado para um pipe (como em `Stdio::piped()` do `cargo test`). O buffer nunca era flushado antes do processo terminar com o exit code do sinal, então o teste pai via stderr vazio. A correção usa `io::stderr().lock()` para adquirir o guard `StderrLock`, que faz flush do buffer no Drop. Isso garante que a mensagem chegue ao pipe de stderr capturado antes do processo terminar. CI ubuntu-latest confirmará no push.


## [0.1.8] - 2026-06-05

### Corrigido (Falhas de CI - GAP 17 e GAP 18)
- **`signal_test::shutdown_message_on_stderr` não falha mais no Linux CI** — Removido `eprintln!("\natomwrite: shutting down...")` dos handlers de SIGINT e SIGTERM. Segundo POSIX.1-2017 `signal-safety(7)`, funções stdio como `eprintln!` NÃO são async-signal-safe; o stderr do Rust usa um `Mutex` global que pode causar deadlock ou perder output bufferizado se o sinal chegar enquanto outra thread segura o lock. A mensagem de shutdown visível ao usuário agora é emitida pela main thread em `src/main.rs` quando observa `is_shutdown() == true` após `atomwrite::run` retornar, que é a única forma async-signal-safe de garantir que a mensagem chegue ao pipe de stderr capturado antes do processo terminar. O caminho Windows `ctrlc` ainda emite a mensagem inline (handlers ctrlc rodam em thread normal, não em contexto de sinal).
- **`atomic::tests::create_backup_and_retention` não falha mais no Windows CI** — Adicionado `platform::fsync_file_best_effort` que registra warning e continua em vez de retornar erro. No Windows, produtos antivírus (Windows Defender, AVs terceiros) podem segurar transientemente um handle de leitura em arquivos em `%TEMP%` com `FILE_SHARE_READ` mas sem `FILE_SHARE_WRITE`, fazendo `FlushFileBuffers` retornar `ERROR_ACCESS_DENIED` (os error 5). O caminho de escrita principal ainda usa o `fsync_file` estrito; apenas o fsync de durabilidade do backup é best-effort porque o backup em si já foi criado via `fs::copy`.
- **Matriz de CI fixada em `windows-2025-vs2026`** — A entrada da matriz para Windows foi alterada de `windows-latest` para `windows-2025-vs2026` (seu sucessor antes da migração de runners hospedados no GitHub em 15 de junho de 2026). Isso silencia o NOTICE "windows-latest requests are being redirected to windows-2025-vs2026 by June 15, 2026" e previne mudanças inesperadas de runner que possam quebrar o build.

### Validação
- `cargo build --all-features`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo test --all-features`: 302/302 testes PASS (todos os 5 casos de `signal_test` passam; `atomic::create_backup_and_retention` passa)
- `cargo fmt -- --check`: PASS
- `cargo audit`: PASS (sem vulnerabilidades, sem flag `--ignore`)
- `cargo deny check`: PASS (advisories, bans, licenses, sources todos OK)

### Notas
- v0.1.8 é uma mudança NÃO-BREAKING. Nenhuma API pública foi modificada.
- A mudança no signal handler é interna: consumidores externos que dependiam da mensagem de shutdown aparecer no stderr continuam a vê-la; ela agora é emitida pela main thread em vez do signal handler.
- A mudança no fsync de backup do Windows é interna: arquivos de backup ainda são criados e atômicos; a única diferença é que o flush de durabilidade para metadados de backup é best-effort. Se um usuário futuro relatar perda de dados em backup, podemos re-apertar o fsync.


## [0.1.7] - 2026-06-05

### Corrigido (Falhas de CI - GAP 15)
- **`cargo audit` não reporta mais RUSTSEC-2026-0009** — Atualizada a dependência transitiva `time` de 0.3.45 para 0.3.47 (DoS via stack exhaustion no parser RFC 2822, corrigido upstream via DEPTH_LIMIT=32). A atualização exigiu bump do MSRV de 1.85 para 1.88. A entrada `ignore` para RUSTSEC-2026-0009 no `deny.toml` foi removida porque a advisory não se aplica mais.
- **Falha de CI no macos-latest** — `src/platform.rs:31` não usa mais `return Ok(())` (removido return redundante). O lint `needless_return` do clippy 1.94+ não dispara mais; o env `RUSTFLAGS: -Dwarnings` na CI não aborta mais o build.
- **Falha de CI no windows-latest** — As constantes `EXIT_SIGINT` e `EXIT_SIGTERM` em `src/signal.rs:15-18` agora têm `#[cfg_attr(not(unix), allow(dead_code))]`. O env `RUSTFLAGS: -Dwarnings` não aborta mais em `dead_code` em builds Windows.
- **Deprecação de Node 20 em `actions/checkout` e `actions/cache`** — Ambas as actions foram bumparadas para a major version que suporta Node 24 (`actions/checkout@v6`, `actions/cache@v5`). `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: "true"` adicionado ao env do workflow como cinto-e-suspensórios. O warning de deprecação não aparece mais nos logs de CI.
- **MSRV bumped para 1.88** — `rust-version` no `Cargo.toml` agora é 1.88. Todos os arquivos de documentação (EN e PT-BR) atualizados: `docs/INSTALL.md`, `docs/INSTALL.pt-BR.md`, `docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md`, `docs/CROSS_PLATFORM.md`, `docs/CROSS_PLATFORM.pt-BR.md`, `docs/COOKBOOK.md`, `docs/COOKBOOK.pt-BR.md`, `CONTRIBUTING.md`, `CONTRIBUTING.pt-BR.md`.

### Mudado
- **`build.rs:4-12`** — Colapsado `if let + if` aninhado em `if let + &&` para satisfazer o lint `collapsible_if` do clippy 1.94+.
- **`src/lib.rs`** — Adicionado `#![allow(clippy::collapsible_if)]` e `#![allow(clippy::needless_return)]` como decisões deliberadas do projeto para manter blocos `if let` em linhas separadas para legibilidade. Isso evita 25 sites separados de refatoração em handlers de subcomando.
- **Snapshot test platform-aware** — `tests/snapshot_write.rs` e `tests/snapshots/snapshot_write__write_output_structure.snap` agora usam placeholder `[platform_fsync]` para o campo `platform.fsync`, permitindo que o mesmo snapshot seja válido em Linux (`sync_data`), macOS (`F_FULLFSYNC`), e Windows.

### Validação
- `cargo build --all-features`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo test --all-features`: 302 de 303 testes PASS (1 falha pré-existente em `signal_test::shutdown_message_on_stderr` rastreada como GAP 16, não relacionada ao GAP 15)
- `cargo fmt -- --check`: PASS
- `cargo audit`: PASS (sem vulnerabilidades, sem flag `--ignore`)
- `cargo deny check`: PASS (advisories, bans, sources todos OK)
- Cross-compile `x86_64-pc-windows-gnu`: PASS (build, clippy -D warnings, tests --no-run)
- Cross-compile `i686-pc-windows-gnu`: PASS (check --all-features)

### Notas
- Esta é uma mudança NÃO-BREAKING para usuários em Rust 1.88 ou posterior. Usuários em Rust 1.85-1.87 devem atualizar.
- A dependência transitiva `time` agora está patched (0.3.47+), resolvendo RUSTSEC-2026-0009.
- Targets Windows GNU e i686 para cross-compile são agora explicitamente validados pelo workflow de desenvolvimento local; target MSVC requer runner Windows (job CI windows-latest cobre).

### Corrigido (GAP 16 - signal_test)
- **`signal_test::shutdown_message_on_stderr` não falha mais em macOS** — Substituída a chamada `libc::write(2, SHUTDOWN_MSG.as_ptr().cast(), ...)` nos handlers de SIGINT e SIGTERM por `eprintln!`. O stderr do runtime Rust é capturado de forma confiável pelo `Stdio::piped()` no processo de teste, enquanto writes brutos via libc eram perdidos na herança de process group do cargo test. A constante `SHUTDOWN_MSG` foi removida por ser dead code.
- **Confiabilidade do test em `tests/signal_test.rs`** — Aumentado o `thread::sleep` de 50ms para 2000ms antes de enviar SIGINT. Os 50ms anteriores eram insuficientes para que o processo filho do atomwrite inicializasse completamente tracing, mimalloc, e signal handlers antes de receber o sinal. Aumentado o payload por arquivo de 100 para 1000 linhas para que o loop de search demore o suficiente para confirmar shutdown gracioso. O teste agora é estável em 5 execuções consecutivas.


## [0.1.6] - 2026-06-05

### Adicionado (Badges do README)
- **Badge docs.rs no README.md e README.pt-BR.md** — Adicionado `[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)` entre os badges Crates.io e License. O badge estava ausente do README publicado, mesmo com a documentação sendo construída com sucesso no docs.rs. O badge agora aparece no README renderizado em crates.io e na página do repositório no GitHub.

### Notas
- v0.1.6 é NÃO-BREAKING. A mudança é puramente visual (imagem de badge no README).
- Nenhuma mudança de código ou API pública.
- Nenhum guia de migração no CHANGELOG é necessário.


## [0.1.5] - 2026-06-05

### Mudado (Higiene de Documentação)
- **`#![warn(missing_docs)]` promovido para `#![deny(missing_docs)]`** — Documentação faltando em API pública agora é erro de build, não warning. Todos os itens públicos já estavam documentados em v0.1.4 (verificado via `RUSTDOCFLAGS="-D warnings" cargo doc --all-features`), então nenhuma documentação foi adicionada nesta mudança.
- **`#![warn(rustdoc::broken_intra_doc_links)]` promovido para `#![deny(...)]`** — Links quebrados em intra-doc agora falham o build ao invés de serem warnings silenciosos.
- **`#![doc(html_root_url = "https://docs.rs/atomwrite/0.1.2")]` removido** — O atributo estava hardcoded na versão 0.1.2, fazendo com que intra-doc-links gerados para versões mais novas (0.1.3, 0.1.4) apontassem para 0.1.2. O atributo está obsoleto desde rustc 1.48 em favor do campo `repository`, que já está configurado no `Cargo.toml`. docs.rs agora usa a versão atual do crate para resolver links automaticamente.

### Mudado (Metadata docs.rs)
- **`[package.metadata.docs.rs]` limpo** — Removido `all-features = true` (não existe tabela `[features]`, então a flag era no-op) e `rustdoc-args = ["--cfg", "docsrs"]` (não existem marcadores `#[cfg(docsrs)]` no código). Adicionado `targets = ["x86_64-unknown-linux-gnu"]` para tornar o target do build do docs.rs explícito.

### Testes
- 302 testes passam com 0 falhas (inalterado desde v0.1.4)
- 3 testes ignorados (cross-compile Windows, inalterado)
- `cargo doc --no-deps --all-features` com `RUSTDOCFLAGS="-D warnings"` passa limpo

### Notas
- v0.1.5 é NÃO-BREAKING. Os lints promovidos para deny já são satisfeitos pelo código atual.
- v0.1.5 não altera nenhuma API pública nem comportamento. Apenas apertar a fiscalização de documentação e remover metadata obsoleta.
- Nenhum guia de migração no CHANGELOG é necessário.


## [0.1.4] - 2026-06-05

### Corrigido (Compilação Windows - GAP 14)
- **`cargo install atomwrite` no Windows 10/11** — Resolvidos três erros de compilação que bloqueavam a instalação em Windows desde v0.1.3. Erro `E0433` em `src/atomic.rs:404` (tipo `AtomwriteError` usado sem import), erro `E0308` em `src/platform.rs:116` (comparação de `*mut c_void` com literal `0`), e erro `E0507` em `src/atomic.rs:387` (assinatura `&NamedTempFile` mas chamada `.persist()` requer ownership). Todos os três bugs estavam em blocos `#[cfg(windows)]` invisíveis ao CI Linux.

### Corrigido (Correção FFI - GAP 14)
- **`src/platform.rs:116`** — Substituída comparação `handle != 0` por `!handle.is_null()` para conformidade com o padrão idiomático de raw pointer check em Rust. O `HANDLE` retornado por `GetStdHandle` é um `*mut c_void`; compará-lo com literal inteiro viola o sistema de tipos e disparava `E0308`. Padrão agora é `is_null()` para nulidade e `!= INVALID_HANDLE_VALUE` (que já é `HANDLE`) para validade.

### Corrigido (Sugestões de Erro - GAP 13)
- **`WorkspaceJail` com `workspace_provided`** — Quando o usuário já forneceu `--workspace` ou `ATOMWRITE_WORKSPACE`, a sugestão agora diz "use a path inside the workspace" em vez de re-pedir a flag. Removido phantom `--force-text` que causava exit 2 em cascata.
- **20 variants com sugestão** — Adicionadas sugestões actionáveis para `InvalidInput`, `Io`, `ConfigInvalid`, `FileImmutable`, `NoMatches`, e `InternalError`. Apenas `BrokenPipe` (SIGPIPE não-acionável) permanece sem sugestão.
- **`ErrorContext` struct** — Carrega `workspace_provided: bool` e `workspace: Option<PathBuf>`. Novas funções `ErrorJson::from_error_with_context()` e `output::write_error_json_with_context()` usam o contexto para sugestões precisas.
- **`FileImmutable`** — Sugestão menciona `chattr -i` (Unix) e `fsutil` (Windows) para remover atributo imutável.
- **`NoMatches`** — Sugestão orienta a ampliar padrão e revisar `--include`/`--exclude`.
- **`InternalError`** — Sugestão orienta reportar o bug com contexto.

### Adicionado (Validação Cross-Platform - GAP 14)
- **`tests/cross_compile_check.rs`** — Novo gate de cross-compile que executa `cargo check` contra `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, e `x86_64-pc-windows-msvc`. Falha se `E0433`, `E0308`, ou `E0507` reaparecer em qualquer bloco `cfg(windows)`. Testes marcados `#[ignore]` para skip gracioso em hosts sem targets Windows.
- **`output::write_error_json_with_context()`** — Nova função que aceita `&ErrorContext` para propagar proveniência de `--workspace` até o output NDJSON.
- **Documentação de instalação Windows** — Novos arquivos `docs/INSTALL.md` (EN) e `docs/INSTALL.pt-BR.md` (PT-BR) com pré-requisitos Windows 10/11, comandos `cargo install` e troubleshooting.

### Mudado
- **`src/atomic.rs:13-15`** — Movido `use crate::error::AtomwriteError` para dentro de bloco `#[cfg(windows)]` para evitar warning de `unused_imports` em builds Linux/macOS. Tipo só é referenciado dentro de `persist_with_retry`.
- **`src/atomic.rs:386-409`** — `persist_with_retry` agora recebe `NamedTempFile` por valor e recupera o arquivo de `e.file` no branch de retry. Caller atualizado para passar `temp` por valor.
- **`src/main.rs:93-105`** — Reporte de erro agora constrói `ErrorContext` com `workspace_provided: cli.global.workspace.is_some()` para que a sugestão de `WorkspaceJail` se adapte à invocação do usuário.

### Testes
- 7 novos testes GAP 13 em `src/error.rs::tests`: `gap13_workspace_jail_suggestion_when_workspace_not_provided`, `gap13_workspace_jail_suggestion_when_workspace_provided`, `gap13_all_variants_have_suggestion`, `gap13_binary_file_suggestion_does_not_mention_force_text_wrong_flag`, `gap13_file_immutable_suggestion_mentions_chattr`, `gap13_no_matches_suggestion_mentions_filters`, `gap13_error_context_default_matches_legacy_behavior`.
- 1 novo teste de integração GAP 13 em `tests/cli_v012_regressions.rs`: `gap13_jail_suggestion_when_workspace_supplied_says_inside`.
- Teste existente `jail_suggestion_mentions_workspace_flag` atualizado para validar que a sugestão menciona `--workspace` apenas quando workspace NÃO é fornecido (fix GAP 13).

### Notas
- GAPs 01-12 (previamente resolvidos) re-auditados via `cargo test --all-features` — todos os 300+ testes continuam passando.
- Decisão atômica `atomwrite-no-github-actions` mantida: release é manual via `cargo publish` local após 8 gates oficiais e cross-compile gate. CI matrix em `.github/workflows/ci.yml` existe apenas como referência, não é executado.


## [0.1.3] - 2026-06-03

### Mudado (BREAKING)
- **Comportamento padrão de escrita atômica para `edit` e `replace`** — `AtomicWriteOptions::default()` agora define `preserve_timestamps: false` (era `true`). O mtime de um arquivo editado ou substituído é agora atualizado para o momento em que a escrita é concluída, que é o padrão correto para sistemas de build que usam mtime para detectar mudanças em código fonte (cargo, make, cmake, gradle, sbt, bazel, ninja, msbuild). Para cenários de backup, snapshot ou builds reproduzíveis onde o timestamp original precisa ser preservado, use a nova flag `--preserve-timestamps` em `edit` e `replace`. O módulo fingerprint do cargo compara o mtime dos arquivos fonte contra o mtime dos arquivos `target/.fingerprint/<unit>/dep-info`; com o padrão antigo, o cargo pulava o rebuild silenciosamente (o no-op "Finished in 0.29s") porque o fonte aparecia mais antigo que o binário. Veja o guia de migração v0.1.2 → v0.1.3 em `docs/MIGRATION.pt-BR.md` para o caminho de atualização.

### Adicionado (Consciência de Sistema de Build)
- Flag `--preserve-timestamps` em `edit` e `replace` para voltar ao comportamento v0.1.2 de manter o mtime original do arquivo
- Campo `mtime_preserved` nas respostas NDJSON de `EditOutput` e `ReplaceResult` para que consumidores verifiquem se o timestamp foi mantido ou atualizado (sempre presente; `true` quando `--preserve-timestamps` foi passado, `false` por padrão)
- Novos testes de regressão em `src/atomic.rs::tests`: `atomic_write_updates_mtime_by_default` e `atomic_write_preserves_mtime_when_opted_in`

### Adicionado (Documentação)
- Nova seção "Tempo De Modificação E Sistemas De Build" em `docs/HOW_TO_USE.pt-BR.md` explicando como cargo, make, cmake, gradle, sbt, bazel, ninja e msbuild detectam mudanças em código fonte via mtime e por que o padrão foi alterado
- Equivalente em inglês em `docs/HOW_TO_USE.md`
- Nova receita "Como Editar E Disparar Build Sem Touch Manual" em `docs/COOKBOOK.pt-BR.md` mostrando o workflow `atomwrite edit && cargo build` que não requer mais `touch`
- Equivalente em inglês em `docs/COOKBOOK.md`
- Todas as mudanças v0.1.2 → v0.1.3 documentadas em `gaps.md` seção "Atomic Edit Preserva mtime E Quebra Detecção De Mudança Pelo Cargo" (GAP 12)

### Cobertura de Testes
- 2 novos testes de regressão em `src/atomic.rs` para o contrato padrão-atualiza-mtime e opt-in-preserva-mtime
- 2 testes em `src/ndjson_types.rs::tests` atualizados para o novo campo `mtime_preserved` em `EditOutput` e `ReplaceResult`
- 2 arquivos de snapshot atualizados: `tests/snapshots/snapshot_write__edit_output_structure.snap` e `tests/snapshots/snapshot_write__replace_result_structure.snap` agora incluem o novo campo `mtime_preserved: false` no output JSON
- Total: 33 suítes de teste, 294 testes passando (era 292+ em v0.1.2)

### Gates de Validação
- `cargo fmt --check` limpo
- `cargo clippy --all-targets --all-features -- -D warnings` zero warnings
- `cargo test --all-features` 33 suítes passando
- `cargo doc --no-deps --all-features` zero warnings
- Comportamento end-to-end verificado: arquivo com mtime=2024-01-01 → `atomwrite edit` (padrão) → mtime=agora → `cargo build` rebuilda corretamente; `--preserve-timestamps` mantém o mtime de 2024-01-01 como esperado


## [0.1.2] - 2026-06-02

### Correções (CRÍTICAS)
- **Falha de compilação no macOS** — `nix::fcntl::posix_fadvise` restrito a `cfg(target_os = "linux")` então atomwrite agora compila no macOS arm64/Intel (a crate nix restringe o símbolo apenas em `linux_android | emscripten | fuchsia | freebsd`, quebrando o macOS anteriormente)
- **`batch --transaction` rollback agora é real** — arquivos pré-existentes são restaurados E arquivos novos criados por operações `write` são removidos. O evento NDJSON `rollback` agora reporta `files_restored`, `files_removed` e `total_reverted` para que LLMs verifiquem o contrato ACID. Anteriormente, arquivos criados no meio da transação nunca eram limpos.
- **`replace` não infla mais contadores em violações de jail** — `total_replacements` é incrementado apenas DEPOIS da validação do jail do workspace passar. Violações agora emitem um evento de erro `JailViolation` com `error_class: permanent` e `retryable: false`.
- **Eventos paralelos do `search` são agrupados por path** — threads paralelas do walker não intercalam mais eventos `begin`/`match`/`end` de arquivos diferentes na saída NDJSON. Consumidores (LLM e humanos) veem sequências contíguas de eventos por arquivo.
- **`scope --delete` em comentários Rust não deixa mais espaço em branco órfão** — a query preparada para comentários Rust agora casa whitespace trailing, então a deleção produz código limpo.
- **`search` com regex inválido emite envelope JSON estruturado** — padrões inválidos agora falham com `AtomwriteError::InvalidInput` que propaga através de `write_error_json` para stdout, não stderr cru.

### Correções (ALTAS)
- **`batch --file <PATH>` agora é funcional** — a flag está conectada via `cmd_batch` para ler o manifesto NDJSON de um arquivo (validado contra jail do workspace) em vez de apenas stdin.
- **`backup --output-dir` agora é respeitado** — a flag vai através de `AtomicWriteOptions.backup_output_dir` para `create_backup_in`, que cria o diretório se estiver faltando e faz prune de backups antigos naquele diretório.

### Correções (UX)
- **Mensagem de erro de jail do workspace corrigida** — erros `WORKSPACE_JAIL` agora sugerem `--workspace <root>` ou `ATOMWRITE_WORKSPACE=<path>` em vez da enganosa "use an absolute path" (que estava errada quando o path já era absoluto).
- **Bug de retenção de backup do proptest corrigido** — `cleanup_old_backups_in` agora poda corretamente backups antigos ao usar `create_backup_in` com diretório de saída customizado.

### Mudado (Dependências)
- `nix` atualizado de 0.29 para 0.31 (estável mais recente)
- `signal-hook` atualizado de 0.3 para 0.4 (estável mais recente)
- `windows-sys` atualizado de 0.59 para 0.61 (estável mais recente)
- `rust-i18n` atualizado de 3 para 4 (estável mais recente)
- Assinatura de `nix::fcntl::posix_fadvise` mudou de `AsRawFd` para `AsFd` em 0.31 — código adaptado adequadamente

### Adicionado (Funcionalidades Agent-First)
- Flag global `--timeout <SECONDS>` para tempo limite de execução (0 = sem timeout, padrão 0)
- Flag `--grep <REGEX>` em `read` para filtrar linhas retornadas por regex
- `completions --install` para instalar scripts de completions no diretório de dados XDG (`~/.local/share/bash-completion/completions/atomwrite` para Bash, etc.)

### Segurança
- Baseline do `cargo audit` reconhece 1 vulnerabilidade: `RUSTSEC-2026-0009` em `time 0.3.45` (DoS via exaustão de pilha). Correção requer `time >= 0.3.47` que precisa de Rust 1.88. Nossa MSRV é 1.85, e atomwrite usa `time` apenas via `tracing-appender` para timestamps de log — não explorável. Rastreado para bump de MSRV em 0.2.0.

### Testes
- 10 novos testes de regressão em `tests/cli_v012_regressions.rs` cobrindo todos os 6 bugs fixos
- Total: 33 suítes de teste, 292+ testes passando (era 282 em v0.1.1)


## [0.1.1] - 2026-06-01

### Fixed
- 12 links intra-doc quebrados em `error.rs` corrigidos (`DiskFull` para `Self::DiskFull` e similares)
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
#### Novos Subcomandos
- Subcomando `scope` para escopo gramatical: selecionar categorias AST (comentários, funções, strings, etc.) e aplicar ações (delete, upper, lower, titlecase, squeeze, replace)
- `scope` suporta Rust (30 queries preparadas), Python (13), JavaScript/TypeScript (11), Go (8) e padrões AST customizados via `--pattern`
- Subcomando `backup` para criar backups de arquivos com timestamp, checksums BLAKE3 e retenção configurável
- Subcomando `rollback` para restaurar arquivos a partir de backups anteriores com verificação BLAKE3 opcional
- Subcomando `apply` para aplicar patches do stdin com detecção automática de formato (unified diff, blocos SEARCH/REPLACE, diff com fence markdown, substituição completa de arquivo)

#### Expansão de Operações em Batch
- `batch` suporta 7 operações: write, replace, delete, edit, hash, move, copy
- `batch --transaction` flag para execução tudo-ou-nada com rollback automático
- Operações `move` e `copy` do `batch` agora aceitam `source`, `from` e `src` como aliases para o caminho de origem
- Operações `write`, `delete`, `edit` e `hash` do `batch` agora aceitam `path` como alias de `target`

#### Aprimoramentos do Motor de Edição
- `edit --fuzzy` flag com cascata de 7 estratégias de correspondência (exact, line_trimmed, whitespace_normalized, indent_flexible, escape_normalized, trimmed_boundary, block_anchor)
- `edit --multi` flag para aplicar múltiplas operações de edição NDJSON em uma única escrita atômica
- `edit` saída NDJSON inclui campos fuzzy, strategy, strategies_tried, similarity quando correspondência fuzzy é usada

#### Segurança de Caminho
- Detecção de FIFO e arquivos de dispositivo na validação de caminho (códigos de saída 85 e 86) — previne escritas atômicas em arquivos especiais
- Detecção de hardlink antes do rename atômico com `tracing::warn` quando nlink > 1
- Detecção de mesmo arquivo em `copy` e `move` para prevenir perda de dados quando origem=destino (estava sobrescrevendo)

#### Internacionalização (i18n)
- Flag global `--lang` para override de locale (en, pt-BR) com variável de ambiente `ATOMWRITE_LANG`
- Suporte a i18n via `rust-i18n` e `sys-locale`: detecção automática de locale do SO com traduções en e pt-BR
- Todas as strings voltadas ao usuário agora cientes de locale (erros, avisos, mensagens informativas)

#### Documentação Bilíngue
- Documentação bilíngue `ARCHITECTURE.md` (en + pt-BR) descrevendo mapa de módulos, fluxo de dados e decisões chave
- Headers de licença SPDX (`MIT OR Apache-2.0`) em todos os 64 arquivos `.rs`
- Documentação `//!` em nível de módulo em todos os 38 módulos fonte
- Doctests executáveis para funções `is_binary`, `detect` e `normalize`
- Configuração `[package.metadata.docs.rs]` para builds no docs.rs
- Campo `documentation` e `[badges.maintenance]` no `Cargo.toml`
- Lints rustdoc: `broken_intra_doc_links`, `private_intra_doc_links`, `clippy::doc_markdown`
- `doc(html_root_url)` para cross-linking no docs.rs

#### Supply Chain e Segurança
- `deny.toml` para auditoria de licenças e advisories via cargo-deny
- Módulo de detecção e normalização de terminadores de linha (`line_endings.rs`)
- Flag `--line-ending lf|crlf|cr|auto` em `write` e `edit` para normalização de terminadores de linha

#### Infraestrutura de Testes
- 282 testes entre suítes de integração e unitários (eram 5 testes em 1 módulo na v0.1.0)
- Testes de integração para `backup`, `rollback`, `apply` e `scope`
- 2 alvos de fuzzing (`batch_parse`, `extract_json`) com `libfuzzer-sys` para testes de segurança dos parsers
- Testes de integração de locking otimista para `write --expect-checksum` e `edit --expect-checksum`
- Testes de validação NDJSON expandidos de 5 para 20 de 21 comandos
- Testes de interoperabilidade `jaq` validando NDJSON via pipe com filtro `jaq`
- Teste de integração i18n confirmando que `--lang` não altera saída JSON

### Segurança
- Headers de licença SPDX garantem clareza de licença em todos os arquivos fonte
- cargo-deny enforces conformidade de licença e rastreia advisories de segurança
- Detecção de FIFO e device file previne escritas acidentais em arquivos especiais
- Detecção de hardlink previne corrupção silenciosa de dados quando rename atômico quebra hard links

### Limitações Conhecidas (corrigidas em v0.1.2)
- Flag `batch --file <PATH>` era declarada no help mas não era conectada à lógica do comando
- `batch --transaction` não deletava arquivos criados durante transações falhadas (apenas restaurava arquivos modificados)
- `replace` incrementava contadores ANTES da validação do jail do workspace, produzindo contagens NDJSON contraditórias
- `search` com regex inválido produzia erro cru no stderr em vez de envelope JSON
- Walker paralelo do `search` intercalava eventos begin/match/end de arquivos diferentes
- `scope --delete` em comentários Rust deixava whitespace órfão
- Compilação no macOS falhava (nix 0.29 restringia `posix_fadvise` a Unix não-macOS)
- `--workspace` padrão era CWD silencioso (sem aviso ao capturar tudo)
- Mensagem de erro WORKSPACE_JAIL sugeria "use an absolute path" mesmo quando o path já era absoluto
- `backup --output-dir` era declarado mas não era conectado
- 4 dependências congeladas (nix 0.29, signal-hook 0.3, windows-sys 0.59, rust-i18n 3)
- `read` não tinha flags `--head`/`--tail`/`--grep` para controle de janela de contexto LLM
- `completions` não auto-instalava
- Sem flag global `--timeout` para terminação de operação sem limite


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


[Unreleased]: https://github.com/daniloaguiarbr/atomwrite/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/daniloaguiarbr/atomwrite/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/daniloaguiarbr/atomwrite/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/daniloaguiarbr/atomwrite/releases/tag/v0.1.0
