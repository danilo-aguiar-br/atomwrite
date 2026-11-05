[Read in English](ARCHITECTURE.md)


# Arquitetura


## Visão Geral
- atomwrite é um CLI Rust de binário único para operações atômicas de arquivo
- Projetado para agentes LLM que precisam de manipulação de arquivos segura e estruturada
- Toda escrita é atômica: tempfile, fsync, rename, fsync do diretório
- Toda resposta é NDJSON no stdout com checksums BLAKE3
- Todos os logs de diagnóstico vão para stderr via `tracing` (`init_tracing` em `src/runtime.rs`, sem telemetria de produto): writer non-blocking, filtro via CLI `-v`/`-q`, arquivo de log opcional sob o XDG state (chave `log.file`), sem knobs de produto `ATOMWRITE_*`/`RUST_LOG` na v0.1.35+
- O plano de dados do agente permanece NDJSON em stdout


## Mapa de Módulos

### Ponto de Entrada
- `src/main.rs` — entrada do binário: setup de signals, init de tracing, dispatch
- `src/lib.rs` — raiz da library: declarações de módulo e o dispatcher `run()`
- `src/lib_dispatch.inc.rs` — dispatch de comandos e `startup_wal_heal`, o passe de `wal-heal` na partida que roda antes do dispatch, só para verbos que gravam e só nos diretórios citados no argv
- `src/runtime.rs` (+ `runtime_cli.inc.rs`) — helpers de partida do binário: tracing, locale, enriquecimento de erro do clap, prescan de schema
- `src/cli.rs` (+ `cli_global.inc.rs`) — clap `#[derive(Parser)]` com as flags globais
- `src/cli_args/` — structs de argumentos por subcomando e enums de valor, divididos em `mod.rs` e `part*.inc.rs`

### Pipeline Principal
- `src/atomic/` — pipeline de escrita atômica: `write.rs` orquestra tempfile + fsync + rename + fsync dir, `rename_path.rs` é o caminho do tempfile, `inplace.rs` guarda a escrita in-place, o fallback EXDEV e os backups com timestamp, `persist_retry.rs` refaz o persist de tempfile no Windows, `types.rs` e `time_fmt.rs` são helpers
- `src/checksum.rs` — cálculo de hash BLAKE3 para arquivos e slices de bytes (usa memmap2 para arquivos grandes)
- `src/file_io.rs` — leitura inteligente de arquivos: um `open`, um `fstat` no handle, uma leitura em buffer de tamanho exato (`try_reserve_exact`), o hash usa `checksum::hash_file`
- `src/platform.rs` — durabilidade e rename por plataforma: `Durability` full|fast|auto, Linux `renameat2` com fallback para rename, macOS `F_FULLFSYNC` em full, e o write reporta no NDJSON `platform.durability` e `platform.rename_method`
- `src/fuzzy/` — cascata compartilhada de match usada por edit, replace, batch e edit-loop: exato, `line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `trimmed_boundary`, `escape_normalized`, `block_anchor`, `unicode_normalized` e `context_aware` (com um passe Jaro-Winkler)
- `src/fuzzy/one_pass.inc.rs` — multi-apply one-pass (`apply_fuzzy_one_pass`) no conteúdo original, nunca reescaneia texto inserido, um pattern embutido no replacement força apply único, default max applies = 1, teto duro 10_000, cancel cooperativo polled no meio da cascata, caps (pattern 64 KiB, Levenshtein 8192 chars, janelas 4096, crescimento max(4x, +16 MiB))
- A saída fuzzy carrega `match_count` e `indent_adjusted` no `EditOutput`, estratégias normalizadas emitem `similarity: null` (não um 1.0 falso), `block_anchor` usa dual-gate na razão do miolo, e `context_aware` aplica um soft floor via XDG `[fuzzy].soft_floor_delta` (padrão 0.05)
- O reflink (copy-on-write) vive em `src/atomic/write.rs` e `src/atomic/inplace.rs`, pela crate `reflink-copy`

### Segurança e Validação
- `src/path_safety.rs` — jail do workspace: prevenção de path traversal, validação de symlink, detecção de FIFO/device, nomes reservados Windows (COM0–9/LPT0–9), NFC, helper de path longo `\\?\`
- `src/storage.rs` — config/data/cache/state via `directories::ProjectDirs` (XDG), sem knob de runtime `ATOMWRITE_HOME` na v0.1.35+
- `src/env_detect.rs` — autodetecção WSL/container/K8s/Termux/Flatpak/Snap para `doctor`, só por sondas de arquivo (nenhuma variável de ambiente é lida)
- `src/signal.rs` (+ `signal_handlers.inc.rs`, `signal_cancel.inc.rs`) — SIGINT/SIGTERM/SIGHUP e o handler de SIGPIPE via signal-hook com coordenação de shutdown gracioso, e `CancelCheck` para o cancel cooperativo
- `src/stdin_cancel.rs` — stdin cancelável: uma thread leitora que desiste quando a execução é cancelada ou o prazo passa
- `src/error/` — enum de erro de domínio com códigos de saída, classificação de erro e flag retryable, dividido em `variants.inc.rs`, `class.inc.rs`, `impl_methods.inc.rs`, `error_json.inc.rs` e `suggest.inc.rs`
- `src/lock.rs` — locking de arquivo advisory via `std::fs::File::try_lock`, que é `flock` no Unix e `LockFileEx` no Windows, no próprio inode do alvo; o sidecar `.<target>.atomwrite.lock` é tomado depois dele, por melhor esforço, para que versões antigas continuem mutuamente excluídas; o `LockGuard::target_unchanged` revalida o inode com `same_file` imediatamente antes da troca, o `delete` toma o mesmo lock antes de calcular o hash e antes do unlink, e o nome do sidecar vira um digest BLAKE3 curto quando o nome literal passaria de 255 bytes (ADR-0079)
- `src/walk_globs.rs` — o único lugar que compila os globs `--include` / `--exclude` de todo verbo, um glob que não compila é `INVALID_INPUT` (exit 65)

### Recuperação de Crash (v0.1.12, G114 + v0.1.15-v0.1.18, G119)
- `src/wal/` — o sidecar WAL, um módulo por responsabilidade
- `src/wal/journal.rs` — escritor do sidecar: anexa entradas `Started` e `Committed` em `.atomwrite.journal.<target>.atomwrite.journal.json`, e o RAII `JournalGuard` do G119 L2
- `src/wal/policy.rs` — `WalPolicy` (`Auto`, `Always`, `Never`) e as heurísticas de criação, incluindo `creation_throttled`, o limite de taxa H3 aplicado só quando um journal está prestes a ser criado
- `src/wal/heuristics.rs` — G119 L4 HeuristicsEngine: `h1_ttl`, `h2_lru_within_cap`, `h3_rate_limit`, `h4_sentinel`, `h5_archive`, onde `heuristics_should_preserve` agrega os votos de retenção de um journal confirmado e não consulta `h3_rate_limit`
- `src/wal/heal.rs` — `recover_orphan_journals(dir)`, `auto_heal_scoped` e o passe do `wal-heal`, que remove sidecars `Committed` e `Aborted`, e remove um sidecar `Started` só quando o PID dono não existe mais, publicado como `dead_owner_removed`
- `src/wal/stats.rs` e `src/commands/wal_stats.rs` — subcomando G119 L5 `wal-stats`: total_journals, by_state, oldest_age, total_size, by_directory, auto_heal_recommended, estimated_reclaim_bytes

### Verificação de Sintaxe (v0.1.12, G72)
- `src/syntax_check.rs` — verificação de sintaxe REAL via `tree-sitter-language-pack`, que substitui a heurística de balanceamento de colchetes do v0.1.11, com 23 linguagens mapeadas por extensão de arquivo
- Extensões desconhecidas fazem fallback para a heurística legada em `src/atomic/syntax_heuristic.rs`
- `src/syntax_check_stub.rs` — o stub usado quando a feature `ast` está desligada

### Saída
- `src/output.rs` (+ `output_lines.inc.rs`, `output_verdict.inc.rs`) — escritor NDJSON com tratamento de broken-pipe, e o carimbo de sucesso/falha do evento terminal
- `src/output_shape.inc.rs` e `src/output_shape/` — a shape global de runtime aplicada na emissão (`--select`/`--fields`, `--filter`, `--limit`, `--sort-by`, `--dedupe`, `--aggregate`, `--join`, `--diff-against`, `--redact`), estes não são campos de schema
- `src/ndjson/` — definições de tipos de saída com derivação de JSON Schema via schemars
- `src/schema_dispatch.inc.rs` e `src/schema_terminal.inc.rs` — a listagem do `--print-schema` e os schemas terminais fechados
- `src/error/` — o envelope de erro sempre emite `ok: false` (obrigatório) mais `match_count` em falhas de match (0 na maioria das falhas), com `best_candidate` opcional em miss/ambiguidade
- `src/constants.rs` — constantes nomeadas para tamanhos de buffer, thresholds e códigos de saída

### Utilitários
- `src/binary_detect.rs` — heurística de byte nulo para detecção de conteúdo binário
- `src/line_endings.rs` — detecção e normalização de LF/CRLF/CR
- `src/lang_utils.rs` — mapas de extensão de linguagens de programação para comandos AST
- `src/locale.rs` (+ `locale_persist.inc.rs`) — locale de UI: `Language`, detecção sys-locale, parse unic-langid, negociação fluent-langneg, OnceLock, arquivo de preferência XDG
- `src/xattr_restore.rs` — salvar e restaurar xattrs (quarantine do macOS, selinux/capabilities do Linux)
- `src/concurrency.rs` (+ `concurrency_walk.inc.rs`) — concorrência limitada e fan-out multi-core, coleta por `WalkParallel`
- `src/config.rs` — carga do `.atomwrite.toml` e do `config.toml` XDG (flags > workspace > global > padrões)

### Confiabilidade, Ajuste e Relato de Falhas (v0.1.37)
- `src/retry.rs` — retry com backoff exponencial, jitter total e prazo duro (`RetryPolicy`, `Backoff`, `retry()`), usado pelo lock advisory e pelo persist de tempfile no Windows, dormindo em fatias curtas e obedecendo o prazo global `--timeout-secs` e a flag de shutdown
- `src/best_effort.rs` — a forma sancionada de ignorar uma falha de propósito: `best_effort` registra em `debug` e segue, enquanto uma falha que muda o resultado é propagada com `?`
- `src/config_tuning.inc.rs` — seções de ajuste (`concurrency`, `search`, `lock`, `retry`, `extract`), sua validação e o acessor global `tuning()`
- `src/commands/config_cmd.rs` (+ `config_cmd_keys.inc.rs`, `config_cmd_write.inc.rs`) — o verbo `config`: uma tabela tipada de 43 chaves, leitura em camadas (flag, workspace, global, embutido), escrita atômica validada que preserva comentários
- `src/constants.rs` — inclui `constants_concurrency.inc.rs` (retry, teto de threads, orçamento do search), `constants_config.inc.rs` (nomes de arquivos de config, mínimos das chaves) e `constants_literals.inc.rs` (literais nomeados: nomes de sidecar, unidades de tempo, tamanhos)
- `src/commands/search_budget.inc.rs` — um orçamento global de bytes para os eventos do search em voo (padrão 8 MiB, chave `search.max_inflight_bytes`), e o stream que está sendo drenado nunca espera, então não há deadlock
- `src/ndjson/part3.inc.rs` — `FileFailure`, `FailureSummary`, `FailureTally` e `partial_of`: `files_failed`, `partial` e `errors[]` (no máximo 50) no resumo terminal de todo verbo que varre árvore, com `partial` derivado da mesma contagem
- `src/output_verdict.inc.rs` — carimbo de sucesso/falha do evento terminal e o marcador `VerdictPublished`, para que um resumo que falha seja a única linha de erro da execução
- `workspace_source` — todo envelope terminal publica `argv` ou `cwd`, então uma execução que tirou a raiz do diretório de trabalho do processo diz isso em vez de esconder a autoridade ambiente
- `replace --expect-checksum` — a comparação roda sob o lock e a divergência sai 82 `STATE_DRIFT`, fechando a janela entre a checagem e a escrita
- `batch --transaction` — todo lock de alvo é tomado em ordem de caminho antes do snapshot e mantido até commit ou rollback, então dois lotes esperam em vez de entrelaçar
- `edit-loop` — o checksum lido sob o lock é a base da escrita, então edição concorrente sai 82 em vez de perder atualização
- Erros tipados — os caminhos de borda de `cli.rs`, `rename_path.rs` e dos auxiliares de fsync levantam `AtomwriteError`, então diretório negado sai 13 `PERMISSION_DENIED` e nunca 255 `INTERNAL_ERROR`
- `src/commands/replace_file.inc.rs` e `src/commands/replace_finish.inc.rs` — o passo por arquivo e o passo final do `replace`, separados de `replace_run.inc.rs`
- `src/walk_globs.rs`, `src/stdin_cancel.rs` e `src/signal_cancel.inc.rs` — o compilador de globs compartilhado, o stdin cancelável e os helpers de cancel listados acima

### Handlers de Subcomando
- `src/commands/` — 47 subcomandos implementados (handlers + aliases, SSOT `CANONICAL_SUBCOMMAND_COUNT` / `agent-surface`), cada um no próprio módulo quando aplicável, e `batch/`, `edit/`, `extract_artifact/` e `scope/` são pastas
- Cada handler recebe args parseados, config global, escritor NDJSON e sinal de shutdown
- Todos os handlers seguem a mesma assinatura: `fn cmd_*(args, global, writer, shutdown) -> Result<()>`
- Baseline v0.1.11 (22): read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, completions
- Adicionados em v0.1.12 (6): set, get, del, case, query, outline
- Adicionados em v0.1.15 (2): wal-heal (G119 L3), wal-stats (G119 L5)
- Adicionados em v0.1.22 (2): edit-loop, prune-backups
- Adicionado em v0.1.25 (1): verify
- Adicionados em v0.1.29 (8): recipe, sparse, semantic-merge, agent-surface, watch, codemod, semantic-search, stat (alias de `read --stat`)
- Módulos de comando v0.1.29: `recipe.rs`, `sparse.rs`, `semantic_merge.rs`, `agent_surface.rs`, `watch.rs`, `codemod.rs`, `semantic_search.rs`
- Host / descoberta (completa 41 → 46): `doctor` (diagnóstico do host), `locale` (locale resolvido + preferência XDG), `commands` (manifesto da árvore de comandos), `scratch` (`scratch init` de staging sob `.atomwrite/scratch/`), e `extract-artifact`
- Adicionado em v0.1.37 (1): `config` (chaves persistentes de ajuste), o 47º subcomando
- Nota histórica: 41 era a superfície documentada até a v0.1.35, e o SSOT atual é 47

### Features Cargo (v0.1.29+)
- `core` — binário slim sem AST/watch (meta PRD 5–8 MiB, 26.560.352 bytes medidos em 2026-11-04, DERIVADO: 26560352 / 1048576 = 25,33 MiB, e ~7,7 MiB na v0.1.29)
- `ast` — ast-grep + tree-sitter + language-pack + serde-saphyr (padrão com lang-rust/ts/py)
- `lang-rust` / `lang-ts` / `lang-py` / `lang-go` / `lang-full` — superfície de linguagem para scope/transform
- `watch` — observação de filesystem via notify (subcomando `watch`), opt-in, e o build padrão responde `watch` com exit 78 e uma dica de rebuild
- `full` — default + lang-full + watch
- `semantic-search` não tem gate de feature: é sempre compilado (Jaccard offline)
- Features padrão: `core`, `ast`, `lang-rust`, `lang-ts`, `lang-py`
- Build slim: `cargo build --release --no-default-features --features core`, e nele `outline`, `query`, `scope`, `transform` e `sparse outline` saem 78 com dica de rebuild (`CONFIG_INVALID`), lido de `src/commands/*_stub.rs` e `src/commands/sparse.rs`, o build slim não foi executado para este documento
- `cargo xtask check-features` roda `cargo check --locked --no-default-features` nos conjuntos `core` e `core,watch` e para na primeira falha (`xtask/src/main.rs`)


## Fluxo de Dados

```
stdin ──> bytes de conteúdo
             │
             ├── [write/edit/apply] ──> atomic_write() ──> tempfile
             │                              │                 │
             │                              │              fsync(fd)
             │                              │                 │
             │                              │           rename(temp, target)
             │                              │                 │
             │                              │           fsync(dir)
             │                              │                 │
             │                              └──> checksum BLAKE3
             │
             ├── [search/replace] ──> WalkParallel ──> motor ripgrep
             │                              │
             │                       crossbeam channel
             │                              │
             │                         eventos NDJSON
             │
             └── [read/hash/list] ──> ops fs diretas ──> eventos NDJSON
                                                                 │
                                                            stdout (NDJSON)

Adições v0.1.12:
  write/edit ──> [se --syntax-check] ──> syntax_check.rs (tree-sitter)
                          │
                          └──> SyntaxError (exit 88) se erros encontrados
  write/edit ──> [se WAL habilitada] ──> wal/journal.rs (entrada Started)
                          │
                          └──> [após rename] ──> wal/journal.rs (entrada Committed)
  query/outline ──> parse tree-sitter ──> DFS iterativo ──> eventos NDJSON
  set/get/del/case ──> toml_edit / heck ──> eventos NDJSON

Adições v0.1.15 (G119):
  wal-heal ──> varre o workspace atrás de .atomwrite.journal.*.json antigos
                  │
                  └──> heuristics_should_preserve ──> ceifa ou pula
  wal-stats ──> agrega by_state, by_directory, idade, tamanho ──> NDJSON

Adições v0.1.18 (G118 resolve-first universal):
  write/edit/copy/apply/move/rollback/set/del/case/replace ──> validate_path (jail)
                          │
                          └──> WORKSPACE_JAIL (exit 126) na primeira violação
```


## Decisões Chave

### BLAKE3 ao invés de SHA-256
- BLAKE3 foi projetado para ser mais rápido que SHA-256 (benchmarks do upstream, não medidos aqui)
- Performance single-threaded importa para latência de CLI
- Não usado para segurança criptográfica, apenas detecção de integridade

### NDJSON ao invés de JSON Array
- Streaming: cada resultado é emitido assim que computado
- Sem necessidade de bufferizar a saída inteira antes de escrever
- Pipe-friendly: ferramentas downstream processam linha por linha
- Erros emitem no mesmo formato com `ok: false` mais `code` e `exit`, e o discriminador `error` da 0.1.37 e anteriores saiu

### tempfile + rename ao invés de In-Place Write
- Atômico: o arquivo alvo nunca fica meio escrito por uma escrita baseada em rename
- O comportamento após SIGKILL ou queda de energia depende do filesystem e de `--durability`
- Backup do original é uma cópia (não hardlink) para evitar corrupção de inode compartilhado
- Fallback in-place (v0.1.12, G55): quando `nlink > 1` (Unix) ou o alvo é um symlink, atomwrite troca para `ftruncate(0) + write_all + fsync_data` para preservar o inode, e o NDJSON ganha `write_strategy: "rename" | "inplace" | "copyback"`

### Jail do Workspace
- Todos os caminhos validados contra a raiz do --workspace
- Previne path traversal via componentes `..`
- Bloqueia symlinks apontando para fora do workspace
- Rejeita FIFO e arquivos de dispositivo (não-atômicos por natureza)

### Tratamento de Signal via signal-hook
- SIGINT e SIGTERM definem um flag atômico para shutdown cooperativo
- Um segundo signal força exit imediato via `_exit` (Unix) / `process::exit` (Windows)
- SIGPIPE não é ignorado nem deixado em `SIG_DFL`: um handler cooperativo (`register_broken_pipe` em `src/signal_handlers.inc.rs`) marca o shutdown e a execução sai com 141 depois de liberar locks, journals e tempfiles
- Isto é um desvio consciente da convenção simples de voltar à disposição padrão, registrado na ADR-0063, porque a morte pela disposição padrão deixava locks, journals e tempfiles para trás
- Singleton compartilhado `ShutdownSignal` (`OnceLock`) para que polling e main-thread `is_shutdown()` vejam a mesma flag
- O cancel cooperativo também cobre o `--timeout-secs` global (padrão 120, passe `0` para desligar, exit 124 no prazo, convenção GNU `timeout`)
- A cascata fuzzy faz poll da flag de cancel no meio do arquivo (estratégias + janelas deslizantes) para o timeout funcionar durante `replace --fuzzy` / edit / batch / edit-loop
- `signal::cancelled_error` carimba o exit code vivo (130 / 143 / 124), nunca hard-code SIGTERM
- Walks longos (`search`, `replace`, `count`, `list`, `hash`, `delete`, `batch`, …) fazem poll da flag e param de aceitar trabalho novo
- Escrita atômica cancela no meio do chunk sem renomear o tempfile (alvo permanece intacto)
- CLI one-shot: sem Tokio `CancellationToken` / `TaskTracker` / systemd `sd_notify` (N/A)

### G72 verificação de sintaxe REAL via tree-sitter (v0.1.12)
- Substitui a heurística de balanceamento de colchetes do v0.1.11 que tinha falsos positivos (indentação Python, template literals JS) e falsos negativos (`import` Python de módulo ausente)
- Usa `tree-sitter-language-pack` com as features `download` + `dynamic-loading`
- 23 linguagens são mapeadas por extensão, e extensões desconhecidas fazem fallback para a heurística legada
- Exit 88 com primeira linha/coluna/kind/mensagem de erro

### G114 sidecar WAL para recuperação de crash (v0.1.12)
- Caminho do sidecar: `.atomwrite.journal.<target>.atomwrite.journal.json`
- Anexa entradas `Started` (op_id, expected_new_checksum, pid, started_at_unix) e `Committed` (op_id, committed_at_unix)
- `recover_orphan_journals(dir)` é consultivo: lê sidecars e reporta órfãos sem tocar no FS
- O caller decide se deve fazer replay, abortar ou ignorar

### tree-sitter-language-pack com dynamic-loading (v0.1.12, ADR-0019, ADR-0057)
- Parsers NÃO são bundled
- São baixados no primeiro uso de uma linguagem e cacheados em `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` (`bundles`, `libs`, `manifest.json`), a partir dos assets de release de `github.com/xberg-io/tree-sitter-language-pack`, cujo manifesto 1.22.1 lista 372 linguagens
- O manifesto da 1.22.1 lista 23.031.807 bytes (cerca de 22 MiB) para o pacote `linux-x86_64`, medido em 2026-10-31, e as execuções seguintes reutilizam o cache
- O cliente HTTP é o `ureq`, que lê as variáveis de proxy do ambiente, e um host sem rede pode preencher o diretório de cache antes
- Alternativa: `tree-sitter` como dep direta, mas adiciona centenas de crates de parser ao tempo de compilação (ADR-0019)

### Arquitetura v14 Tier 3 (v0.1.12)
- `set/get/del` usam `toml_edit` (preserva comentários e ordem das chaves) para TOML e `serde_json` (canônico) para JSON
- `get/del` usam descida manual de `Table` via os helpers `get_toml_path` e `remove_toml_path` (ADR-0024) em vez de `toml_edit::Document::get`, que trata chaves dotted como literais
- `case` usa a crate `heck` para 5 estilos de identifier-case
- `query/outline` usam DFS iterativo via pilha `Vec<Node>` para evitar stack overflow em arquivos profundos (em comparação com travessia recursiva via `TreeCursor`)

### Cascata fuzzy compartilhada para edit multi-par (v0.1.15, G117, ADR-0026)
- `match_pair(content, old, new, fuzzy_mode)` extrai a cascata, compartilhada pelos caminhos de par único e multi-par
- O multi-par roda a cascata por par, e o conteúdo evolui entre pares (a ordem é comportamento definido)
- Envelopes de sucesso ganham `pairs_total` + `pair_results`, e falhas levantam `EditPairFailed` com `failed_pair_index` (reutiliza `INVALID_INPUT`, exit 65)
- `--partial` (opt-in) aplica os pares que casam, e zero pares aplicados mapeia para `NO_MATCHES` (exit 1)

### Alvo do write resolvido primeiro (v0.1.15, G118, ADR-0027)
- `cmd_write` resolve o alvo via `validate_path` uma única vez, antes de append/prepend, detecção automática de line ending e `--expect-checksum`
- Elimina a dupla identidade de caminho (CWE-367): com CWD divergente, o append não trunca mais e a divergência de checksum sai com 82
- Convenção: todo comando mutante DEVE resolver o alvo via `validate_path` antes de qualquer `exists()`/leitura

### L1 WalPolicy + L4 HeuristicsEngine (v0.1.16, G119, ADR-0028, ADR-0062)
- `WalPolicy { Auto, Always, Never }` permite ao caller ajustar quando o sidecar WAL é escrito, e `Auto` o cria para alvo acima de 1 MiB, para `edit`/`replace`, ou fora de diretório git, e pula uma escrita trivial (4 KiB ou menos) dentro do git
- `crate::wal::heuristics` agrega os votos de retenção via `heuristics_should_preserve(target, committed_at_unix, count, rank)`, e as alavancas usam constantes + configuração XDG/CLI na v0.1.35+ (sem knobs de produto `ATOMWRITE_WAL_*` em env)
- O limite de taxa H3 é só um voto na criação (`creation_throttled`): depois de 10 journals abertos em 60 s o `Auto` para de abrir novos enquanto as escritas continuam atômicas, e um journal que foi gravado é sempre removido (ADR-0062)
- O campo `wal_policy` em `WriteOutput` NDJSON expõe a decisão por chamada

### L3 auto-heal no startup (v0.1.17, G119, ADR-0028, ADR-0062)
- `atomwrite` executa um passe autônomo de `wal-heal` antes do dispatch (`startup_wal_heal` em `src/lib_dispatch.inc.rs`) com threshold de 3600 s e budget de 100 ms
- Roda só para um verbo que grava, e só nos diretórios citados no argv, nunca no workspace inteiro
- Remove sidecars `Committed` e `Aborted` e nunca um `Started`, que é o órfão que merece atenção depois de um crash
- Opt-out pela flag global `--no-auto-heal` (`src/cli_global.inc.rs`), e registra info estruturado quando ceifa, debug quando nada para ceifar, warn em falha

### Guard de stdin vazio em 4 camadas (v0.1.16, G120, ADR-0029)
- L1 rejeita 0 bytes do stdin por padrão em `read_stdin_content` com opt-out `--allow-empty-stdin`
- L2 rejeita stdin vazio em `handle_append_prepend`
- L3 emite um warning `tracing::info!` quando `--append` + `--expect-checksum` + stdin vazio combinam ambiguamente, opt-out via `--no-checksum-when-empty`
- L4 sempre emite `stdin_bytes_read: u64` em `WriteOutput` NDJSON para gate tardio de agente

### G118 resolve-first universal + G117 casos de borda (v0.1.18, ADR-0030)
- 10 comandos mutantes pré-validam paths raiz via `validate_path` antes de construir qualquer walker ou worker: `write`, `edit`, `copy`, `apply`, `move`, `rollback`, `set`, `del`, `case`, `replace`
- `replace /etc/passwd` aborta em microssegundos com um único envelope `WORKSPACE_JAIL` em vez de caminhar o filesystem inteiro
- 3 testes de regressão de caso de borda G117: exact-match Unicode (diacríticos UTF-8), preservação de line ending CRLF após replace, multi-par onde o mesmo `--old` aparece duas vezes
- 1 teste de integração G120 L3: a combinação cross-flag `--append + --expect-checksum + --allow-empty-stdin` está coberta end-to-end

### Internacionalização
- Traduções embutidas em compile-time via rust-i18n (`locales/en.toml`, `locales/pt-BR.toml`)
- Detecção: `sys-locale` uma vez no startup, e no Linux essa crate lê `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` e `LANG`, enquanto `directories` lê `XDG_*` e `HOME` para os caminhos de configuração
- O código do produto não chama `env::var` (um teste guarda isso), e as leituras das dependências estão listadas na ADR-0057
- Normalização de tags brutas com `unic-langid` (`_` → `-`, remove `.UTF-8`), e `C`/`POSIX` não geram preferência de idioma
- Negociação com `fluent-langneg` (`NegotiationStrategy::Lookup`) contra os locales disponíveis
- Enum tipado `Language` (`En` | `PtBr`, `#[non_exhaustive]`) + `OnceLock<LocaleState>` imutável
- Precedência: CLI `--locale` → preferência em `storage::config_dir()` (XDG → locale) → SO → `en` (sem knobs env de produto)
- Override: flag global `--locale` (value_parser clap, campo Rust `lang` estável), e não existe knob de produto `ATOMWRITE_LANG` na v0.1.35+
- Diagnóstico: relatório NDJSON de `atomwrite locale`, `locale --set` / `locale --clear` para a preferência XDG
- Códigos NDJSON e mensagens Display de erro permanecem em inglês (contrato de máquina do agente)
- Sugestões de erro e avisos humanos selecionados em stderr seguem o locale resolvido via `t!`
- Features opcionais `i18n-cjk` / `i18n-rtl` / `i18n-europe` / `i18n-full` são placeholders: o binário padrão só embute `en` + `pt-BR` completos
- `build.rs` declara `cargo:rerun-if-changed` para `locales/`, `locales/en.toml` e `locales/pt-BR.toml`
- Testes de paridade de chaves: `tests/locale_parity.rs`

### Política de macros (uma macro declarativa local, sem crate proc-macro)
- Decisão: o código de produção define uma única macro local do projeto, `invalid_input!` (`src/error/mod.rs`, `macro_rules!`), que monta um erro `AtomwriteError::InvalidInput` a partir de uma string de formato
- O harness de testes tem mais uma, `impl_hermetic!` em `tests/common/mod.rs`, e não existe crate local `proc-macro = true`
- Esgotar generics, traits, funções e `const` asserts antes de introduzir macro
- Macros do ecossistema só onde sintaxe não pode ser função:
  - Derive: `clap`, `serde`, `thiserror`, `schemars` para CLI / NDJSON / erros
  - Atributo: `#[tracing::instrument]` nas entradas de comando
  - Function-like (externas): `rust_i18n::i18n!`, `schemars::schema_for!`, `tracing::{info,debug,…}!`
  - Built-ins std: `env!` / `option_env!` (versão), `matches!`, `vec!`, `format!`, `write!`/`writeln!`, `const _: () = assert!(…)`
- Preferir structs tipadas `Serialize` + `JsonSchema` em vez de `serde_json::json!` ad-hoc para contratos NDJSON de agente
- `serde_json::json!` aparece só na montagem de JSON Schema (`src/schema_dispatch.inc.rs`, `src/schema_terminal.inc.rs`) e nos valores dinâmicos das chaves do verbo `config` (`src/commands/config_cmd*.rs`), e o re-tag de filhos usa `output::ndjson_insert_field`
- Sem `todo!` / `unimplemented!` / `dbg!` em caminhos de produção, e os guardas de código-fonte proíbem `panic!` e `.unwrap()` ali (veja Arquitetura de Testes)
- Sem crate local `proc-macro = true` (forçaria o custo de compilação de `syn`/`quote` num CLI que não precisa de DSL)


## Estratégia de Erro
- Enum `AtomwriteError` com derives Display via thiserror
- Cada variante mapeia para um código de saída compatível com sysexits
- Classificação de erro: permanent, transient, conflict, precondition_failed
- Erros transient e conflict são marcados como retryable para loops de retry de agentes
- Todos os erros serializam para NDJSON no stdout com campos legíveis por máquina
- O campo `suggestion` em `ErrorJson` fornece orientação de recuperação acionável para cada variante de erro
- A struct `ErrorContext` (adicionada em v0.1.4) carrega `workspace_provided: bool` e `workspace: Option<PathBuf>` do parser CLI para a saída de erro
- `ErrorJson::from_error_with_context(err, &ErrorContext)` produz sugestões context-aware
- A sugestão de `WorkspaceJail` se adapta com base em se o usuário forneceu `--workspace` (somente CLI/XDG na v0.1.35+)
- O legado `ErrorJson::from_error(err)` delega para `from_error_with_context` com `ErrorContext::default()` (compatibilidade retroativa)
- 38 variantes de erro no total (`rg -c '#\[error' src/error/variants.inc.rs`), e o manifesto do agente lista 42 entradas de exit code com 34 números distintos (`atomwrite agent-surface --format json | jaq '.exit_codes'`)
- v0.1.24 auditoria de erros tipados: TODOS os `anyhow::bail!()` voltados ao usuário convertidos para variantes `AtomwriteError`, e nenhum caminho de erro retorna exit 1 genérico sem envelope JSON
- Uma execução com falha termina com exatamente um evento terminal com `ok: false` (ADR-0061)


## Registros de Decisão de Arquitetura (ADRs)
- Veja `docs/decisions/README.md` para o índice completo de ADRs, que é a autoridade desta lista
- As ADRs 0019 a 0095 vivem em `docs/decisions/`, cada uma com tradução pt-BR, todas no formato Michael Nygard (Status, Context, Decision, Consequences, Alternatives, Trigger to revisit)
- 0019 — tree-sitter-language-pack em vez de dependência direta do tree-sitter
- 0020 — caminho e formato do sidecar WAL
- 0021 — v14 query/outline aceitam apenas kind names, não S-expressions
- 0022 — G72 tree-sitter substitui a heurística de colchetes
- 0023 — a recuperação WAL do G114 é consultiva, não automática
- 0024 — get/del em TOML usam descida manual de Table no caminho com pontos
- 0025 — `--positions` é opt-in e limitado aos modos `--query`/`--tree`
- 0026 — G117 edit multi-par: paridade fuzzy, relato por par, `--partial` opt-in (v0.1.15)
- 0027 — G118 write resolve o alvo contra o workspace antes de todos os pré-passos (v0.1.15)
- 0028 — G119 limpeza WAL autônoma em 5 camadas: L1 WalPolicy, L2 JournalGuard, L3 auto-heal no startup, L4 HeuristicsEngine, L5 wal-stats (v0.1.15-v0.1.17)
- 0029 — G120 guard de stdin vazio e validação cruzada de `--append`, `--expect-checksum` e stdin vazio (v0.1.16)
- 0030 — v0.1.18: replace pré-valida paths raiz, teste G120 L3, casos de borda G117
- 0031 — search e replace resolvem paths raiz contra o workspace por um helper compartilhado (G121)
- 0032 — G122: casamento real de S-expressions no `query`
- 0033 — consolidação das derivas de documentação de exit codes da v0.1.19
- 0034 — testes guiados por help: anti-padrão e o processo corretivo da v0.1.20
- 0035 — defesa em profundidade em seis camadas para o subcomando `write` (v0.1.20)
- 0036 — `--partial` no `edit`: par único continua NO_MATCHES, multi-par aplica os que casam
- 0037 — `GlobalArgs.lang` renomeado para `--locale`, liberando o namespace `--lang` (v0.1.20)
- 0038 — backup cumprido é apagado: apagar após o sucesso é o padrão (v0.1.21)
- 0039 — helper edit-loop: N edições em 1 invocação (v0.1.22)
- 0040 — subcomando prune-backups: limpeza manual de backups legados (v0.1.22)
- 0041 — `allow_hyphen_values` para 12 campos CLI de texto em 8 structs desde a v0.1.37, porque os três posicionais exigem `--` (v0.1.23)
- 0042 — backup por padrão para comandos que mutam conteúdo (v0.1.23)
- 0043 — guarda de shrink com `--expect-checksum` (v0.1.23)
- 0044 — `--old-file`/`--new-file` para o comando edit (v0.1.23)
- 0045 — sugestões acionáveis para erros de parsing do clap (v0.1.24)
- 0046 — retrofit diff resolve-first (v0.1.24)
- 0047 — correção do modo read-only do scope e migração para `find_all` (v0.1.24)
- 0048 — `BackupOpts` unificado e um único resolvedor `resolve_backup()`, com precedência flags CLI > `.atomwrite.toml` `[defaults]` / XDG > default embutido (knobs env removidos na v0.1.35)
- 0049 — config viva encanada: `load_config` chamado uma única vez em `lib.rs::run()`, `DefaultsSection` propagado a cada handler mutante (v0.1.28)
- 0050 — guard de stdin-tty: `main.rs` calcula `stdin.is_terminal()` e propaga `stdin_is_tty` até `cmd_edit`, então modos consumidores de stdin falham rápido com exit 65 em vez de bloquear (v0.1.28)
- 0051 — fechamento de gaps da v0.1.29, superfície de agente e honestidade do binário
- 0052 — contrato de agente residual da v0.1.30
- 0053 — exceções da auditoria de regras do clap (v0.1.32)
- 0054 — contrato one-shot fuzzy da v0.1.34: multi-apply one-pass, `--timeout-secs` padrão 120 / exit 124, cancel polled no meio do fuzzy, caps de recurso
- 0055 — contrato agent-first da v0.1.36 (GAP-2026-019…033): jail em paths de content-source + staging scratch, envelope de erro `ok: false`, `similarity: null`, globais `--select`/`--filter`/`--limit` (ver `docs/decisions/0055-v0-1-36-agent-contract-gaps.pt-BR.md`)
- 0056 — remover o teto de compilação `release_max_level_info` do `tracing`
- 0057 — leituras de ambiente por dependências, download de parsers, MSRV 1.90 e stack YAML
- 0058 — o verbo `config` e as chaves de ajuste tipadas
- 0059 — um módulo de retry com jitter total e prazo global
- 0060 — um orçamento de bytes para os eventos do `search` em voo
- 0061 — um evento terminal por execução com falha (`publish_failure` e `VerdictPublished`)
- 0062 — limite de taxa do WAL na criação do journal e um `startup_wal_heal` limitado
- 0063 — SIGPIPE: um handler cooperativo com exit 141 (desvio consciente)
- 0064 — teto automático de 16 threads e um pool rayon criado sob demanda
- 0065 — teto de conteúdo do fuzzy e polls de cancel cooperativo
- 0066 — `DoctorChecksFailed`: `doctor --strict` sai 1 com uma linha terminal
- 0067 — schemas terminais fechados e um `--print-schema` fiel
- 0068 — testes herméticos e guardas de código-fonte
- 0069 — divisão do `constants.rs` e dos módulos grandes em partes `.inc.rs`
- 0070 — `watch` vira uma feature opt-in
- 0071 — `extract-artifact`: sinal limitado, nunca bytes brutos
- 0072 — superfície agent-native: shape de stream, `--print-schema` antes do clap, guardas de efeito colateral
- 0073 — um ponto terminal de cancel e `deadline_exceeded`
- 0074 — o apply estrito roda todo read-modify-write sob um lock
- 0075 — grammar pack 1.22, ast-grep 0.51 e MSRV 1.90
- 0076 — o `read` aceita vários arquivos no formato raw
- 0077 — família de resposta e honestidade da varredura
- 0078 — um schema por verbo, servido como união
- 0079 — o lock é tomado no inode do alvo
- 0080 — o `sparse outline` conta o journal sob `--hidden`
- 0081 — os campos do veredito nomeiam o mecanismo e o sujeito
- 0082 — a família sparse segue uma política só
- 0083 — `--hidden` segue a flag em todo verbo que varre
- 0084 — todo verbo que muta publica o veredito de escrita
- 0085 — a família edit publica o veredito de escrita
- 0086 — o `apply` publica o caminho do backup e o rollback
- 0087 — o snapshot da transação segue a política de backup
- 0088 — a deleção guarda o desfazer e a reversão declara o que manteve
- 0089 — o alvo é sempre array e o batch nomeia as cópias
- 0090 — a transação é dona da cópia e a escrita que falha nomeia o que deixou
- 0091 — a transação empresta a cópia para uma reescrita
- 0092 — o delete copia só quando pedido
- 0093 — a chave de backup distingue ausente de false
- 0094 — o delete recursivo domina a árvore nomeada inteira
- 0095 — os offsets de submatch da busca são bytes


## Arquitetura de Testes
- Suítes de integração: 112 arquivos `.rs` em `tests/` (conte com `fd -e rs -d 1 . tests`), mais testes unitários colocalizados em `src/` e doctests, e o total por teste não é congelado neste documento (conte `#[test]` com `rg -c '#\[test\]' tests src`)
- Testes unitários são colocalizados com o código sob módulos `#[cfg(test)]`
- Testes de integração vivem em `tests/` e usam `assert_cmd` + `predicates` para testes shell-out
- Todo teste que roda o binário passa por `common::hermetic(...)` (`tests/common/mod.rs`): `HOME` e `XDG_*` vazios, `LANG` fixo, e o cache XDG real para os parsers AST não serem baixados de novo (ADR-0068)
- Guardas de código-fonte quebram o build: `tests/cli_v0137_hardcode_guard.rs` (literais de duração, infixo `.bak.`, nomes de sidecar, números de unidade de tempo fora de `constants*`, `env::var`, `Command::new`, nenhum arquivo `.sh`, nenhum diretório `.github`) e `tests/cli_v0137_let_underscore_guard.rs` (`let _ =`, `.unwrap()`, `.expect(`, `panic!`, `todo!`, `unimplemented!` em código de produção), os dois com listas de exceção vazias
- Guardas de documentação: `tests/docs_consistency.rs` e `tests/docs_exit_codes.rs`
- Testes property-based via `proptest` para checksum, backup e fuzzy
- Gate de cross-compile via `tests/cross_compile_check.rs`
- Testes de snapshot via `insta` para saída NDJSON estável
- Dois alvos de fuzz em `fuzz/fuzz_targets/` (`batch_parse`, `extract_json`)
- Contrato de agente v0.1.36: `tests/cli_v0136_gaps.rs`


## Contratos residuais v0.1.35 (notas de arquitetura)
- Guarda de write grande: default-deny quando o alvo existente excede XDG `[write].confirm_large_bytes`, o que exige `--ack-overwrite` (one-shot), e `--require-large-ack` é independente (não é alias clap de `--confirm`)
- Superfície delete: `--plan` / `--dry-run` plan-only, e `--confirm` e `--yes`/`-y` são rejeitados fail-closed
- Watch: sempre emite o NDJSON terminal `type:watch_summary`, idle padrão 500 ms, debounce da CLI ou do XDG `[watch].debounce_ms`
- Semantic-merge: markers de conflito default ON, e `--no-conflict-markers` desliga
- SRP: monólitos secundários fatiados via `include!` (ADR-0069)
- Observabilidade: somente `init_tracing`, diagnósticos locais em stderr, sem telemetria de produto
- Config: knobs de produto só CLI + XDG / `.atomwrite.toml`, e o `doctor` não lê nenhuma variável de ambiente (a detecção do host usa só sondas de arquivo)
- Idioma do contrato: `message` e `reason` dos erros NDJSON e o texto de ajuda do clap são em inglês por contrato, e só a `suggestion` é localizada (en e pt-BR, com `suggestion_lang`)


## Contrato de agente v0.1.36 (ADR-0055)
- Histórico (v0.1.36 e início da v0.1.37): 46 subcomandos (`doctor`, `locale`, `commands`, `scratch` completaram o salto 41 → 45, `extract-artifact` veio na v0.1.37), e o SSOT final da v0.1.37 é 47 com `config`
- Envelope de erro sempre `ok: false` (obrigatório), `match_count` presente (0 na maioria das falhas de match)
- Honestidade fuzzy: estratégias normalizadas → `similarity: null`, dual-gate em `block_anchor`, soft floor via XDG `[fuzzy].soft_floor_delta` (padrão 0.05) em `context_aware`
- Paths de content-source (`--old-file`/`--new-file`) permanecem sob o jail do workspace, e agentes estagiam via `scratch init` → `.atomwrite/scratch/` (política, não vulnerabilidade)
- Flags de shape global (`--select`/`--fields`, `--filter`, `--limit`) remodelam stdout em runtime, não são campos de schema NDJSON, e erros sempre emitem sob `--filter`


## Contratos da v0.1.37 e v0.1.38 — leitura limitada, patch honesto, lock no inode

### Leitura limitada
- O `read` compõe o teto (`read.max_output_bytes`, 24.000 bytes), as janelas de faixa, `--number`, `--symbol`, `--line N --enclosing` e `--no-comments` numa passada só sobre o mesmo visitante de linhas
- O caminho raw lê alvo único em streaming pelo mesmo `stream_raw_targets` que o caminho de vários alvos já usava, então o pico de memória não cresce mais com o tamanho do arquivo, e o BLAKE3 só roda com `--verify-checksum`
- O teto corta em linha completa e no fim do último item do outline, o evento NDJSON carrega `truncated: true` com a dica `next`, e o aviso no stderr carrega o `--lines` exato que lê o resto
- `--no-comments` sem `--number` é recusado, porque uma linha removida apontaria o `apply` seguinte para a linha errada

### Formatos compactos
- `search --format text`, `list --format text` e `outline --format tsv` respondem no formato que as ferramentas nativas do agente já parseiam
- `search.format`, `read.number` e `output.profile` são chaves XDG, então o formato barato não depende de uma flag repetida em toda chamada
- O `outline` parseia bytes crus, publica `parse_errors` com `first_error_line` e fecha uma seção Markdown na linha anterior ao próximo título de nível igual ou maior
- O `search` fecha com `ALL_SKIPPED_BINARY` (exit 89) quando todo candidato era binário, então "nada foi buscado" deixa de parecer "nada casou"

### Contrato do patch
- O `apply` lê todo arquivo de um patch multi-arquivo, casa linha inteira e recusa bloco ambíguo ou malformado antes da primeira gravação
- Uma gravação que falha depois de outro arquivo já escrito restaura os arquivos já gravados, então a chamada não deixa patch pela metade
- A checagem de sintaxe compara a contagem de erros antes e depois da troca e recusa só quando ela sobe, então arquivo que já tinha erro continua editável (ADR-0086)
- `apply --show` e `diff` publicam linhas a partir de 1, e o `diff` separa `old_line` de `new_line`

### Lock e veredito
- O lock é `std::fs::File::try_lock` no inode do alvo, e o sidecar `.<alvo>.atomwrite.lock` é arquivo de compatibilidade cuja falha não aborta mais a escrita (ADR-0079)
- O envelope de falha carrega `ok`, `code`, `exit`, `path` e `suggestion`; `message`, `error_class`, `retryable` e `suggestion_lang` só aparecem com `-v`, e `target_resolved` é sempre array (ADR-0089)
- Todo verbo que muta publica `write_strategy` e, quando uma cópia foi recusada, `backup_skipped` (ADR-0084, ADR-0085)
- O `delete --recursive` assume a árvore inteira numa chamada e publica `backups_kept`, `backup_bytes` e `dirs_left` quando existe backup ou sobra, então o chamador sabe o que sobrou sem varrer de novo


## Manutenção
- Este documento e `ARCHITECTURE.md` são mantidos juntos, então uma mudança em um chega ao outro na mesma entrega
- As contagens aqui (variantes de erro, schemas, arquivos de teste) vêm dos comandos mostrados ao lado delas, meça de novo antes de mudar um número
