# Guia de Testes do atomwrite


[Read in English](TESTING.md)


## Objetivos
- Gastar menos tokens: um objeto NDJSON por chamada substitui o encadeamento de shell, e o `commands --brief` responde em 5.020 bytes onde a árvore completa custa 67.492
- Reduzir o custo financeiro, porque a conta acompanha os tokens e as idas e voltas, e uma chamada substitui as várias chamadas de shell que uma tarefa de arquivo pede
- Reduzir os erros com escrita atômica e checksum BLAKE3 em toda resposta, então um resultado velho ou parcial aparece em vez de ser presumido
- Reduzir as alucinações com discriminador `type`, schema fechado por verbo e código de saída que nomeia a falha
- Reduzir o tempo gasto em cada tarefa, porque a leitura, a busca e a escrita rodam num processo só em vez de numa corrente de ferramentas de shell
- Reduzir a latência com execução local e sem rede no caminho do arquivo, medida nas medianas de 5,6 ms no `read` e 6,7 ms no `write` de arquivo pequeno
- Multiplicar a velocidade das operações com `batch`, `edit-loop`, `replace`, `case`, `transform` e `codemod` resolvendo muitos arquivos numa chamada
- Reduzir o lead time entre a solicitação e o resultado verificado, porque a resposta já traz o checksum, o `target_resolved` e o próximo passo
- Tornar o agente mais rápido, mais veloz, mais eficaz e mais assertivo, já que o código de saída, o `code` e o `suggestion` dizem o que fazer sem tentar às cegas


## Linha de Release Atual: v0.1.38
- A suíte completa saiu 0 em 2026-11-04 com `cargo test --no-fail-fast`, e a corrida imprimiu 134 blocos `test result: ok`
- O schema do envelope de falha exige `ok`, `code` e `exit`, então uma suíte que afirmava `error: true` passa a ler `.ok == false` mais `.code`
- Nenhum teste foi acrescentado para as correções da 0.1.38 e as suítes existentes foram atualizadas no lugar, porque a prova é a execução real no binário instalado e não uma maquete nova
- O binário 0.1.38 foi executado em 2026-11-04 e respondeu ao `agent-surface` com 47 ferramentas, 42 códigos de saída e 34 números distintos, e ao `config list` com 43 chaves
- Os 47 exemplos do smoke test abaixo rodaram contra o binário 0.1.38 em 2026-11-04, e 46 saíram 0 enquanto o `watch` saiu 78


## O Que Há de Novo na v0.1.37
- Superfície completa da CLI sob política de teste: 47 subcomandos (`CANONICAL_SUBCOMMAND_COUNT`): `agent-surface`, `apply`, `backup`, `batch`, `calc`, `case`, `codemod`, `commands`, `completions`, `config`, `copy`, `count`, `del`, `delete`, `diff`, `doctor`, `edit`, `edit-loop`, `extract`, `extract-artifact`, `get`, `hash`, `list`, `locale`, `move`, `outline`, `prune-backups`, `query`, `read`, `recipe`, `regex`, `replace`, `rollback`, `scope`, `scratch`, `search`, `semantic-merge`, `semantic-search`, `set`, `sparse`, `stat`, `transform`, `verify`, `wal-heal`, `wal-stats`, `watch`, `write`
- Suites de documentação: `cargo test --test docs_coverage --test docs_consistency --test docs_exit_codes --test docs_schemas_sync`
- O `docs_coverage` exige todo subcomando e toda flag local em cada documento principal, e o `docs_consistency` confere todo total citado contra o binário real e prova que o próprio leitor reprova uma contagem velha
- O `docs_exit_codes` compara as tabelas de saída do README com o `agent-surface`, e o `docs_schemas_sync` mantém os arquivos de schema e os nomes publicados em passo
- Verbo `config`: `cargo test --test cli_v0137_config_verb` (ida e volta de set/get/unset, escopo obrigatório, valor inválido não grava nada, camadas chave a chave, gravação atômica 0600)
- Evento terminal e relato de falhas: `cargo test --test cli_v0137_terminal_event --test cli_v0137_terminal_pipe --test cli_v0137_terminal_schema` (um evento terminal, `MATCH_FAILED` para par ausente, `files_failed` e `errors[]`, exit 141 com leitor fechado, os seis schemas de resumo)
- Retry, cancelamento e limites: `cargo test --test cli_v0137_retry_deadline --test cli_v0137_retry_guards --test cli_v0137_cancel --test cli_v0137_search_rss --test cli_v0137_wal_startup_heal`
- Guardas de código-fonte (quebram o build): `cargo test --test cli_v0137_hardcode_guard --test cli_v0137_let_underscore_guard`
- `cli_v0137_hardcode_guard` varre o código de produção em `src/` (pula arquivos `constants*`, arquivos de teste, itens `#[cfg(test)]` e comentários). Ele proíbe literais de duração em `Duration::from_*`, o infixo `.bak.`, nomes de sidecar e de sonda e números de unidade de tempo fora de `constants*`, e proíbe `env::var`, `env::var_os`, `env::vars` e `Command::new` em qualquer código de produção. Também confere que o repositório não traz nenhum arquivo `.sh` nem diretório de automação remota. A lista `ALLOWED` de exceções está vazia
- `cli_v0137_let_underscore_guard` proíbe `let _ =` em código de produção, e proíbe ali `.unwrap()`, `.expect(`, `unreachable!(`, `panic!(`, `todo!(` e `unimplemented!(`. As duas listas de exceção estão vazias
- Harness hermético: todo teste que roda o binário passa por `common::hermetic(...)` em `tests/common/mod.rs`. O filho recebe `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME` e `XDG_STATE_HOME` vazios sob o diretório tmp do target do cargo, `LANG` e `LC_ALL` fixados em `en_US.UTF-8`, e `LANGUAGE` e `LC_MESSAGES` removidos
- O `XDG_CACHE_HOME` continua apontando para o cache real da máquina de propósito: a feature `ast` baixa os parsers tree-sitter ali no primeiro uso, e um cache vazio mandaria os testes de `outline`, `query` e `--syntax-check` para a rede
- Um teste que precisa da própria config redireciona `HOME` e `XDG_*` depois de chamar `hermetic`, porque a chamada `env` posterior vence
- Versão e build slim: `cargo test --test cli_v0137_version`; `cargo xtask check-features` confere os conjuntos de features `core` e `core,watch`
- A execução de teste suportada usa as features padrão: `cargo test --locked`. Não use `cargo test --no-default-features --features core` como gate (veja Perfis de teste local)
- DoD local (só gates locais, sem automação remota): `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`
- Pin docs/agentes em `^0.1.38`


## Smoke Test dos 47 Subcomandos
- Um exemplo por subcomando, cada um executado contra o `atomwrite 0.1.38` (o comando, o resumo de uma linha e o exemplo)
- Preparação: `WS` é o caminho absoluto de um workspace vazio que contém `src/a.rs` (um `unwrap()` e um comentário), `src/b.rs` (a palavra `beta` uma vez), `src/c.rs` (com `user_id`), `src/tmp.rs`, `config.toml` (uma tabela `[package]` com `name` e uma tabela `[features]` com `x`), `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` e `rules.yaml` (uma lista YAML com `id`, `language`, `pattern` e `rewrite`)
- `HASH_A` é o `checksum` que `hash src/a.rs` imprime
- Rode-os com `HOME` e as variáveis `XDG_*` apontando para dentro do workspace, como o `common::hermetic` faz na suíte de testes, para não tocar em nenhuma config, preferência de locale ou arquivo de completion reais
- Os exemplos rodam na ordem abaixo: o `rollback` precisa do `backup` anterior e o `edit-loop` precisa do `edit` anterior
- Resultado: 46 dos 47 saem 0, e o `watch` sai 78 porque o build padrão não tem a feature `watch`. Um build com `--features watch` não foi executado

- `read` — Lê um arquivo com checksum, tamanho e linhas, com opção de só início, fim ou faixa. Exemplo: `atomwrite --workspace "$WS" read --head 2 src/a.rs`
- `write` — Cria ou sobrescreve um arquivo de forma atômica a partir do stdin. Exemplo: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` — Edita um arquivo por texto exato ou aproximado, número de linha ou marcador. Exemplo: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `delete` — Apaga arquivos ou árvores com backup, ou só mostra o plano. Exemplo: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `copy` — Copia um arquivo ou árvore com verificação de checksum. Exemplo: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `move` — Move ou renomeia um arquivo de forma atômica. Exemplo: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `apply` — Aplica um patch (diff unificado, SEARCH/REPLACE, arquivo inteiro) vindo do stdin. Exemplo: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `search` — Busca conteúdo de arquivos em paralelo com o motor do ripgrep. Exemplo: `atomwrite --workspace "$WS" search alpha src/`
- `replace` — Troca texto em vários arquivos com escrita atômica e casamento aproximado. Exemplo: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `transform` — Reescreve código por padrão de AST (ast-grep). Exemplo: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `hash` — Mostra o checksum BLAKE3 de arquivos. Exemplo: `atomwrite --workspace "$WS" hash src/a.rs`
- `count` — Conta linhas e arquivos por extensão ou por tamanho. Exemplo: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` — Compara dois arquivos (unificado, estatística ou mudanças). Exemplo: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `list` — Lista uma árvore de diretórios com metadados. Exemplo: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` — Extrai campos de NDJSON lido do stdin. Exemplo: `printf '{"a":1}\n' | atomwrite --workspace "$WS" extract --field a`
- `extract-artifact` — Sinal limitado dos cabeçalhos de PDF, OOXML e imagem (nunca os bytes brutos). Exemplo: `atomwrite --workspace "$WS" extract-artifact img.png`
- `scope` — Escopo gramatical: age sobre comentários, strings, funções e outras categorias de código. Exemplo: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `regex` — Gera uma regex a partir de exemplos. Exemplo: `atomwrite --workspace "$WS" regex abc-12 xyz-99 id-7 --digits`
- `calc` — Cálculo e conversão de unidades. Exemplo: `atomwrite --workspace "$WS" calc "2 hours to seconds"`
- `completions` — Gera ou instala completions de shell (bash, zsh, fish, elvish, powershell). Exemplo: `atomwrite --workspace "$WS" completions bash`
- `backup` — Cria backups com data e hora e checksum BLAKE3. Exemplo: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` — Restaura um arquivo a partir de um backup anterior. Exemplo: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `batch` — Executa operações NDJSON do stdin, com opção de transação única. Exemplo: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
- `edit-loop` — Aplica N pares antigo/novo numa escrita só, tudo ou nada. Exemplo: `printf '[{"old":"gamma","new":"delta"}]\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` — Remove arquivos .bak antigos por idade ou quantidade. Exemplo: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run --recursive .`
- `wal-stats` — Mostra o estado dos journals WAL (só leitura). Exemplo: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` — Remove journals WAL terminais órfãos mais antigos que um limite. Exemplo: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `verify` — Confere um arquivo contra um hash BLAKE3 esperado (exit 0 se igual, 81 se diferente). Exemplo: `atomwrite --workspace "$WS" verify src/a.rs HASH_A`
- `set` — Grava um valor num caminho com pontos em arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` — Lê um valor num caminho com pontos em arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` — Remove uma chave de um arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` — Renomeia identificadores entre estilos de caixa em vários arquivos. Exemplo: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` — Percorre a AST do tree-sitter e emite os nós. Exemplo: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` — Emite a estrutura de alto nível de um arquivo de código. Exemplo: `atomwrite --workspace "$WS" outline src/a.rs`
- `semantic-merge` — Merge de três vias por linha com marcadores de conflito (não é AST). Exemplo: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `sparse` — Lista, leitura e outline com orçamento para árvores grandes. Exemplo: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576`
- `recipe` — Lista e roda receitas embutidas de vários passos. Exemplo: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `stat` — Só metadados, alias de read --stat. Exemplo: `atomwrite --workspace "$WS" stat src/a.rs`
- `agent-surface` — Manifesto de ferramentas para agentes (a fonte única dos subcomandos). Exemplo: `atomwrite --workspace "$WS" agent-surface --format json`
- `watch` — Transmite eventos do sistema de arquivos com debounce e termina com watch_summary (exige build com `--features watch`, o build padrão sai 78). Exemplo: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500`
- `codemod` — Aplica uma lista YAML de regras de reescrita por AST numa árvore. Exemplo: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` — Ranking offline por Jaccard de arquivos para uma consulta. Exemplo: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `doctor` — Diagnóstico de prontidão do host. Exemplo: `atomwrite --workspace "$WS" doctor`
- `scratch` — Cria o diretório de staging .atomwrite/scratch dentro do jail do workspace. Exemplo: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`
- `locale` — Mostra, define ou limpa o idioma da interface. Exemplo: `atomwrite --workspace "$WS" locale`
- `commands` — Imprime a árvore completa de comandos em JSON. Exemplo: `atomwrite --workspace "$WS" commands --include-globals`
- `config` — Lê e grava chaves de ajuste persistentes (43 chaves, escopos global e workspace). Exemplo: `atomwrite --workspace "$WS" config get fuzzy.threshold`


## O Que Há de Novo na v0.1.36
- Histórico (v0.1.36): CAPA residual de contrato de agente, erros always-emit sob `--filter`; extract `--field`; `CANONICAL_SUBCOMMAND_COUNT` era 46
- Histórico (v0.1.36): a superfície completa da CLI sob política de teste era de 46 subcomandos (`CANONICAL_SUBCOMMAND_COUNT`): `agent-surface`, `apply`, `backup`, `batch`, `calc`, `case`, `codemod`, `commands`, `completions`, `config`, `copy`, `count`, `del`, `delete`, `diff`, `doctor`, `edit`, `edit-loop`, `extract`, `get`, `hash`, `list`, `locale`, `move`, `outline`, `prune-backups`, `query`, `read`, `recipe`, `regex`, `replace`, `rollback`, `scope`, `scratch`, `search`, `semantic-merge`, `semantic-search`, `set`, `sparse`, `stat`, `transform`, `verify`, `wal-heal`, `wal-stats`, `watch`, `write`
- A suíte `tests/cli_v0136_gaps.rs` cobre pontos de contrato GAP-019…033 (jail do workspace / staging scratch, erro `ok:false` + `match_count`, honestidade fuzzy com estratégias normalizadas emitindo `similarity: null` (nunca 1.0 falso), política dual-gate / soft-floor, flags de shape, extract `--field` e não `--select`, fórmulas de staging)
- Gate SSOT local: o inventário de gaps é nota local não versionada, então o status corrente de cada gap vive na série de ADRs e a revisão mantém os dois alinhados
- Rode: `cargo test --test cli_v0136_gaps`
- DoD local (só gates locais): `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`
- Pin histórico `^0.1.36`; ADR-0055


## O Que Há de Novo na v0.1.35
- Residuais A-* fechados: suite verde sem `delete --yes`; sobrescrita grande default-deny (`--ack-overwrite`); `watch` sempre emite `watch_summary`; markers de semantic-merge default ON; monólitos fatiados com `include!`
- DoD local (só gates locais): `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`, `cargo check --target x86_64-pc-windows-gnu`, `cargo install --path . --force`
- Suite de contrato: `cargo test --test cli_e2e_v0135`
- Veja o ADR-0055 para a matriz de closeout A-*


## O Que Há de Novo na v0.1.34
- A suíte `tests/cli_v0133_oneshot_fuzzy.rs` trava o contrato one-shot fuzzy + timeout (fix de código registrado como 0.1.33; publicação docs-complete 0.1.34)
- Cobre: hang de expand-section quando NEW embute OLD termina em menos de 2 segundos; embeds forçam apply único via `apply_fuzzy_one_pass`; teto rígido 10_000 applies; timeout global padrão 120 / exit 124; cancel cooperativo no meio da cascata / exit 143
- Contrato explícito sob teste: `apply_fuzzy_one_pass` one-pass E→D (nunca reescaneia texto inserido); NEW embutindo OLD não pode hangar
- Rode: `cargo test --test cli_v0133_oneshot_fuzzy`
- Mantenha as suítes mais antigas listadas abaixo
- Veja [ADR-0054](decisions/0054-v0-1-34-oneshot-fuzzy-timeout.pt-BR.md)


## O Que Há de Novo na v0.1.30
- A suíte `tests/cli_v0130_agent_contract.rs` cobre o contrato residual: match_count, indent_adjusted, modos fuzzy, recipe bak skip, sparse outline kind, semantic-merge help line-based, unicidade replace-all. Nota (v0.1.35): `--fuzzy off` = exact-only (G-010), não rejeitado
- A suíte `tests/cli_v0129_fuzzy_replace.rs` cobre a superfície 0.1.29: replace fuzzy, `best_candidate`, durability, recipe, sparse, semantic-merge, stat, agent-surface, semantic-search, `platform.rename_method`
- `replace --fuzzy auto|aggressive|off` (`off` = exact-only, G-010; padrão `auto`) com `--fuzzy-threshold` opcional e `--progress-every` em massa
- Envelopes de falha de match podem incluir `best_candidate` (schema `best-candidate.schema.json`)
- Cancel cooperativo emite eventos NDJSON `cancelled` (schema `cancelled-event.schema.json`, exit 143)
- O NDJSON do write reporta `platform.rename_method` (`renameat2` ou `rename`)
- `write --durability full|fast|auto` para trade-off de custo de fsync
- Histórico (na v0.1.29): novos subcomandos: `semantic-merge`, `sparse`, `recipe`, `stat`, `agent-surface`, `watch` (feature), `codemod`, `semantic-search` — 41 subcomandos no total
- Matriz de features: slim `core` (26.560.352 bytes = 25,33 MiB medidos em 2026-11-04, acima do size-gate local de 15 MiB) vs default AST (71.084.440 bytes = 67,79 MiB medidos em 2026-11-04)
- Gates locais (manuais; sem scripts locais remoto): size-gate slim ≤15 MiB, cujo build slim medido em 2026-11-04 é 25,33 MiB e fica acima dele; core-test; schema-diff em `docs/schemas`
- Rode a suíte de contrato 0.1.30: `cargo test --test cli_v0130_agent_contract`
- Rode a suíte 0.1.29: `cargo test --test cli_v0129_fuzzy_replace`


## O Que Há de Novo na v0.1.28
- 661 testes passando, 0 falhas, 3 ignorados (gate de cross-compile)
- BREAKING: `delete` agora cria backup por padrão (era opt-in) — passe `--no-backup` para desabilitar
- BREAKING: `move`/`copy` sobrescrevendo um destino existente agora exigem `--force` OU um `--backup` explícito
- GAP-CLI-SURFACE-DRIFT resolvido: struct única `BackupOpts` achatada via `#[command(flatten)]` em 15 subcomandos mutantes, substituindo 15 conjuntos de flags divergentes declarados manualmente
- GAP-CONFIG-DEFAULTS-DEAD resolvido: chaves `backup`/`retention` de `[defaults]` do `.atomwrite.toml` agora são efetivas de ponta a ponta via uma única chamada `resolve_backup()` alimentada por `load_config()` em `lib.rs::run()`
- Novo guard compartilhado de stdin-tty no `edit`: 7 modos consumidores de stdin falham rápido com exit 65 `INVALID_INPUT` em vez de bloquear indefinidamente quando stdin é um terminal
- Novos arquivos de teste: `cli_v0128_backup_matrix.rs` (12 testes), `cli_v0128_config_defaults.rs` (6 testes), `cli_v0128_edit_stdin_guard.rs` (3 testes), `cli_v0128_batch_backup.rs` (6 testes)
- 33 subcomandos, 32 ADRs (0019-0050)


## O Que Há de Novo na v0.1.27
- 631+ testes passando, 0 falhas, 3 ignorados (gate de cross-compile)
- 10 bugs corrigidos: BUG-SEC-001 (CRÍTICO escape via symlink-dir), BUG-SCOPE-004 (CRÍTICO scope --delete destrói código), BUG-GET, BUG-001, BUG-002, BUG-005, BUG-008, BUG-SCOPE-002, BUG-SCOPE-003, BUG-SCOPE-005
- 3 limitações conhecidas documentadas: GAP-01 (test-fn), GAP-02 (doc-comment), GAP-03 (JS export)
- `tests/cli_get_del.rs` atualizado: `get` chave ausente espera exit 65 INVALID_INPUT (era exit 4)
- Teste unitário `lookup_rust_queries_known` do `scope.rs` atualizado para padrões com pub
- Teste unitário `lookup_go_queries_known` do `scope.rs` atualizado para var sem tipo
- 33 subcomandos, 29 ADRs (0019-0047)


## O Que Há de Novo na v0.1.25
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
- 609 testes passando (575 baseline v0.1.22 + 12 GAP-2026-015 + 7 GAP-2026-016 + 4 GAP-2026-017 + 8 GAP-2026-018 + 3 de [Unreleased] Windows)
- 33 subcomandos (32 da v0.1.22 + `verify` da v0.1.25)
- 26 ADRs em docs/decisions/ (0019-0044)
- GAP-2026-015 (allow_hyphen_values) fechado; 12 campos CLI em 8 structs aceitam valores iniciando com `-`, e os três posicionais exigem `--` desde a 0.1.37
- GAP-2026-016 (backup-by-default) fechado; backup habilitado por padrão em 9 structs que mutam conteúdo
- GAP-2026-017 (guarda de shrink) fechado; writes que reduzem >50% bloqueados quando --expect-checksum ativo
- GAP-2026-018 (--old-file/--new-file) fechado; edit lê match/substituição de arquivos, contornando ARG_MAX
- Novos testes: `tests/cli_v0123_hyphen_values.rs` (12), `tests/cli_v0123_backup_default.rs` (7), `tests/cli_v0123_shrink_guard.rs` (4), `tests/cli_v0123_old_file.rs` (8)


## O Que Há de Novo na v0.1.22: Totais de Teste
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


## O Que Há de Novo na v0.1.12 (histórico)

Esta seção resume as mudanças relevantes para testes em v0.1.12. A release adicionou 96 novos testes (eram 320 baseline) para um total de 445 testes em 43 suítes. A v0.1.15 eleva o total para 542 testes (+2 testes unitários na v0.1.14, +8 do G117 em `tests/cli_edit.rs`, +6 do G118 em `tests/cli_write.rs`).

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


### Como Executar

```bash
# Executar todos os testes
cargo test

# Executar a suíte one-shot fuzzy + timeout v0.1.33/v0.1.34
cargo test --test cli_v0133_oneshot_fuzzy
# Executar a suíte de contrato residual v0.1.30
cargo test --test cli_v0130_agent_contract
# Executar a suíte v0.1.29 (replace fuzzy + nova superfície)
cargo test --test cli_v0129_fuzzy_replace

# Executar apenas a suíte de regressão v0.1.12
cargo test --test cli_v012_regressions
cargo test --test cli_v012_audit_regressions
cargo test --test cli_v012_batch4_regressions

# Execução suportada: a suíte completa com as features padrão
cargo test --locked

# Build slim: só prova que compila (o xtask existe só num checkout do código-fonte)
cargo xtask check-features

# Executar com output visível para debug
cargo test --test cli_v012_syntax_check -- --nocapture

# Executar o gate de cross-compile
cargo test --test cross_compile_check -- --ignored
```

### Cobertura
- Histórico (na v0.1.12): 20.19% de cobertura de linhas via `cargo tarpaulin` (935/4631 linhas cobertas), medido naquela versão e não repetido desde então
- Ficou baixa porque o tarpaulin conta apenas testes unitários, não testes de integração CLI
- A suíte de testes de integração é a métrica primária de cobertura (veja Estatísticas Atuais)

### Dependências Adicionadas
- `tree-sitter-language-pack = "1.8"` -- 305 linguagens para query/outline/syntax-check naquela versão, e a árvore hoje fixa a 1.22.1 com 372
- Todas as dependências de teste já estão na seção dev-dependencies

### ADRs e Schemas
- 29 ADRs em `docs/decisions/` (0019-0047) explicam as decisões arquiteturais por trás das features v0.1.12 a v0.1.27
- Histórico (na v0.1.27): 29 schemas JSON em `docs/schemas/`; a contagem atual é 54 (índice completo em `docs/schemas/README.md`)
- Veja [docs/decisions/README.md](decisions/README.md) para a lista completa de ADRs


## Por Que Testes Categorizados
- Cada categoria de teste valida uma camada diferente do sistema
- Testes unitários verificam lógica pura em isolamento
- Testes de integração verificam comportamento da CLI de ponta a ponta
- Testes de snapshot travam a estrutura de saída JSON
- Testes property-based descobrem edge cases que humanos perdem
- Executar todas as categorias juntas dá confiança de que atomwrite funciona corretamente


## Estatísticas Atuais
- Execução final da v0.1.38 registrada no `CHANGELOG.md`: 131 alvos de teste, 1486 testes passaram, 0 falharam e 9 ignorados (os testes `cross_compile_*`)
- Inventário medido com `fd` e `rg` na árvore de código: 131 alvos de teste de integração em `tests/` (mais `tests/common/mod.rs`, `doc_scan.rs` e `schema_check.rs`), 994 funções `#[test]` em `tests/` e 485 em `src/` (DERIVADO: 994 + 485 = 1479), mais 9 arquivos de snapshot, 4 arquivos de proptest e 2 alvos de fuzzing
- Meça de novo com `rg -c '#\[test\]' tests src` e `cargo test --lib --tests`. Um teste gerado por macro (proptest) não entra nessa contagem do `rg`, então o número de testes que rodam pode diferir do número de atributos `#[test]`
- Suítes por tema: unitários + integração + snapshot + property-based + sinal + tracing + NDJSON + regressão + cross-compile + concorrência
- Histórico (v0.1.11 e v0.1.12): 96 novos testes foram adicionados então:
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
- 3 testes do gate de cross-compile em `tests/cross_compile_check.rs` (v0.1.4; hoje são 9)
- 9 arquivos de snapshot em `tests/snapshots/`
- 2 arquivos de regressão proptest
- 2 alvos de fuzzing em `fuzz/fuzz_targets/`


## Categorias de Teste
### Testes Unitários (src/)
- Localizados dentro dos arquivos fonte via módulos `#[cfg(test)]`
- Testam funções puras sem interação com filesystem ou CLI
- `path_safety.rs` -- 5 testes para prevenção de path traversal e workspace jail
- `checksum.rs` -- correção do hashing BLAKE3

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
- `snapshot_write__error_workspace_jail_structure.snap`

### Testes de Regressão
- `tests/cli_v012_regressions.rs` -- 11 testes para regressões de v0.1.2 a v0.1.4
- `jail_suggestion_mentions_workspace_flag` -- atualizado na v0.1.4 (GAP 13): afirma que a sugestão menciona `--workspace` quando nenhum workspace é fornecido
- `gap13_jail_suggestion_when_workspace_supplied_says_inside` -- adicionado na v0.1.4 (GAP 13): afirma que a sugestão diz "inside the workspace" quando `--workspace` É fornecido
- Outras regressões cobrem scope, batch path/source aliases, fuzzy edit, search files dedup, edit multi NDJSON, clap JSON errors, json-schema, validate path jail, dead flags, mtime

### Gate de Cross-Compile (v0.1.4)
- `tests/cross_compile_check.rs` -- 9 testes (3 protegiam compilação só de Windows na v0.1.4, 6 a mais cobrem Windows arm64, Linux musl e aarch64, Apple aarch64 e x64)
- Eles NÃO foram validados na v0.1.37: o host da auditoria não tem os linkers estrangeiros (por exemplo `x86_64-w64-mingw32-gcc`)
- Os 9 testes são `cross_compile_windows_gnu_x64_succeeds`, `cross_compile_windows_gnu_i686_succeeds`, `cross_compile_windows_msvc_succeeds`, `cross_compile_windows_arm64_msvc_succeeds`, `cross_compile_linux_musl_x64_succeeds`, `cross_compile_linux_aarch64_gnu_succeeds`, `cross_compile_linux_aarch64_musl_succeeds`, `cross_compile_apple_aarch64_succeeds` e `cross_compile_apple_x64_succeeds`
- `cross_compile_windows_gnu_x64_succeeds` -- afirma que `cargo check --target x86_64-pc-windows-gnu` sucede e não emite E0433, E0308, ou E0507
- `cross_compile_windows_gnu_i686_succeeds` -- afirma o mesmo para `i686-pc-windows-gnu` (Windows 32-bit)
- `cross_compile_windows_msvc_succeeds` -- afirma o mesmo para `x86_64-pc-windows-msvc` (toolchain Microsoft Visual C++)
- Todos os testes são `#[ignore]` por padrão para pular em hosts sem os targets Windows instalados
- Execute com `cargo test --test cross_compile_check -- --ignored`
- Pula graciosamente quando o linker necessário (lib.exe para MSVC, i686-w64-mingw32-gcc para 32-bit) está ausente
- Obrigatório antes de qualquer release que toque código `#[cfg(windows)]` (veja `docs/INSTALL.md` e `docs/CROSS_PLATFORM.md`)

### Testes Property-Based (proptest)
- Localizados em `tests/proptest_checksum.rs` -- 2 testes
- Localizados em `tests/proptest_backup.rs` -- 2 testes
- Usam a crate `proptest` para gerar inputs aleatórios
- Verificam que checksums BLAKE3 são determinísticos para qualquer input
- Verificam que criação de backup funciona para conteúdos arbitrários de arquivo
- Arquivos de regressão armazenados ao lado dos arquivos de teste

### Testes de Sinal
- Localizados em `tests/signal_test.rs` -- 5 testes
- `sigpipe_handler_ends_the_run_with_exit_141` prova que um pipe fechado encerra a execução com exit 141 (o SIGPIPE tem handler cooperativo, não `SIG_DFL`)
- `sigint_during_search_exits_130` e `sigterm_during_search_exits_143` provam os exit codes dos sinais
- `batch_interrupted_by_signal` e `shutdown_message_on_stderr` provam a limpeza e a mensagem de shutdown
- Verificam comportamento de shutdown gracioso na recepção de sinais

### Testes de Validação NDJSON
- Localizados em `tests/ndjson_valid_test.rs` -- 21 testes (contados com `rg -c '#\[test\]'`)
- 19 testes validam a saída NDJSON de 19 dos 47 comandos: `search`, `hash`, `list`, `diff`, `write`, `read`, `edit`, `replace`, `count`, `backup`, `copy`, `move`, `delete`, `batch`, `calc`, `regex`, `rollback` (caso não encontrado), `apply` e `extract`
- 2 testes de interoperabilidade `jaq` (`search` e `read`) verificam parsing de JSON via pipe, e são pulados quando o `jaq` não está no `PATH`

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


## Perfis de teste local
### Rápido (Desenvolvimento)
- Execute `cargo test` com configurações padrão
- Pule testes property longos com `PROPTEST_CASES=10`
- Adequado para hooks de pre-commit
- Tempo total: estimativa antiga de menos de 30 segundos, não medida na v0.1.37

### Slim (só verificação de compilação)
- `cargo test --no-default-features --features core` compila mas NÃO fica verde: 44 testes em 11 alvos (`cli_scope`, `cli_transform`, `cli_query`, `cli_outline`, `cli_concurrency`, `cli_e2e_v0135` e outros) exigem a feature `ast` e não têm `#[cfg(feature = "ast")]` (`gaps.md` W19, `CHANGELOG.md` Notes)
- Use `cargo xtask check-features` para provar que o build slim compila: ele roda `cargo check --locked` nos conjuntos `core` e `core,watch` e para na primeira falha
- Rode a suíte de testes com as features padrão

### Completo (pré-release local v0.1.36+)

Gate local obrigatório de release (só gates locais):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
cargo test --tests
cargo test --test cli_v0136_gaps
```

### Completo (pré-release local)
- Execute `cargo test` com todas as features
- Execute `cargo clippy -- -D warnings` para linting
- Execute `cargo fmt -- --check` para formatação
- Execute `cargo insta test` para verificação de snapshots
- Execute testes property com contagem padrão de cases
- Adequado para validação de pull request
- Tempo total: estimativa antiga de menos de 2 minutos, não medida na v0.1.37

### Release (Pré-Publicação)
- Execute `cargo test --release` para testes com build otimizado
- Execute `cargo test --test proptest_checksum` com `PROPTEST_CASES=1000`
- Execute `cargo test --test proptest_backup` com `PROPTEST_CASES=1000`
- Verifique que todos os snapshots estão atualizados com `cargo insta test`

### Performance (somente local)
Rules Rust eficiência + latência: medir antes de otimizar; nunca comparar
debug com release; preferir percentis de cauda (P50/P99/P999/P9999) à média.

```bash
# Microbenchmarks Criterion (hash + read/mmap). Profile herda release (LTO fat, CGU=1).
# Cada bench também imprime `latency_hist … P50=…ns P99=…ns P999=…ns P9999=…ns` no stderr.
cargo bench --bench hash_benchmark
cargo bench --bench read_benchmark

# Binário opcional focado em tamanho (mesmo LTO/strip do release, opt-level = "z")
cargo build --profile min-size

# Binário com CPU do host só para profiling local (não publicar):
RUSTFLAGS='-C target-cpu=native' cargo build --release

# Profile de hot path (instale samply ou cargo-flamegraph à parte):
# samply record ./target/release/atomwrite search PATTERN --path .
# cargo flamegraph --bin atomwrite -- search PATTERN --path .
```

Notas de orçamento de latência (CLI one-shot, não HFT):

| Path | Dominado por | Notas |
|------|----------------|-------|
| `hash` multi-arquivo | BLAKE3 + FS | Digests em paralelo (rayon), NDJSON ordenado |
| `hash`/`read` arquivo grande | mmap + page cache | `posix_fadvise(SEQUENTIAL)` no Linux |
| stdout NDJSON | serialização + flush | Flush por linha é intencional (TTFB do agente) |
| PGO/BOLT / isolcpus / mlockall | — | N/A para CLI multi-host de vida curta |

Knobs de release em `Cargo.toml` (`[profile.release]`, `[profile.bench]`,
`[profile.min-size]`) e linker mold em `.cargo/config.toml` (Linux x86_64).
Sem gates de benchmark remotos: os benchmarks rodam só na máquina local (política do projeto).


## Logging e knobs do harness de teste
- Logging do binário é somente CLI + XDG (v0.1.35): use `-v`/`-vv`/`-vvv` e `-q`/`-qq`; tee opcional de arquivo sob o state XDG `…/atomwrite/logs` via `directories::ProjectDirs` (sem knobs de produto `ATOMWRITE_*` ou `RUST_LOG` no binário).
- Entrypoint de tracing: `init_tracing` (diagnósticos locais apenas — sem telemetria de produto).
- Apenas harness Cargo/teste (não config de produto): `PROPTEST_CASES`, `INSTA_UPDATE`, `RUST_TEST_THREADS`, `CARGO_TARGET_DIR`.
- Não documente nem dependa de variáveis de ambiente de produto para logging, locale, workspace, backup ou home override — removidas na v0.1.35 (use só CLI + XDG).
- O `doctor` não lê variável de ambiente: a detecção do host usa só sondas de arquivo; isso não configura knobs de produto.
- O helper `common::hermetic` define `HOME`, `XDG_*`, `LANG` e `LC_ALL` só no processo filho, para os testes; não é uma configuração de produto.


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

### Download de Parsers nos Testes
- Os testes de `outline`, `query` e `--syntax-check` precisam do pacote de parsers tree-sitter (23.031.807 bytes em Linux x86_64, o que dá 21,97 MiB) no `XDG_CACHE_HOME` real; a primeira execução o baixa, então rode uma vez com rede ou encha o cache antes
- Os testes nunca leem o `HOME`, a config nem a preferência de locale reais: o helper `common::hermetic` os isola, e só o cache é compartilhado

### Um Teste Depende da Máquina
- Se um teste passa numa máquina e falha em outra, confira se ele monta o comando por `common::hermetic(...)` e não lê config de desenvolvedor
- Um teste que define a própria config deve redirecionar `HOME` e `XDG_*` depois de chamar `hermetic`

### Testes Slim Falham
- `cargo test --no-default-features --features core` deve falhar 44 testes em 11 alvos que exigem a feature `ast`; rode a suíte com as features padrão (veja Perfis de teste local)

### Gate de Cross-Compile Falha
- Confirme que o target Windows está instalado: `rustup target list --installed`
- Instale targets ausentes: `rustup target add x86_64-pc-windows-gnu i686-pc-windows-gnu x86_64-pc-windows-msvc`
- No Linux, instale o toolchain necessário: `mingw64-gcc` (Fedora) ou `mingw-w64` (Ubuntu)
- Para Windows 32-bit: instale `mingw32-gcc` separadamente
- Para MSVC: instale Visual Studio 2019+ Build Tools com workload C++ (lib.exe deve estar no PATH)
- Reexecute após corrigir: `cargo test --test cross_compile_check -- --ignored`
- O gate reporta linker ausente com snippet de stderr claro como "lib.exe" ou "i686-w64-mingw32-gcc"; case essa string com o toolchain ausente


## v0.1.20 — Novidades

Esta release introduz uma nova camada de segurança chamada intention guards e renomeia a flag global `--lang` para `--locale` para desambiguar do seletor tree-sitter `--lang` usado por `scope` e `transform`.

### Intention Guards (5 flags OPT-IN)
- `--require-backup` — exige `--backup`: aborta quando falta e o arquivo alvo existe (não recebe valor)
- `--confirm` — flag legada de awareness no `write` (histórico: Y/N interativo em sobrescrita grande). Substituído na v0.1.35: sobrescrita grande é default-deny one-shot via `--ack-overwrite` (sem Y/N); `delete --confirm` é rejeitado (use `--plan`)
- `--auto-rotate` — força um backup de rotação automática para arquivos modificados há pouco (idade de `[write].auto_rotate_max_age_secs`, padrão 24 h; não recebe valor)
- `--risk-threshold <PERCENT>` — limite de variação de tamanho, em percentual, para o aviso de risco (padrão 255 = desligado; por exemplo `--risk-threshold 50`)
- `--locale <en|pt-BR>` — renomeado de `--lang` para desambiguar do `--lang` tree-sitter

### Outras Adições
- `count --by-size` — lista os maiores arquivos da árvore com tamanhos e contagem de linhas
- `read --format raw|ndjson` — seleciona entre saída byte-stream e envelope NDJSON estruturado (uma grafia antiga com `--mode` nunca existiu)
- `search --no-begin-end` — desabilita a decoração implícita de âncoras `^` e `$` na saída regex
- `write --preserve-timestamps` — preserva o mtime do arquivo fonte ao sobrescrever
- `scope --lang rust` — alias explícito de `scope --language`; o `transform` aceita só `-l`/`--language`

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

# Ou via ruplacer (só simula até você acrescentar --go)
ruplacer -- '--lang' '--locale' .
```


## Toda flag local, por comando
- Esta lista nomeia toda flag local de cada comando, e as flags globais ficam em `atomwrite --help`.
- `agent-surface`: `--format`
- `apply`: `--ack-overwrite`, `--all`, `--allow-shrink`, `--backup`, `--dry-run`, `--format`, `--keep-backup`, `--no-backup`, `--no-syntax-check`, `--retention`, `--show`, `--strict-atomic`
- `backup`: `--dry-run`, `--output-dir`, `--retention`
- `batch`: `--backup`, `--batch-size`, `--dry-run`, `--file`, `--input-schema`, `--keep-backup`, `--no-backup`, `--retention`, `--strict-atomic`, `--transaction`
- `calc`: `--stdin`
- `case`: `--backup`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`, `--subvert`, `--to`
- `codemod`: `--backup`, `--dry-run`, `--keep-backup`, `--no-backup`, `--retention`, `--rules`, `--strict-atomic`
- `commands`: `--brief`, `--include-globals`
- `completions`: `--install`
- `config`: não tem flag local própria, só o conjunto global
- `copy`: `--backup`, `--dry-run`, `--force`, `--keep-backup`, `--no-backup`, `--no-reflink`, `--preserve-xattr`, `--preserve`, `--recursive`, `--retention`, `--strict-atomic`
- `count`: `--by-extension`, `--by-size`, `--exclude`, `--include`, `--top`
- `del`: `--backup`, `--dry-run`, `--force-missing`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`
- `delete`: `--backup`, `--confirm`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--no-backup`, `--older-than`, `--plan`, `--recursive`, `--retention`, `--yes`
- `diff`: `--algorithm`, `--context`, `--stat`, `--unified`
- `doctor`: `--strict`
- `edit`: `--after-line`, `--after-match`, `--allow-sequential-drift`, `--backup`, `--before-line`, `--before-match`, `--between`, `--delete-range`, `--dry-run`, `--expect-checksum`, `--fuzzy-threshold`, `--fuzzy`, `--keep-backup`, `--line-ending`, `--multi`, `--new-file`, `--new`, `--no-backup`, `--old-file`, `--old`, `--partial`, `--preserve-timestamps`, `--range`, `--replace-all`, `--retention`, `--strict-atomic`, `--wal-policy`
- `edit-loop`: `--allow-sequential-drift`, `--backup`, `--keep-backup`, `--line-ending`, `--no-backup`, `--partial`, `--retention`, `--strict-atomic`, `--syntax-check`
- `extract`: `--delimiter`, `--field`, `--stdin`
- `extract-artifact`: `--max-sample-rows`, `--max-text-chars`
- `get`: não tem flag local própria, só o conjunto global
- `hash`: `--exclude`, `--recursive`, `--stdin`, `--verify`
- `list`: `--all`, `--count-by-ext`, `--depth`, `--exclude`, `--format`, `--include`, `--long`
- `locale`: `--clear`, `--set`
- `move`: `--backup`, `--dry-run`, `--force`, `--keep-backup`, `--no-backup`, `--preserve-hardlinks`, `--retention`
- `outline`: `--format`, `--kind`, `--language`, `--positions`
- `prune-backups`: `--dry-run`, `--max-age-secs`, `--max-count`, `--recursive`
- `query`: `--kinds`, `--language`, `--positions`, `--query`, `--tree`
- `read`: `--all`, `--bytes`, `--context`, `--crop`, `--dpi`, `--enclosing`, `--force`, `--format`, `--grep`, `--head`, `--high-res`, `--line`, `--lines`, `--max-tokens`, `--no-comments`, `--number`, `--ocr`, `--page`, `--pages`, `--stat`, `--symbol`, `--tail`, `--verify-checksum`
- `recipe`: não tem flag local própria, só o conjunto global
- `regex`: `--case-insensitive`, `--digits`, `--no-anchors`, `--repetitions`, `--spaces`, `--stdin`, `--words`
- `replace`: `--backup`, `--dry-run`, `--exclude`, `--expect-checksum`, `--fuzzy-threshold`, `--fuzzy`, `--include`, `--keep-backup`, `--literal`, `--max-replacements`, `--no-backup`, `--preserve-case`, `--preserve-timestamps`, `--preview`, `--progress-every`, `--regex`, `--retention`, `--strict-atomic`, `--word`
- `rollback`: `--backup`, `--dry-run`, `--keep-backup`, `--latest`, `--no-backup`, `--retention`, `--strict-atomic`, `--timestamp`, `--verify`
- `scope`: `--action`, `--backup`, `--delete`, `--dry-run`, `--exclude`, `--include`, `--keep-backup`, `--language`, `--no-backup`, `--pattern`, `--query`, `--replace-with`, `--retention`, `--strict-atomic`
- `scratch`: não tem flag local própria, só o conjunto global
- `search`: `--binary`, `--case-insensitive`, `--context`, `--count`, `--exclude`, `--files`, `--fixed`, `--format`, `--include-fifo`, `--include`, `--invert`, `--limit`, `--max-columns`, `--max-count`, `--max-filesize`, `--multiline`, `--no-begin-end`, `--offset`, `--pcre2`, `--regex`, `--smart-case`, `--sort`, `--target`, `--word`
- `semantic-merge`: `--backup`, `--base`, `--dry-run`, `--expect-checksum`, `--fail-on-conflict`, `--keep-backup`, `--no-backup`, `--no-conflict-markers`, `--ours`, `--output`, `--retention`, `--strict-atomic`, `--theirs`, `--write-conflict-markers`
- `semantic-search`: `--include-backups`, `--index-dir`, `--k`, `--min-score`
- `set`: `--backup`, `--dry-run`, `--keep-backup`, `--no-backup`, `--preserve-timestamps`, `--retention`, `--strict-atomic`
- `sparse`: não tem flag local própria, só o conjunto global
- `stat`: `--all`, `--bytes`, `--context`, `--crop`, `--dpi`, `--enclosing`, `--force`, `--format`, `--grep`, `--head`, `--high-res`, `--line`, `--lines`, `--max-tokens`, `--no-comments`, `--number`, `--ocr`, `--page`, `--pages`, `--stat`, `--symbol`, `--tail`, `--verify-checksum`
- `transform`: `--backup`, `--dry-run`, `--exclude`, `--include`, `--inline-rules`, `--keep-backup`, `--language`, `--no-backup`, `--pattern`, `--retention`, `--rewrite`, `--rules`, `--strict-atomic`, `--verify-parse`
- `verify`: não tem flag local própria, só o conjunto global
- `wal-heal`: `--dry-run`, `--max-duration-ms`, `--threshold-secs`
- `wal-stats`: `--dry-run`
- `watch`: `--checksum`, `--debounce-ms`, `--gitignore`, `--idle-exit-ms`, `--max-events`
- `write`: `--ack-overwrite`, `--allow-empty-stdin`, `--allow-shrink`, `--append`, `--auto-rotate`, `--backup`, `--confirm`, `--dry-run`, `--durability`, `--expect-checksum`, `--keep-backup`, `--line-ending`, `--max-size`, `--no-backup`, `--no-checksum-when-empty`, `--prepend`, `--preserve-timestamps`, `--require-backup`, `--require-large-ack`, `--retention`, `--risk-threshold`, `--strict-atomic`, `--syntax-check`, `--wal-policy`
