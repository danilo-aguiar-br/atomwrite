# atomwrite -- Contrato de Integração com Agentes


[Read in English](AGENTS.md)


## O Que Há de Novo na v0.1.38 (2026-11-04)
- O envelope de falha carrega `ok`, `code` e `exit`, e o `error: true` saiu porque duplicava `ok: false` mais `code`, custava 13 bytes em toda falha e nenhum consumidor decidia por ele sozinho; quem o lia passa a ler `.ok == false`
- O `read --format raw` para em 24.000 bytes e imprime uma linha no stderr com o `--lines` que lê o resto, e o `--all` desliga o teto
- O `read --format raw` de alvo único honra `--max-filesize` e sai 65 `FILE_TOO_LARGE` em vez de transmitir um arquivo acima do teto
- Uma imagem chega como uma linha `data:`, o `--high-res` sobe a faixa para 2.576 pixels e 4.784 tokens visuais, e uma imagem já enviada nesta sessão responde com uma linha curta de token, a menos que `--force` seja dado
- `read doc.pdf` devolve o texto de cada página, `--pages` seleciona janelas, `needs_vision` nomeia a página sem texto, e `--ocr` reconhece caracteres numa imagem ou numa página renderizada em Rust puro
- O `search --format text` escreve uma linha `caminho:linha:coluna:texto` por match, e uma árvore em que todo arquivo era binário sai 89 `ALL_SKIPPED_BINARY` em vez de `NO_MATCHES`
- O `apply` casa linhas inteiras, aplica vários arquivos numa invocação, publica as linhas mudadas com `--show` e ordena os alvos antes de tomar os locks
- O lock mira o inode com `std::fs::File::try_lock`, e o `delete` toma o mesmo lock antes de calcular o hash e antes do unlink, então uma exclusão e uma gravação no mesmo alvo não se entrelaçam mais
- O `delete -r <dir>` domina a árvore nomeada inteira, incluindo entradas ocultas, ignoradas, links, fifo, socket e dispositivo, e um link nunca é seguido
- O `target_resolved` é sempre uma lista na ordem de invocação, porque um caminho pode conter a vírgula que antes unia vários alvos
- O `[defaults].backup = true` alcança o `delete` e o `batch delete`, e o `[delete].backup` vence sobre ele só no `delete`
- O `output.profile = "lean"` tira os digests, as contagens de bytes e o tempo de um registro de mutação bem-sucedida, o que levou um `apply` de três linhas de 433 bytes para 196
- Um arquivo cuja assinatura de imagem está corrompida falha no `read` com exit 65 `INVALID_INPUT` em vez de devolver os bytes como texto
- O `outline` lê um título Markdown como uma `section` que vai até o fim da própria seção, e o `read --symbol Part` devolve essa seção inteira com as subseções
- O `config` lista 43 chaves, e o `atomwrite commands --brief` imprime uma linha por subcomando em 5.020 bytes contra 67.492 da árvore completa
- Fixe os agentes em `^0.1.38`. 47 subcomandos (SSOT)


## O Que Há de Novo na v0.1.37 (2026-10-31)
- SSOT: 47 subcomandos (`CANONICAL_SUBCOMMAND_COUNT` / `atomwrite agent-surface`). O novo é `config`
- `config`: `config path|list|get|set|unset|doctor`; `--scope global|workspace` é obrigatório em `path`, `set` e `unset` (não há padrão); `list` e `get` mostram o valor efetivo com a origem (`default`, `global`, `workspace` ou `explicit`); 43 chaves, todas na seção Configuração abaixo
- Um evento terminal: o resumo que falha em `search`, `replace`, `transform`, `scope`, `edit-loop`, `semantic-merge` ou `recipe` carrega `ok:false`, `code`, `exit` e o detalhe do erro na mesma linha; nenhum segundo envelope de erro vem depois. Pipe quebrado e veredito `hash_verify` já publicado são as únicas exceções
- `MATCH_FAILED`: um par do `edit-loop` ou do `edit` multi-par sem match sai 65 com o código `MATCH_FAILED` e lê `best_candidate` como o `edit`
- `files_failed` / `errors[]`: `search`, `transform`, `replace`, `scope`, `count`, `list`, `sparse`, `semantic-search` e `case` listam os arquivos que não puderam ser lidos ou gravados (cada entrada tem `path`, `code` e `message`, `path` é o marcador `<walk>` quando a varredura falhou sem nomear um arquivo próprio; no máximo 50, `files_failed` guarda a contagem total, e `partial: true` acompanha as duas chaves sempre que um arquivo falhou); raiz nomeada na invocação que não existe reprova a execução com exit 4 mesmo tendo casado, `search`, `replace`, `transform`, `scope`, `case`, `sparse`, `semantic-search` e `backup` mantêm exit 0 quando responderam e algum arquivo falhou, enquanto `count`, `list`, `hash`, `copy`, `delete` e `prune-backups` reprovam a execução sobre uma árvore parcial, porque a resposta desses seis é um total sobre a árvore inteira, `--format text` escreve cada falha no stderr com o caminho e o motivo, e execução sem match e com falha sai com a primeira falha
- Schemas: um nome por verbo passa a servir o stream inteiro: `recipe`, `codemod`, `batch`, `search`, `replace`, `list`, `backup`, `semantic-search`, `sparse`, `semantic-merge`, `case` e `watch` respondem cada um com a união de todo registro que escrevem, o evento terminal e o envelope de erro; `recipe` marca o registro de filho reemitido com `step` e `codemod` com `rule_id`; os quatro nomes `*-summary` deixaram de existir; `count` serve o schema de contagem total; `--print-schema` lista 55 nomes
- Teto do fuzzy: arquivo acima de `[fuzzy].max_content_bytes` (256 MiB) falha no fuzzy com exit 65 (`INVALID_INPUT`); `--fuzzy off` segue só exato; a estratégia `escape_normalized` é pulada acima de 32 MiB
- Cancelamento: todo verbo que varre encerra o canal de dado no envelope `cancelled`, cujo campo `deadline_exceeded` separa prazo de `--timeout-secs` estourado (exit 124) de sinal (exit 130, 143 ou 129 para SIGHUP) sem ler o `reason`; o registro sai sem moldagem, então `--filter`, `--select` e `--limit` não o descartam
- Envelope de sucesso: todo evento de sucesso carrega `ok: true`; verbos com efeito colateral trazem também `target_resolved`, sempre array mesmo quando um caminho só foi resolvido, e `target_source` (`argv`, ou `stdin` quando o patch ou o manifesto o nomeia), e `search`, `read`, `count`, `list`, `hash` e `diff` trazem os dois também (`workspace` só no padrão `.` de `count` e `list`, que é a raiz ambiente); `suggestion_lang` nomeia o idioma da `suggestion`; valores `null` de topo saíram do stdout
- Execução hospedada por agente: `--non-interactive` (nunca pergunta, nunca lê confirmação) e `--confirm-side-effect` entram nas flags globais, junto com as flags de formato de stream `--sort-by`, `--dedupe`, `--aggregate`, `--join`, `--truncate-content`, `--redact`, `--normalize-encoding`, `--diff-against` e `--max-output-bytes`
- QUEBRA -- `--max-output-bytes`: exige `--allow-truncate` (cada flag é rejeitada sem a outra, exit 2) e corta todo valor string, `checksum` incluído
- QUEBRA -- `apply`: um diff unificado é comparado com o arquivo (contexto e linhas removidas, deslocamento até 64 linhas); divergência sai 65 `MATCH_FAILED` com `best_candidate` e não grava nada, onde antes gravava errado com exit 0; hunk só de inserção insere DEPOIS da linha nomeada; lote SEARCH/REPLACE com bloco ausente falha inteiro, e bloco que casa em dois lugares é `MATCH_AMBIGUOUS`
- QUEBRA -- `semantic-merge`: com `--fail-on-conflict` um merge com conflitos sai 84 (`MERGE_CONFLICT`), não 65
- QUEBRA -- `STATE_DRIFT`: verbos de leitura-modificação-escrita (`edit`, `edit-loop`, `set`, `del`, `config set`, `apply`, `write --append`) serializam no lock advisory automático; `case`, `transform`, `scope` e `codemod` falham um arquivo que mudou desde a leitura com `STATE_DRIFT` (82, retryable), então rode de novo e nunca assuma que a gravação aconteceu
- QUEBRA -- entrada inválida é 65, não 255: regex ou glob inválido em qualquer verbo (`sparse` e `hash -r` inclusive), padrão AST, regra YAML, `query -Q`, `read --grep`, `locale --set` ou operação de `edit --multi` sai 65 `INVALID_INPUT`; 255 `INTERNAL_ERROR` é só falha interna de verdade
- QUEBRA -- `sparse list`: `--include` e `--exclude` são aplicados como globs de verdade
- QUEBRA -- `--hidden` em `hash`, `copy -r` e `semantic-search`: os três verbos escondem dotfile no padrão e só o visitam sob `--hidden`, então uma caminhada que nunca passou a flag devolve o inventário menor; um caminho nomeado na invocação continua sendo lido sem a flag, e `sparse list` e `sparse outline` aceitam o próprio `--all` para a mesma visita
- QUEBRA -- empate do fuzzy: um bloco ausente que só lembra várias linhas é `MATCH_FAILED` com `best_candidate`; `MATCH_AMBIGUOUS` exige dois ou mais candidatos que passam o dual-gate
- Outras mudanças de saída: `doctor --strict` com check reprovado ou com aviso sai 1 (`DOCTOR_CHECKS_FAILED`); `write` interrompido por sinal ou prazo sai 143, 130 ou 124, não 255; o handler de SIGPIPE é cooperativo e termina com exit 141 depois de limpar. Uma escrita que falha porque o leitor fechou sai 141 (`BROKEN_PIPE`) sem panic e sem relatório de falha: `completions`, `--print-schema`, `--help` e todo verbo. Saída muito curta que cabe no buffer do pipe (por exemplo `--version | head -1`) pode terminar antes de o leitor fechar e então sai 0
- Build: `watch` não está no conjunto de features padrão; o binário padrão responde `watch` com exit 78 (`CONFIG_INVALID`), e um build com `--features watch` o executa. Os parsers tree-sitter são baixados em runtime na primeira vez que uma linguagem é necessária (por exemplo `outline`, `query` e `--syntax-check`); o download respeita as configurações de proxy do host e precisa de rede uma vez
- Exit codes para ramificar: 65 (leia `best_candidate`), 78 (config ou feature ausente no build slim), 82 (drift), 83 (lock), 84 (conflito de merge), 87 (`edit-loop --partial`), 124 (prazo), 129 (SIGHUP), 141 (pipe quebrado), 143 (SIGTERM); a tabela completa está em Exit Codes
- Pin de agentes em `^0.1.37`


## O Que Há de Novo na v0.1.36 (2026-07-31)
- Contrato de agente residual (GAP-2026-019…033 + CAPA): jail de content-source, `scratch init`, shape global de stdout
- Shape (global): `--select`/`--fields` (CSV), `--filter KEY=VAL` (AND repetível, só top-level), `--limit N` (só long — sem `-n` global; `replace` usa `-n`)
- Erros always-emit: falhas sempre imprimem NDJSON com `ok:false`; `--filter` nunca dropa; `--select` pode afinar chaves; `--limit` só em sucesso
- Histórico (v0.1.36): 46 subcomandos (`CANONICAL_SUBCOMMAND_COUNT` / `atomwrite agent-surface`). Não documentar 41/42 como atual
- `extract`: campos posicionais ou `--field` local (repetível). NUNCA `extract --select` — o `--select` global só projeta a linha emitida (nome local colide com clap)
- Pin `^0.1.37`. Config CLI + XDG / `.atomwrite.toml`. Suíte: `cli_v0136_gaps`. ADR-0055


## O Que Há de Novo na v0.1.35 (2026-07-19)
- Gaps residuais A-* fechados (gaps.md daquela versão, §20): suite verde sem `delete --yes`; sobrescrita grande default-deny (`--ack-overwrite`); `watch` sempre emite `watch_summary`; markers de conflito do semantic-merge default ON; monólitos fatiados com `include!` (SRP)
- Write em arquivos grandes: alvos existentes acima de XDG `[write].confirm_large_bytes` (padrão 100 KiB) exigem `--ack-overwrite` (agente one-shot; sem Y/N interativo). `--confirm` é flag legada de awareness; `--require-large-ack` é independente (sem colisão de alias)
- Delete: use `--plan` ou `--dry-run` para plan-only; `delete --confirm` e `delete --yes`/`-y` são rejeitados (fail-closed). Execute de novo sem essas flags para apagar
- Watch: NDJSON final `type:watch_summary` em todo caminho de saída; idle padrão 500 ms (`--idle-exit-ms` / XDG `[watch].idle_exit_ms`); debounce via CLI ou XDG `[watch].debounce_ms`
- Semantic-merge: markers de conflito gravados por padrão; opt-out `--no-conflict-markers`
- DoD local: `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`, `cargo check --target x86_64-pc-windows-gnu`, `cargo install --path . --force`
- Suite de contrato: `cargo test --test cli_e2e_v0135`
- Pin histórico `^0.1.35` (substituído por `^0.1.36`). Configuração somente CLI + XDG / `.atomwrite.toml` (sem knobs `ATOMWRITE_*` de produto; sem telemetria de produto)


## O Que Há de Novo na v0.1.34
- Publicação docs-complete do runtime one-shot da v0.1.33 (mesmo comportamento do binário)
- Global `--timeout-secs` (alias `--timeout`) padrão 120 (era 0); `0` desabilita; prazo esgotado → exit 124
- Multi-apply fuzzy é one-pass E→D no conteúdo original (`apply_fuzzy_one_pass`); nunca reescaneia texto inserido
- Máximo de applies fuzzy padrão = 1 se `--max-replacements` omitido; teto rígido 10_000
- Se o replacement contém o pattern (string fixa), força apply único (expansão segura de seção)
- Cancel cooperativo consultado no meio da cascata fuzzy; caps: pattern 64 KiB, lev 8192 chars, max windows 4096, growth max(4×,+16 MiB)
- Caps fuzzy (constantes em `src/constants.rs`): `FUZZY_MAX_PATTERN_BYTES` (64 KiB), `FUZZY_MAX_LEVENSHTEIN_CHARS` (8192), `FUZZY_MAX_WINDOWS` (4096), `FUZZY_MAX_BUFFER_GROWTH_FACTOR`/`FUZZY_MAX_BUFFER_GROWTH_BYTES` (max 4× / +16 MiB), teto rígido `FUZZY_HARD_MAX_REPLACEMENTS` (10_000)
- Suíte de regressão: `tests/cli_v0133_oneshot_fuzzy.rs` — hang < 2s, embeds, timeout
- Histórico (na v0.1.33): 41 subcomandos; fuzzy ainda só `auto|aggressive` (`off` rejeitado exit 65). Substituído na v0.1.35: `off` = exact-only (G-010).
- Veja [ADR-0054](decisions/0054-v0-1-34-oneshot-fuzzy-timeout.pt-BR.md) para o contrato one-shot fuzzy + timeout (exit 124 vs 143)


## O Que Havia de Novo na v0.1.30
- BREAKING residual: `--fuzzy off` é rejeitado (exit 65); só `auto` e `aggressive`. Substituído na v0.1.35: Off = exact-only (G-010).
- Sucesso de edit no NDJSON inclui `match_count` e opcional `indent_adjusted`
- `platform.backup_method` é `reflink_or_copy` (nunca hardlink do arquivo vivo)
- Recipe hash exclui `*.bak.*`; sparse outline emite itens AST reais


## O Que Havia de Novo na v0.1.29
- BREAKING: o padrão de `replace` é `fuzzy=auto` após zero matches exatos; na 0.1.30 exact-only via `--fuzzy off` é rejeitado. Substituído na v0.1.35: Off = exact-only (G-010).
- 8 subcomandos novos para agentes: `recipe`, `sparse`, `semantic-merge`, `agent-surface`, `watch`, `codemod`, `semantic-search`, `stat`
- Cascata fuzzy compartilhada no `replace` (`--fuzzy`, `--fuzzy-threshold`); erros de match podem incluir `best_candidate`
- `write --durability full|fast|auto`; NDJSON `platform.durability`, `platform.rename_method`, `platform.backup_method`
- Linux `renameat2` com fallback para rename
- Features Cargo: `core` / `ast` / `lang-*` / `watch` / `semantic` / `full` (core slim ~7,7 MiB; meta PRD 5–8 MiB = só core)
- `agent-surface` é o manifesto de tools da CLI — integração por subprocesso, CLI com saída NDJSON
- `recipe` executa dispatch multi-passo real (search→replace→hash)
- Cancel cooperativo: SIGINT/SIGTERM aborta stdin e trabalho em chunks (o envelope `cancelled` com `deadline_exceeded` chegou na v0.1.37, veja acima)
- `semantic-search` é ranking offline por tokens/índice com foundation Jaccard (não embeddings OpenRouter)
- Histórico (na v0.1.29): 41 subcomandos no total, 38 schemas JSON, 683 testes listados


## O Que Há de Novo na v0.1.28
- BREAKING: `delete` agora cria backup por padrão (era opt-in) — passe `--no-backup` para desabilitar; `--keep-backup` no `delete` é redundante (backups de deleção são sempre preservados e nunca auto-removidos) e agora expõe campo `warnings` no envelope NDJSON em vez de um no-op silencioso
- BREAKING: `move`/`copy` sobrescrevendo destino existente agora exigem `--force` OU `--backup` explícito
- Struct `BackupOpts` unificada (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) flattened via `#[command(flatten)]` em todos os 15 subcomandos que mutam: `write`, `edit`, `edit-loop`, `replace`, `transform`, `scope`, `apply`, `set`, `del`, `case`, `batch`, `delete`, `move`, `copy`, `rollback` (GAP-CLI-SURFACE-DRIFT, ADR-0048)
- `replace --retention` e `delete --no-backup` agora parseiam com sucesso — antes retornavam exit 2 `ARGUMENT_PARSE_ERROR` por declarações de flag divergentes por struct
- `--backup`/`--no-backup` agora são mutuamente exclusivos via `conflicts_with` (exit 2) em vez da última flag vencer silenciosamente
- `rollback` mantém seu snapshot de segurança pré-rollback opt-in via `--backup` explícito — exceção documentada ao contrato unificado default-true
- Zero literais `retention: 5` hardcoded remanescentes; `Default` impls em `config.rs`/`atomic.rs` leem `constants::DEFAULT_BACKUP_RETENTION`
- Chaves `[defaults]` `backup`/`retention` do `.atomwrite.toml` agora são efetivas ponta a ponta em todo subcomando que muta (GAP-CONFIG-DEFAULTS-DEAD, ADR-0049) — precedência: flags CLI (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > config XDG > default embutido (`true`/`5`) — sem knobs de env de produto
- `batch --retention <N>` e `batch --backup` agora são efetivos ponta a ponta, incluindo o passo de pre-backup transacional e a operação `delete` dentro de um batch
- Nova guarda stdin-tty nos modos de `edit` que consomem stdin (`--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`, `--multi`): falha rápido com exit 65 (`INVALID_INPUT`) e `suggestion` acionável quando stdin é um terminal, em vez de bloquear indefinidamente (ADR-0050)
- Os modos de `edit` que consomem stdin agora rejeitam `--old`/`--new`/`--old-file`/`--new-file` no parse via `conflicts_with_all` (exit 2) em vez de despachar silenciosamente para `edit_by_marker`
- 3 novos ADRs: 0048 (BackupOpts unificado), 0049 (config plumbing ao vivo), 0050 (guarda stdin-tty)
- 661 testes passando (0 falhas, 3 ignorados); novos arquivos de teste: `cli_v0128_backup_matrix.rs` (12), `cli_v0128_config_defaults.rs` (6), `cli_v0128_edit_stdin_guard.rs` (3), `cli_v0128_batch_backup.rs` (6)


## O Que Há de Novo na v0.1.27
- CORREÇÃO DE SEGURANÇA (BUG-SEC-001): escape do jail do workspace via symlink-directory corrigido — `canonicalize_existing_prefix` agora resolve symlinks em componentes do caminho antes da verificação de jail. Afeta: write, read, edit, edit-loop, apply, set, del, copy, move. Exit 126 na violação.
- CORREÇÃO CRÍTICA (BUG-SCOPE-004): `scope --query comments --delete` não destrói mais código em linhas com comentários inline — `expand_to_full_line` agora preserva código antes do comentário
- `get` para chave ausente retorna exit 65 (`INVALID_INPUT`) — reverte o revert da v0.1.26 para exit 4 (BUG-GET)
- `edit-loop` endurecido: stdin vazio retorna exit 65 (BUG-001), JSON inválido emite erro NDJSON (BUG-002), `pair_results` inclui campos `old`/`new` (BUG-008)
- Queries `fn`, `struct`, `enum`, `trait`, `const`, `static`, `type-alias`, `mod`, `use`, `async-fn`, `unsafe-fn` do `scope` agora capturam variantes `pub` (BUG-SCOPE-002/003)
- `scope --query var` em Go casa declarações sem tipo explícito (BUG-SCOPE-005)
- Sugestão do `write --syntax-check` corrigida para "remove --syntax-check" (BUG-005)
- Limitações conhecidas: `test-fn` (GAP-01), `doc-comment` (GAP-02), JS `export` (GAP-03) — padrões multi-nó do ast-grep
- 631+ testes passando, 33 subcomandos, 29 ADRs


## O Que Há de Novo na v0.1.25
- 49 gaps resolvidos (GAP-071 a GAP-134) em 6 rodadas de auditoria e2e (~505 cenários)
- NOVO SUBCOMANDO: `verify <PATH> <BLAKE3>` (o hash é posicional) — verificação dedicada de checksum (33 subcomandos no total)
- NOVO: arquivo de configuração `.atomwrite.toml` — hierarquia da época: CLI > env > `.atomwrite.toml` local > XDG `~/.config/atomwrite/config.toml` > defaults (substituída na v0.1.35: não há variáveis de ambiente, veja a seção Configuração)
- NOVAS FLAGS: `delete --older-than <DURATION>` (s/m/h/d/w), `delete --plan` (plan-only; `--confirm`/`--yes` rejeitados desde v0.1.35), `replace --preserve-case` (adaptação UPPER/lower/Title), `search --pcre2`, `edit --fuzzy-threshold <FLOAT>`, `scope --action symbols|normalize`
- NOVAS FLAGS: `copy --no-reflink`, `copy --preserve-xattr`, `move --preserve-hardlinks`
- FUZZY MATCHING: Jaro-Winkler (estratégia `context_aware_jw`) adicionado na cascata de 9 estratégias; campo `diff_preview` nas respostas quando fuzzy match utilizado
- CORREÇÕES CRÍTICAS: `write --backup` não reporta mais `backup_path` fantasma (GAP-101), `set` não redireciona mais chave em scalar TOML (GAP-102), overflow de `size_delta_pct` corrigido (GAP-120)
- CORREÇÕES ALTAS: `copy --preserve` agora preserva permissões E mtime (GAP-103/133), `copy --backup`/`replace --backup` agora retêm arquivos `.bak` (GAP-104/105; superado na v0.1.37 para o `replace`, que apaga o `.bak` no sucesso a menos que se passe `--keep-backup`, conforme o ADR-0038), `write --require-backup` aplica a retenção (GAP-106), erros de I/O agora emitem envelope NDJSON (GAP-098)
- CORREÇÕES MÉDIAS: campo de saída do `hash` renomeado `value` → `checksum` (GAP-107), `list --long` datas ISO 8601 (GAP-116), `outline --positions` emite byte offsets (GAP-109), `list` em diretório inexistente retorna exit 4 (GAP-110), `get`/`del` chave ausente retorna exit 65 INVALID_INPUT (GAP-111), `batch move/copy` respeita `--force` (GAP-108), `scope --query comments` captura block comments (GAP-123)
- MUDANÇA DE COMPORTAMENTO: `case` sem `--subvert` retorna exit 65 com mensagem explicativa (GAP-127)
- MUDANÇA DE COMPORTAMENTO: `scope --query test-fn` retorna erro acionável com alternativas (GAP-134)
- Testes baseados em propriedades para fuzzy matching via proptest (GAP-086)
- Validação de conflitos de flags: `append+prepend`, `fixed+regex`, `literal+regex` rejeitados no parse (GAP-093)
- 631 testes passando (10 novos), 0 clippy warnings, 0 fmt diffs
- 3 JSON schemas atualizados (del-result, write-output, read-output)


## O Que Há de Novo na v0.1.24
- 52 bugs corrigidos (GAP-2026-019 a GAP-2026-070) em auditoria e2e abrangente
- AUDITORIA DE ERROS TIPADOS: TODOS os `anyhow::bail!()` em caminhos voltados ao usuário foram convertidos para variantes `AtomwriteError`. Todo erro agora emite JSON estruturado no stdout com o exit code correto. Acabou o exit 1 silencioso sem envelope JSON.
- Mudanças de exit code: caminhos que retornavam exit 1 agora retornam exit 4 (NotFound) ou exit 65 (InvalidInput). Agentes DEVEM atualizar seus parsers de erro.
- `delete --recursive` AGORA FUNCIONA (era no-op para diretórios)
- `hash --recursive` AGORA FUNCIONA (era aceito mas nunca percorria diretórios)
- `search --multiline` AGORA FUNCIONA (a flag não era propagada ao SearcherBuilder)
- `replace` REJEITA padrão vazio (antes destruía arquivos silenciosamente)
- `batch --transaction` reverte corretamente `move`/`copy` no rollback
- `scope`/`count`/`transform`/`diff` agora resolvem as raízes de varredura contra `--workspace`
- Valores de `get`/`set`/`del` não saem mais com aspas duplas (valor cru na resposta JSON)
- Timestamp de backup: `YYYYMMDD_HHMMSS_mmm` (resolução de milissegundos evita colisão)
- `rollback --timestamp` aceita casamento por PREFIXO (compatível com o formato antigo)
- `prune-backups --max-count` ordena por nome de arquivo (lexicográfico) em vez de mtime
- `case --subvert` mudou de `num_args=2..` guloso para `num_args=2` exato
- Campo `op` do `edit --multi` agora é opcional (inferido como "exact" quando `old`+`new` estão presentes)
- `hash --stdin` não exige mais o argumento PATHS
- `regex` removeu `allow_hyphen_values`; use `--` POSIX para exemplos que começam com hífen
- ARGUMENT_PARSE_ERROR (exit 2) ganha o campo `suggestion` context-aware (ADR-0045)
- 3 novos ADRs: 0045 (sugestão do clap), 0046 (diff resolve primeiro), 0047 (scope somente leitura)
- 621 testes passando (12 novos)


## O Que Há de Novo na v0.1.23
- GAP-2026-015 fechado — `allow_hyphen_values = true` adicionado a 12 campos CLI que aceitam texto em 8 structs; os três posicionais (padrão do `search` e do `replace`, substituto do `replace`) exigem `--` desde a 0.1.37. Valores que começam com `-` (bullets Markdown `- item`, números negativos `-5`, entradas YAML `- key: val`, conteúdo de diff `--- a/file`) agora são aceitos como dados, não interpretados como flags. Excluído: `CaseArgs.subvert` (incompatível com `num_args = 2..`). Veja ADR-0041.
- GAP-2026-016 fechado — backup agora vem habilitado por padrão em 9 structs que mutam conteúdo: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`. O backup é removido sozinho após o sucesso (padrão `keep_backup: false` inalterado). Opt-out: flag `--no-backup`. Precedência: flag CLI > `.atomwrite.toml` / XDG > padrão (true). Sem knobs de env de produto. 4 structs sem conteúdo inalteradas: `DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`. Veja ADR-0042.
- GAP-2026-017 fechado — escritas que encolhem o arquivo em mais de 50% agora são BLOQUEADAS quando `--expect-checksum` está ativo. Retorna exit 65 (`INVALID_INPUT`) com a sugestão de passar `--allow-shrink`. O `risk_assessment` (L1) passa a bloquear (não só informar) quando `--expect-checksum` está ativo e o arquivo encolhe. Sem `--expect-checksum`, o comportamento é inalterado. Veja ADR-0043.
- GAP-2026-018 fechado — novas flags `--old-file <PATH>` e `--new-file <PATH>` no `edit`. O conteúdo é lido de arquivos dentro do processo atomwrite, contornando a expansão do shell e o ARG_MAX do kernel (~131 KB). `conflicts_with` impede misturar `--old` com `--old-file` (exit 2). A guarda de mistura cruzada rejeita `--old` + `--new-file` e `--old-file` + `--new` (exit 65). `strip_file_trailing_newline()` remove uma quebra de linha final para paridade com o argv. `PairResult.source` reporta "arg" ou "file". Veja ADR-0044.
- 609+ testes passando (31 novos: 12 hyphen + 7 backup + 4 shrink + 8 old-file)
- 4 novos ADRs: 0041, 0042, 0043, 0044


## O Que Há de Novo na v0.1.22
- GAP-2026-012 Frente 3 fechado — novo sub-comando `edit-loop [PATH]` aplica N pares `{old, new}` em 1 invocação via NDJSON no stdin. Reduz 5 chamadas `edit` sequenciais (5 spawns de subprocess, 5 recapturas de checksum) para uma única escrita atômica. Suporta `--partial`, `--backup`, `--keep-backup`, `--line-ending`, `--preserve-timestamps`, `--fuzzy`, `--expect-checksum`. Veja `tests/cli_v0121_edit_loop.rs` e ADR-0039.
- GAP-2026-013 Frente 2 fechado — novo sub-comando `prune-backups [PATHS]...` oferece limpeza manual de arquivos `.bak.YYYYMMDD_HHMMSS` legados da v0.1.20 e anteriores. Flags: `--max-age-secs <SECONDS>`, `--max-count <N>`, `--dry-run` (uma chave, desligada por padrão; veja a seção do prune-backups). Reusa `cleanup_old_backups_in` de `src/atomic.rs`. Veja `tests/cli_v0121_prune_backups.rs` e ADR-0040.
- 2 novos schemas NDJSON: `edit-loop-output.schema.json` (com `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`) e `prune-backups-output.schema.json` (com `action`, `path`, `reason`, `total`, `elapsed_ms`).
- 32 subcomandos no total (adicionados `edit-loop` e `prune-backups` aos 30 anteriores).


## O Que Há de Novo na v0.1.21
- GAP-2026-012 fechado — nova flag `--allow-sequential-drift` em `edit` aceita drift de checksum entre edits sequenciais no mesmo arquivo, eliminando o `STATE_DRIFT` (exit 82) falso-positivo quando o mesmo agente é dono do arquivo entre iterações. Comportamento padrão (sem a flag) inalterado: re-capturar checksum entre edits.
- GAP-2026-013 Frente 4 fechado — `edit` e `rollback` agora expõem flags `--backup` e `--retention` para paridade com `write` e `replace`. Default `backup: false` preserva comportamento existente.
- GAP-2026-014 v2 fechado — default de backup mudou para deletar após sucesso. Anteriormente, backups acumulavam com a política `retention: u8` (padrão 5). Agora, `--backup` cria um `.bak.{timestamp}` que é removido inline no sucesso. Nova flag opt-in `--keep-backup` preserva o backup. Operações que falham sempre preservam o backup para inspeção. Veja ADR-0038 e `tests/cli_v0121_backup_keep_flag.rs`.
- Novo padrão documentado: capturar checksum no topo de cada iteração de loop via `atomwrite read --json | jaq -r '.checksum'`, depois passar para `--expect-checksum`. Elimina STATE_DRIFT para edits sequenciais pelo mesmo agente.


## O Que Há de Novo na v0.1.15 (estendido em v0.1.18)
- G117: o `edit` multi-par `--old/--new` agora roda a mesma cascata fuzzy de 9 estratégias do caminho single, por par. Envelopes de sucesso ganham `pairs_total` e `pair_results[{index, matched, strategy, similarity}]`; falhas ganham `failed_pair_index` (exit 65, arquivo intacto). Novo opt-in `--partial` aplica os pares que casam e relata os demais.
- G118: o `write` resolve o alvo contra `--workspace` ANTES de append/prepend, detecção automática de line ending e `--expect-checksum`. Com CWD divergente, drift de checksum agora retorna exit 82 (`STATE_DRIFT`) em vez de sobrescrever silenciosamente, e alvos fora do jail retornam exit 126 cedo.
- 542 testes passando; os ADRs 0031-0037 documentam o fechamento de sete gaps entre v0.1.19-v0.1.20.


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças v0.1.12 mais relevantes para agentes de IA que usam atomwrite como ferramenta. Todos os 13 gaps da auditoria PRD fechados em v0.1.11+v0.1.12 estão listados abaixo.

### Subcomandos Adicionados (v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` — escreve um valor em um caminho dotted em um arquivo TOML ou JSON, preservando comentários e ordem das chaves via `toml_edit`. Use isto em vez de reescrever o arquivo de config inteiro (economiza tokens, preserva formatação).
- `get <PATH> <KEY_PATH>` — lê um valor em um caminho dotted. NDJSON: `{"type":"get","key_path","value","found","format"}`. Use isto em vez de ler o arquivo de config inteiro.
- `del <PATH> <KEY_PATH>` — remove uma chave. Flag `--force-missing` trata chaves ausentes como no-op success. Use isto para scripts de cleanup idempotentes.
- `case <PATHS...> --subvert OLD NEW --to <style>` — renomeia identificadores em múltiplos arquivos via `heck`. Estilos: `snake`, `camel`, `pascal`, `kebab`, `screaming-snake`.
- `query <PATH> [--kinds|-Q <PATTERN>|--tree] [--positions]` — caminha um AST tree-sitter e emite nós como NDJSON. 305 linguagens via `tree-sitter-language-pack`.
- `outline <PATH> [--kind <KIND>] [--positions]` — extrai estrutura de alto nível (funções, classes, structs, enums, traits, módulos) como NDJSON.

### Flags Adicionadas (Críticas para Agentes)
- `--format raw|ndjson` em `read` — emite bytes crus para composabilidade Unix com `sed`, `awk`, `diff`, `patch`. G81.
- `--syntax-check` em `write` — invoca o parser tree-sitter para validar código. Exit 88 em erro de sintaxe. G72.
- `--max-filesize <BYTES>` em `search` — pula arquivos maiores que o limite (padrão 10 MiB). G68.
- `--max-columns <N>` em `search` — trunca matches com >N colunas (padrão 500). G68.
- `--literal` (alias `-F`) em `replace` — desabilita interpretação de regex. G66.
- `--rules <file.yaml>` e `--inline-rules <YAML>` em `transform` — multi-rule YAML para refactors em cascata. G44.
- `--batch-size <N>` em `batch` — controla pico de memória (padrão 100). G77.
- `--no-reflink` em `backup`/`copy` — desabilita CoW para filesystems sem suporte. G64.
- `--include-fifo` em `search` — volta a abrir named pipes, que o walker pula por padrão. G56.
- Renames cross-device (EXDEV) caem para copiar-e-renomear. G90. `--strict-atomic` no verbo que escreve recusa esse fallback e sai 91.
- Lock advisory de escrita via `std::fs::File::try_lock`, que embrulha o `flock` no Unix e o `LockFileEx` no Windows. G54. Desde a 0.1.37 o lock é automático, não tem flag e espera `lock.acquire_timeout_ms` (padrão 5000) antes do exit 83.

### Códigos de Erro Adicionados (5 Novos)
- 83 `LockTimeout` (G54 lock advisory via flock excedido)
- 88 `SyntaxError` (G72 `--syntax-check` via parser tree-sitter)
- 91 `ExdevFallbackDisabled` (G90: `--strict-atomic` num verbo que escreve recusa o fallback de cópia Docker/NFS; a opção de biblioteca `WriteOptions.strict_atomic` chega ao mesmo caminho)
- 92 `CopyBackBlake3Failed` (G114 escrita in-place perdeu integridade de checksum)
- 93 `OrphanJournal` (G114 sidecar WAL deixado por crash)
- Veja OBRIGATÓRIO -- Exit Codes abaixo para a tabela completa: 41 linhas sobre 33 números de saída distintos.

### Recuperação de Crash (G114)
- `atomic_write` escreve `.atomwrite.journal.<target>.atomwrite.journal.json` com entradas `Started`/`Committed`.
- `recover_orphan_journals(dir)` é consultivo (sem auto-replay, sem auto-delete).
- O agente recebe eventos `{"type":"wal_recovery","orphan_journals":[...]}` e decide.

### Gaps Fechados (13 dos Top 20 do PRD)

G39 xattr, G41 binary detect (content_inspector), G54 advisory lock, G56 FIFO skip, G58 line endings, G64 reflink CoW, G66 --literal, G68 --max-filesize, G72 syntax check, G74 --threads, G76 `diff --algorithm`, G77 --batch-size, G80 SIGPIPE, G81 --format raw, G90 EXDEV fallback, G116 fuzzy match, v14 Tier 3 (set/get/del/case/query/outline).

### Dependências Adicionadas
- `tree-sitter-language-pack = "1.8"` (305 linguagens; os parsers são baixados em runtime no primeiro uso, cerca de 19 MiB no cache, e depois carregados dinamicamente)
- `toml_edit` (preserva formatação TOML)
- `heck = "0.5"` (conversão de case)
- `reflink-copy = "0.1"` (backup CoW)
- `content_inspector = "0.2"` (detecção UTF-16)
- `xattr = "1"` (extended attributes)

### Cobertura de Testes
- 542 testes passando (461 baseline v0.1.15 + 8 G117 edge cases v0.1.18 + 2 G118 replace pre-validation v0.1.18 + 16 incrementos cross-platform/WAL/auditoria v0.1.16-v0.1.18)
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas em `docs/schemas/` (set, get, del, case, query, outline, wal-recovery)
- Veja [docs/decisions/README.md](decisions/README.md) para decisões arquiteturais


## Por Que atomwrite
- Seu agente faz dezenas de chamadas de ferramenta para ler, escrever, buscar e substituir arquivos
- Cada chamada custa tokens, latência e espaço na janela de contexto
- atomwrite substitui tudo isso com uma CLI que lida com todas as operações de arquivo
- Toda escrita é atômica: tempfile, fsync, rename, fsync-dir
- Toda saída é NDJSON: um objeto JSON por linha no stdout
- Toda resposta inclui um checksum BLAKE3
- O checksum na resposta elimina leituras de verificação


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


## Economia
### Economia de Tokens
- Cada subcomando custa ~50-200 tokens de saída
- Um batch de 100 escritas custa 1 chamada bash em vez de 100 chamadas de ferramenta
- O checksum nas respostas de escrita economiza uma leitura por escrita
- Uma sessão típica de refatoração economiza 500+ chamadas de ferramenta

### Janela de Contexto
- Saída NDJSON é compacta e estruturada
- Sem formatação verbosa para humanos para interpretar
- Agentes consomem a saída diretamente sem etapas de extração


## Soberania
- atomwrite é um binário Rust standalone com zero dependências de runtime
- Sem serviço cloud, sem API key, sem acesso à rede necessário
- Todas as operações executam localmente com latência sub-milissegundo
- O agente controla todos os aspectos das operações de arquivo
- Sem vendor lock-in a qualquer framework de agente


## Agentes Compatíveis
- Claude Code (Anthropic)
- Cursor (Anysphere)
- Windsurf (Codeium)
- Aider
- OpenAI Codex CLI
- Qualquer agente que invoque comandos bash e interprete JSON


## Quickstart

```bash
cargo install atomwrite --locked
WS="$PWD"
echo "hello" | atomwrite --workspace "$WS" write src/hello.txt
atomwrite --workspace "$WS" read src/hello.txt
atomwrite --workspace "$WS" search 'hello' src/
atomwrite --workspace "$WS" replace 'hello' 'world' src/
atomwrite calc "2 hours + 30 minutes to seconds"
```


## 47 Subcomandos (SSOT)
- A lista é o manifesto de `atomwrite agent-surface --format json`; cada entrada abaixo tem um resumo de uma linha e um exemplo que foi rodado contra o binário 0.1.38
- Os exemplos assumem que `WS` é a raiz absoluta do workspace (`WS="$PWD"`) e as fixtures do projeto sob teste
- `read` -- lê arquivos com metadados, checksum e conteúdo opcional; `--stat` pula o conteúdo, `--head N`, `--tail N`, `--lines START:END`, `--grep <REGEX>` filtra as linhas retornadas, `--verify-checksum HASH` sai 81 se não confere, `--format raw|ndjson` (`raw` emite os bytes), `--symbol NAME`, `--line N` com `--context N` e `--enclosing`, `--number`, `--no-comments`, `--bytes`, `--all`, `--crop WxH`, `--dpi N`, `--max-tokens N`, `--pages`, `--ocr`, `--high-res` e `--force`
  - Exemplo: `atomwrite --workspace "$WS" read --head 2 src/a.rs`
- `write` -- cria ou sobrescreve arquivos atomicamente via stdin; `--expect-checksum` (exit 82 em drift), `--durability full|fast|auto`, `--syntax-check` valida com tree-sitter (exit 88), `--allow-shrink` para reescrita que encolhe mais de 50%, `--append`, `--prepend`; alvos existentes grandes exigem `--ack-overwrite` (default-deny acima de `write.confirm_large_bytes`, exit 65); `--max-size BYTES`, `--wal-policy auto|always|never`, `--no-checksum-when-empty`, `--auto-rotate`, `--risk-threshold PERCENT` e `--require-large-ack` completam a superfície local
  - Exemplo: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` -- edita cirurgicamente por número de linha (`--after-line`, `--before-line`, `--range`, `--delete-range`), marcador de texto (`--after-match`, `--before-match`, `--between`) ou match exato (`--old`/`--new`, pares repetíveis, ou `--old-file`/`--new-file`); `--fuzzy auto|aggressive|off` onde `off` = exact-only (padrão `auto`), `--fuzzy-threshold`, `--replace-all`, `--multi` para multi-edit NDJSON; o sucesso NDJSON traz `match_count`, `indent_adjusted` e `checksum_after`
  - Exemplo: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `search` -- busca conteúdo de arquivos em paralelo (engine ripgrep); `-C`/`--context N`, `-m`/`--max-count N`, `--invert`, `--sort path`, `-F`/`--fixed`, `-w`/`--word`, `-i`/`--case-insensitive`, `-g`/`--include`, `--exclude`, `-P`/`--pcre2`, `-U`/`--multiline`, `--include-fifo` (abre FIFOs, que são pulados por padrão); zero matches sai 1 com `NO_MATCHES`; `--regex`, `--smart-case`, `--binary`, `--files`, `--offset N`, `--limit N`, `--format text|ndjson` e `--no-begin-end` completam a superfície
  - Exemplo: `atomwrite --workspace "$WS" search alpha src/`
- `replace` -- substitui texto em múltiplos arquivos com escritas atômicas; `--fuzzy auto|aggressive|off` onde `off` = exact-only (padrão `auto`), `--fuzzy-threshold`, `--progress-every`, `--dry-run`; multi-apply fuzzy é one-pass E→D (`apply_fuzzy_one_pass`, nunca reescaneia texto inserido); máximo de applies padrão 1 se `--max-replacements` (`-n`) omitido (teto rígido 10_000); se o replacement contém o pattern (string fixa), força apply único; falhas de match podem emitir `best_candidate`, e o `--regex` local trata o padrão como expressão regular
  - Exemplo: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `hash` -- calcula checksums BLAKE3 (leia o campo `checksum`); `--verify HASH`, `--stdin`, `-r`/`--recursive`, `--exclude`
  - Exemplo: `atomwrite --workspace "$WS" hash src/a.rs`
- `delete` -- deleta arquivos, copiando um só quando a chamada pede (`--backup` ou `[defaults].backup = true`); `--plan` para listagem plan-only, `-r` para diretórios, `--older-than`, `-g`/`--include`; `delete --confirm` / `--yes` / `-y` são rejeitados
  - Exemplo: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `count` -- conta linhas e arquivos por extensão (`--by-extension`) ou por tamanho (`--by-size --top N`)
  - Exemplo: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` -- compara dois arquivos (`--unified`, `--stat`, `-C N`, `--algorithm myers|patience|lcs`); segundo path `-` lê stdin
  - Exemplo: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `move` -- move ou renomeia arquivos atomicamente; sobrescrever um destino exige `--force` ou `--backup`
  - Exemplo: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `copy` -- copia arquivos com verificação de checksum; `--recursive`, `--preserve`, `--no-reflink`, `--preserve-xattr`; sobrescrever um destino exige `--force` ou `--backup`
  - Exemplo: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `list` -- lista a estrutura de arquivos do projeto com metadados; `--long`, `--depth N`, `--count-by-ext`, `--all`, `-g`/`--include`, `--exclude`
  - Exemplo: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` -- extrai campos de NDJSON no stdin ou colunas de texto; campos posicionais ou o `--field` local (repetível), `--delimiter`, `--stdin`; o `--select` global projeta a saída e nunca é flag local
  - Exemplo: `printf '{"a":1}\n' | atomwrite --workspace "$WS" extract --field a`
- `extract-artifact` -- extrai sinal delimitado de cabeçalhos de PDF, OOXML e imagem (nunca os bytes brutos do artefato); `--max-sample-rows N` limita as linhas de dados devolvidas por planilha (0 = só o cabeçalho) e `--max-text-chars N` limita o texto do documento
  - Exemplo: `atomwrite --workspace "$WS" extract-artifact img.png`
- `calc` -- avalia expressões matemáticas e conversões de unidades (engine fend); as expressões são em inglês (`2 hours to seconds`), um nome de unidade em português sai 65
  - Exemplo: `atomwrite --workspace "$WS" calc "2 hours to seconds"`
- `regex` -- gera regex a partir de exemplos (engine grex); flags como `--digits`, `--words`, `--spaces`, `--repetitions N`, `--case-insensitive` e `--no-anchors` generalizam o padrão
  - Exemplo: `atomwrite --workspace "$WS" regex abc-12 xyz-99 id-7 --digits`
- `transform` -- busca e reescrita estrutural por AST (ast-grep, 29 linguagens embutidas medidas no binário publicado); `-p`/`--pattern` com `-r`/`--rewrite`, `-l`/`--language`, `--rules`, `--inline-rules`, `--dry-run`, `--verify-parse`, `--include` e `--exclude`; zero matches sai 1
  - Exemplo: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `scope` -- escopo gramatical sobre categorias de código; `--delete` para remover matches; `--action upper|lower|titlecase|squeeze` para transformações de texto; `--replace-with "texto"` para substituição customizada; `--query` para consultas preparadas (comments, fn, strings, struct, etc); `--pattern` para padrões AST customizados; suporta Rust (30 queries), Python (13), JS/TS (11), Go (8)
  - Exemplo: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `backup` -- cria backups com timestamp e checksums BLAKE3; `--retention` para a contagem de retenção, `--output-dir`, `--dry-run` para preview
  - Exemplo: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` -- restaura um arquivo a partir de backup; `--timestamp` ou `--latest` seleciona o backup, `--verify` valida o checksum, `--dry-run` faz preview
  - Exemplo: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `apply` -- aplica patches do stdin com detecção automática de formato (unified diff, blocos SEARCH/REPLACE, markdown-fenced, arquivo completo); `--format` força o formato; um unified diff é comparado com o arquivo (deslocamento até 64 linhas) e a divergência sai 65 `MATCH_FAILED` sem gravar nada; `--strict-atomic` sai 91 em vez da queda para cópia entre dispositivos, e `--no-syntax-check` pula a checagem de parse que recusa patch que quebra o arquivo
  - Exemplo: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `batch` -- executa múltiplas operações de um manifesto NDJSON no stdin (write, replace, delete, edit, hash, move, copy); `--transaction` para tudo-ou-nada, `--dry-run`, `--batch-size`, `--file PATH` e `--input-schema`
  - Exemplo: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
  - O `--transaction` copia todo alvo antes da primeira mutação, então um `write`, `replace`, `edit`, `copy`, `delete` ou `move` dentro dela publica essa cópia em `backup_path` e diz em `warnings` que nenhuma segunda cópia foi feita; fora da transação a operação escreve a própria cópia
- `completions` -- gera completions de shell (bash, zsh, fish, elvish, powershell); o script vai para o stdout como texto puro; `--install` grava no diretório de dados da plataforma para o shell (nunca num caminho do argv, e o ajuste `[storage].home` o relocaliza enquanto o ambiente do processo não) e emite `{"type":"installed","path":"<arquivo de completion>","ok":true,"target_resolved":["<mesmo caminho>"],"target_source":"argv"}`, onde `target_source` é `argv` pelo contrato de evidência de alvo
  - Exemplo: `atomwrite --workspace "$WS" completions bash`
- `config` -- chaves persistentes de ajuste desde a v0.1.37: `path`, `list`, `get`, `set`, `unset` sobre 43 chaves do `config.toml` XDG e do `.atomwrite.toml` do workspace; `--scope global|workspace` obrigatório em `path`, `set` e `unset` (veja Configuração)
  - Exemplo: `atomwrite --workspace "$WS" config get fuzzy.threshold`
- `set` -- escreve um valor em um caminho dotted em um arquivo TOML ou JSON via `toml_edit`, preservando comentários; auto-coage int, bool, float e string
  - Exemplo: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` -- lê um valor em um caminho dotted; NDJSON: `{"type":"get","key_path","value","found","format"}`; arquivo TOML ou JSON inválido sai 65
  - Exemplo: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` -- remove uma chave; `--force-missing` trata chave ausente como no-op de sucesso
  - Exemplo: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` -- renomeia identificadores em ARQUIVOS via `heck` (um diretório sai 0 e não faz nada); `--subvert OLD NEW`, `--to snake|camel|pascal|kebab|screaming-snake`, `--dry-run`
  - Exemplo: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` -- caminha um AST tree-sitter e emite nós como NDJSON; 372 linguagens via `tree-sitter-language-pack`; modos: `--kinds`, `-Q`/`--query <PATTERN>`, `--tree`, `--positions`
  - Exemplo: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` -- extrai a estrutura de alto nível (funções, classes, structs, enums, traits, módulos) de UM arquivo de código como NDJSON; `--kind` (repetível), `--positions`
  - Exemplo: `atomwrite --workspace "$WS" outline src/a.rs`
- `wal-stats` -- inspeciona o estado do journal WAL para diagnóstico local; só leitura; escopo via `--workspace <DIR>`; relatório com `total_journals`, `by_state`, `oldest_age_secs`
  - Exemplo: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` -- remove journals terminais (committed ou aborted) mais antigos que `--threshold-secs` (padrão 3600) dentro de um PATH obrigatório; orçamento de wall-clock via `--max-duration-ms` (padrão 100); um journal deixado por crash (`Started`) nunca é removido
  - Exemplo: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `edit-loop` -- aplica N pares `{old, new}` numa escrita só, tudo ou nada, NDJSON no stdin (um par sem match sai 65); `--partial` grava os pares que casam e sai 87; também `--backup`, `--keep-backup`, `--fuzzy`, `--expect-checksum`
  - Exemplo: `printf '{"old":"gamma","new":"delta"}\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` -- limpeza manual dos siblings `.bak.*` de ARQUIVOS alvo; exige `--max-age-secs <SECONDS>` ou `--max-count <N>`; `--dry-run` é uma chave desligada por padrão
  - Exemplo: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run src/a.rs`
- `verify` -- verifica um arquivo contra um hash BLAKE3 esperado (posicional `PATH HASH`); exit 0 se casa, exit 81 se não
  - Exemplo: `atomwrite --workspace "$WS" verify src/a.rs "$(atomwrite --workspace "$WS" hash src/a.rs | jaq -r .checksum)"`
- `recipe` -- lista e roda pipelines multi-passo versionados (`search-replace-verify`, `edit-loop-syntax-check`); `recipe run` exige `--path` ou `--target`; hash recursivo pula `*.bak.*`
  - Exemplo: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `sparse` -- `list`, `read` e `outline` de monorepo sob orçamento; `sparse list` aceita `--max-files`, `--max-bytes`, `--include`, `--exclude` (globs de verdade, um inválido sai 65) e `--all` (toda entrada, oculta incluída); o outline também aceita `--all` e emite kinds AST reais
  - Exemplo: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576`
- `semantic-merge` -- merge de três vias por linha para conflitos multi-agente (não AST); markers de conflito default ON, `--write-conflict-markers true|false` declara isso e `--no-conflict-markers` desliga; `--fail-on-conflict` sai 84
  - Exemplo: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `agent-surface` -- o manifesto de tools derivado do clap (só CLI; integração por subprocesso, CLI com saída NDJSON) e a fonte única dos 47 subcomandos e dos exit codes
  - Exemplo: `atomwrite --workspace "$WS" agent-surface --format json`
- `watch` -- eventos de filesystem com `--debounce-ms N`, `--checksum` e `--gitignore`; sempre termina com o NDJSON `type:watch_summary`; exige build com `--features watch` (o binário padrão sai 78 `CONFIG_INVALID`)
  - Exemplo: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500`
- `codemod` -- campanha AST multi-regra a partir de um arquivo YAML de regras, com resumo `by_rule_id`; rode `--dry-run` antes
  - Exemplo: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` -- ranking offline de tokens por similaridade de Jaccard (sempre compilado); `--k`, `--min-score`, `--index-dir` e `--include-backups`
  - Exemplo: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `stat` -- alias de `read --stat` (só metadados)
  - Exemplo: `atomwrite --workspace "$WS" stat src/a.rs`
- `doctor` -- diagnósticos de prontidão do host; não lê nenhuma variável de ambiente; `--strict` sai 1 com `DOCTOR_CHECKS_FAILED` quando um check falha ou avisa
  - Exemplo: `atomwrite --workspace "$WS" doctor`
- `locale` -- mostra, define (`--set en|pt-BR`) ou limpa (`--clear`) a preferência de locale da UI (XDG); distinto do `--locale` global
  - Exemplo: `atomwrite --workspace "$WS" locale`
- `commands` -- emite a árvore completa de comandos em JSON para descoberta; `--include-globals` acrescenta as 33 flags globais
  - Exemplo: `atomwrite --workspace "$WS" commands --include-globals`
- `scratch` -- `scratch init <PATH>` cria o diretório de staging (canônico `.atomwrite/scratch`) dentro do workspace para `--old-file`/`--new-file` (NUNCA `/tmp` do SO fora do workspace; exit 126 `WORKSPACE_JAIL`)
  - Exemplo: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`


## OBRIGATÓRIO — Staging de conteúdo (`--old-file` / `--new-file`)
Estagie arquivos de conteúdo dentro do jail do workspace. O `scratch init` recebe o diretório como posicional obrigatório (sem padrão; sem ele o exit é 2). Prefira a fórmula da skill:

```bash
WS="$PWD"
atomwrite --workspace "$WS" scratch init .atomwrite/scratch
# então grave old/new sob $WS/.atomwrite/scratch/
```

NUNCA estagie `--old-file`/`--new-file` em `/tmp` do SO fora de `--workspace` (exit 126 `WORKSPACE_JAIL`).


## OBRIGATÓRIO — Hierarquia de edição
- Prefira o nível mais preciso que resolve a tarefa
- Ordem: `search` → `transform` → `scope` → `codemod` → `replace` → `edit` → `write`
- `write` de arquivo completo é o último recurso
- Use `transform`/`scope` para estrutura AST; `replace`/`edit` para texto; `write` só quando reescrita total for necessária
- Reescrita total intencionalmente menor: `write --allow-shrink`


## OBRIGATÓRIO -- Contrato de Saída
- stdout: SEMPRE NDJSON estruturado (um objeto JSON por linha)
- stderr: apenas logs (formato tracing, somente com `--verbose`)
- Todo evento de sucesso tem um campo discriminador `"type"` e `"ok":true`; um envelope de erro tem `"ok":false` mais `"code"` no lugar, porque o antigo duplicado `"error":true` custava 13 bytes em toda falha e nenhum consumidor decidia por ele sozinho (veja Formato de Saída JSON para as poucas saídas que não são NDJSON)
- Flush após cada linha
- NUNCA interprete stderr como dados estruturados
- SEMPRE interprete stdout linha por linha como JSON


## OBRIGATÓRIO -- Contrato CRUD
### Create (write)
- Envie conteúdo via stdin
- Receba path, bytes_written, checksum, info de plataforma
- Backup vem ligado por padrão e é removido depois de uma escrita bem-sucedida; passe `--keep-backup` para preservar a versão anterior, `--no-backup` para desligar
- Use `--expect-checksum` para locking otimista (a checagem roda antes de o lock de escrita ser tomado, então dois escritores ainda podem disputar entre a checagem e a escrita)

### Read (read)
- Receba path, content, lines, bytes, checksum, permissions, modified, kind
- Use `--stat` para pular conteúdo (apenas metadados)
- Use `--lines START:END` para leituras parciais (1-based inclusivo)
- Use `--head N` para primeiras N linhas, `--tail N` para últimas N linhas
- Use `--grep <REGEX>` para filtrar linhas retornadas às que casam com regex
- Arquivos binários são auto-detectados e conteúdo é omitido

### Update (edit, replace, transform)
- `edit` -- cirúrgico: por número de linha, marcador de texto ou match exato
- `replace` -- em massa: em múltiplos arquivos com suporte a regex
- `transform` -- estrutural: reescrita por AST em codebases
- Todos os três retornam checksums antes e depois da modificação
- Todos os três suportam `--dry-run` para preview
- `edit` e `replace` suportam `--preserve-timestamps` para dispensar a atualização de mtime (padrão: mtime é atualizado para refletir a mudança, então sistemas de build como cargo/make/cmake detectam a mudança do fonte sem `touch` manual)
- A saída NDJSON de `edit` e `replace` inclui o campo `mtime_preserved: bool` para verificar qual caminho foi tomado

### Delete (delete)
- Receba path, bytes, checksum_before
- Um backup é feito por padrão e mantido no sucesso (`--no-backup` desliga); restaure com `rollback`
- Use `-r` / `--recursive` para diretórios
- Use `--plan` para listagem plan-only (`--dry-run` também lista sem apagar); `--older-than` filtra por idade


## OBRIGATÓRIO -- Formato de Saída JSON
- Toda linha é um objeto JSON; eventos de sucesso carregam `type` e `ok: true`, envelopes de erro carregam `ok: false` mais `code` e `exit`
- Algumas saídas não são eventos NDJSON: `completions SHELL` imprime o script de shell cru, e `read --format raw` imprime bytes crus (`extract` imprime os campos escolhidos sem a chave `type`)
- Os exemplos abaixo são saída real com `--workspace /ws`; os checksums estão abreviados para `abc...`

### Resposta de Write

```json
{"type":"write","status":"success","path":"sample.txt","bytes_written":12,"checksum":"abc...","elapsed_ms":0,"stdin_bytes_read":12,"wal_policy":"auto","platform":{"fsync":"sync_data","dir_fsync":"sync_all","durability":"fast","rename_method":"renameat2"},"ok":true,"target_resolved":["/ws/sample.txt"],"target_source":"argv"}
```

### Resposta de Read

```json
{"type":"read","path":"/ws/sample.txt","content":"hello world\n","lines":1,"bytes":12,"checksum":"abc...","permissions":"0o600","modified":"2026-10-20T02:01:45Z","kind":"text","binary":false,"mode":"full","ok":true}
```

### Resposta de Edit

```json
{"type":"edit","path":"/ws/sample.txt","edits":1,"mode":"exact","bytes_before":12,"bytes_after":9,"checksum_before":"abc...","checksum_after":"def...","lines_before":1,"lines_after":1,"elapsed_ms":0,"fuzzy":false,"strategy":"exact","strategies_tried":1,"diff_preview":" hello","mtime_preserved":false,"match_count":1,"indent_adjusted":false,"ok":true,"target_resolved":["/ws/sample.txt"],"target_source":"argv"}
```

### Match de Search

```json
{"type":"begin","path":"/ws/sample.txt"}
{"type":"match","path":"/ws/sample.txt","line_number":1,"lines":"hi world","byte_offset":0,"submatches":[{"match":"hi","start":0,"end":2}]}
{"type":"end","path":"/ws/sample.txt","stats":{"matches":1,"lines_searched":1}}
{"type":"summary","files_visited":1,"files_matched":1,"total_matches":1,"elapsed_ms":3,"ok":true}
```

### Resultado de Replace

```json
{"type":"replaced","path":"/ws/sample.txt","replacements":1,"bytes_before":9,"bytes_after":10,"checksum_before":"abc...","checksum_after":"def...","elapsed_ms":0,"mtime_preserved":false,"fuzzy":false,"strategy":"exact","strategies_tried":1}
{"type":"summary","files_visited":1,"files_matched":1,"files_modified":1,"files_skipped":0,"total_replacements":1,"elapsed_ms":4,"ok":true,"target_resolved":["/ws/sample.txt"],"target_source":"argv"}
```

### Contadores de corte no evento terminal
- `records_suppressed` conta os registros que o escritor descartou antes do terminal: recusa por `--filter`, projeção de `--select` que esvaziou o registro, duplicata de dedupe e bloco cortado por `--limit`
- `records_suppressed` conta registros e nunca arquivos, então ele não se compara com `files_matched`, porque um arquivo pode contribuir com vários registros
- `files_budget_dropped` conta as entradas que o orçamento de `--limit` descartou, tanto de `cap_collected` quanto do escritor
- `files_budget_dropped` nomeia a unidade que o verbo produz: arquivos num verbo de caminhada, registros num verbo de streaming
- Os dois números são piso, porque os workers podem parar antes de produzir o resto
- `files_planned` aparece em `--dry-run` e `--preview` com os arquivos que a execução teria mudado, e `files_modified` continua zero sem escrita real
- `truncated` é carimbado no registro que `--truncate-content` ou `--max-output-bytes` cortou, e `truncated_records` é o total desses cortes na execução
- `matches_suppressed` (search `--max-count`), `replacements_suppressed` (replace `--max-replacements`), `below_min_score` (semantic-search `--min-score`) e `items_omitted` (outline `--limit`) nomeiam o corte do próprio verbo, então nenhum verbo esconde o que a flag dele removeu
- `matches_suppressed` é teto de registro e não de leitura, porque provar que o arquivo tinha mais custa no máximo um match lido além do `--max-count` naquele arquivo
- Todo campo fica ausente quando o valor é zero, e `--print-schema` declara todos, então o validador nunca vê chave não declarada

### Envelope de Erro

Search/replace/transform/scope com zero matches (exit 1): o próprio resumo é o evento terminal, com `ok:false` e o detalhe do erro na mesma linha:

```json
{"type":"summary","files_visited":1,"files_matched":0,"total_matches":0,"elapsed_ms":3,"ok":false,"code":"NO_MATCHES","exit":1,"target_resolved":["src"],"target_source":"argv","suggestion":"broaden the search pattern; check --include / --exclude filters; verify the file content","match_count":0}
```

Miss de edit/fuzzy (exit 65): inspecione o `best_candidate` opcional e ajuste `--old`:

```json
{"ok":false,"code":"MATCH_FAILED","exit":65,"target_resolved":["src/main.rs"],"target_source":"argv","match_count":0,"suggestion":"inspect best_candidate in the error NDJSON and adjust --old, or lower --fuzzy-threshold","best_candidate":{"text":"fn compute_total(items: &[u32]) -> u32 {","line":1,"column":1,"similarity":0.905,"strategy":"context_aware_jw","diff_preview":"-fn compute_totl(items+fn compute_total(items: &[u32]) -> u32 {"}}
```

- Todo envelope de erro inclui `"ok":false`, e a falha de match acrescenta `"match_count"` (0 num miss, a contagem real para `MATCH_AMBIGUOUS`)
- `"suggestion_lang"` aparece só com `-v`, a mesma flag que devolve `"message"`, `"error_class"` e `"retryable"`
- As estratégias fuzzy normalizadas (`line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `trimmed_boundary`, `escape_normalized`) omitem `similarity`; não leia a ausência como 1.0
- O campo `workspace` aparece apenas em erros `WORKSPACE_JAIL` e reporta a raiz do workspace resolvida
- `suggestion` é context-aware: a sugestão de `WORKSPACE_JAIL` muda com base em se `--workspace` foi fornecido; se `--old-file`/`--new-file` estiver fora do jail, estagie com `scratch init` sob o workspace (não diga só "ajuste `--workspace`")
- Veja `docs/schemas/` para as definições de JSON Schema de todos os tipos de saída (`atomwrite --print-schema` lista os 55 nomes; `error-output.schema.json` descreve o envelope e o campo `workspace`, e `code` é uma string livre ali)


## OBRIGATÓRIO -- Exit Codes
- Envelopes de erro no stdout são sempre NDJSON com `ok: false`; leia `exit` e `code` antes de tratar uma linha como sucesso
- A tabela abaixo tem 41 linhas sobre 33 números de saída distintos: 35 entradas de variante de erro mais `SUCCESS`, `ARGUMENT_PARSE_ERROR`, `TIMEOUT`, `SIGHUP`, `SIGINT` e `SIGTERM`; ela é gerada de `atomwrite agent-surface --format json | jaq '.exit_codes'` e confere com `atomwrite --help`
- As 35 variantes de `src/error/variants.inc.rs` viram 33 entradas porque `EditPairFailed` reusa `MATCH_FAILED` no 65 e `MissingFlag` reusa `INVALID_INPUT` no 65
- Cada linha termina com o `error_class` e a flag `retryable`, que o `-v` põe no envelope
- Ramifique primeiro por `exit` e depois por `code`, porque vários códigos dividem um número (1, 65 e 130)
<!-- exit-codes:begin -->
- `0` SUCCESS: sucesso (classe `none`, retryable `false`)
- `1` NO_MATCHES: zero matches encontrados (search, replace, transform, scope; não é falha) (classe `permanent`, retryable `false`)
- `1` DOCTOR_CHECKS_FAILED: `doctor --strict` achou checks reprovados ou com aviso (o evento do relatório traz todos os checks) (classe `precondition_failed`, retryable `false`)
- `2` UNSUPPORTED_COMBINATION: flags que não podem ser combinadas foram passadas juntas (classe `permanent`, retryable `false`)
- `2` ARGUMENT_PARSE_ERROR: linha de comando rejeitada pelo parser (classe `permanent`, retryable `false`)
- `4` FILE_NOT_FOUND: arquivo não encontrado (classe `permanent`, retryable `false`)
- `4` GLOB_NO_MATCHES: o glob não casou nenhum arquivo (classe `permanent`, retryable `false`)
- `13` PERMISSION_DENIED: permissão negada (classe `permanent`, retryable `false`)
- `28` DISK_FULL: disco cheio (sem espaço no dispositivo) (classe `transient`, retryable `true`)
- `30` QUOTA_EXCEEDED: cota excedida (classe `transient`, retryable `true`)
- `65` INVALID_INPUT: entrada inválida (argumentos ruins ou dados malformados, incluindo stdin vazio sem `--allow-empty-stdin` e arquivo acima de `[fuzzy].max_content_bytes` no fuzzy) (classe `permanent`, retryable `false`)
- `65` MATCH_FAILED: padrão não casou, ou um par do `edit-loop` sem match (leia `best_candidate`) (classe `permanent`, retryable `false`)
- `65` MATCH_AMBIGUOUS: padrão casou mais de uma vez (leia `best_candidate`) (classe `permanent`, retryable `false`)
- `65` BINARY_FILE: conteúdo binário rejeitado (classe `precondition_failed`, retryable `false`)
- `65` FILE_TOO_LARGE: arquivo acima do limite de tamanho (classe `precondition_failed`, retryable `false`)
- `69` GRAMMAR_UNAVAILABLE: a gramática tree-sitter de um idioma não está nesta máquina (baixe uma vez com a rede ligada) (classe `precondition_failed`, retryable `false`)
- `73` CROSS_DEVICE: rename cross-device (limite de filesystem) (classe `conflict`, retryable `true`)
- `74` IO_ERROR: erro de I/O (classe `transient`, retryable `true`)
- `78` CONFIG_INVALID: configuração inválida, ou verbo que precisa de uma feature ausente neste build (slim) (classe `permanent`, retryable `false`)
- `80` TRANSACTION_ROLLBACK_FAILED: rollback da transação do batch falhou (backups mantidos) (classe `permanent`, retryable `false`)
- `81` CHECKSUM_VERIFY_FAILED: verificação de checksum falhou (classe `precondition_failed`, retryable `false`)
- `82` STATE_DRIFT: state drift (mismatch de checksum, lock otimista falhou) (classe `conflict`, retryable `true`)
- `83` LOCK_TIMEOUT: lock não obtido em `lock.acquire_timeout_ms` (padrão 5000) (classe `conflict`, retryable `true`)
- `84` MERGE_CONFLICT: `semantic-merge` encontrou conflitos (classe `permanent`, retryable `false`)
- `85` FIFO_DETECTED: FIFO detectado (named pipe não pode ser escrito atomicamente) (classe `precondition_failed`, retryable `false`)
- `86` DEVICE_FILE: arquivo de dispositivo detectado (bloco ou caractere) (classe `precondition_failed`, retryable `false`)
- `87` EDIT_LOOP_PARTIAL: `edit-loop --partial` gravou os pares que casaram e alguns pares falharam (classe `permanent`, retryable `false`)
- `88` SYNTAX_ERROR_DETECTED: erro de sintaxe detectado (`--syntax-check` falhou) (classe `precondition_failed`, retryable `false`)
- `91` EXDEV_FALLBACK_DISABLED: fallback EXDEV desabilitado (levantado quando `--strict-atomic` é passado e o alvo mora em outro sistema de arquivos que o arquivo temporário) (classe `precondition_failed`, retryable `false`)
- `92` COPY_BACK_BLAKE3_FAILED: verificação BLAKE3 do copy-back falhou (classe `conflict`, retryable `true`)
- `93` ORPHAN_JOURNAL: journal órfão detectado (classe `precondition_failed`, retryable `false`)
- `124` TIMEOUT: prazo de `--timeout-secs` global excedido (default 120; passe `0` para desligar) (classe `precondition_failed`, retryable `false`)
- `126` WORKSPACE_JAIL: violação do jail do workspace (caminho escapa do workspace) (classe `precondition_failed`, retryable `false`)
- `127` SYMLINK_BLOCKED: symlink bloqueado (alvo fora do workspace) (classe `precondition_failed`, retryable `false`)
- `128` IMMUTABLE_FILE: arquivo imutável (não pode modificar) (classe `precondition_failed`, retryable `false`)
- `129` SIGHUP: terminal desligado (SIGHUP) (classe `none`, retryable `false`)
- `130` CANCELLED: execução cancelada antes de terminar (o status segue a causa) (classe `permanent`, retryable `false`)
- `130` SIGINT: interrompido por SIGINT (classe `none`, retryable `false`)
- `141` BROKEN_PIPE: pipe quebrado (SIGPIPE ou erro de escrita `BrokenPipe`) (classe `permanent`, retryable `false`)
- `143` SIGTERM: terminado por SIGTERM (classe `none`, retryable `false`)
- `255` INTERNAL_ERROR: erro interno (classe `permanent`, retryable `false`)
<!-- exit-codes:end -->
- `IO_ERROR` (74) é `transient` e retentável, exceto os casos de diretório usado como arquivo, não-diretório usado como diretório e argumento inválido, que são `precondition_failed` e não retentáveis
- `edit-loop` e `edit` multi-par reportam par sem match como `MATCH_FAILED` (65) com `best_candidate`; `edit-loop --partial` grava os pares que casam e sai 87
- `semantic-merge --fail-on-conflict` sai 84; sem a flag um merge com conflitos grava os markers e sai 0 com `status` `conflict`
- Uma execução com falha termina em EXATAMENTE UM evento terminal com `ok: false`; os campos `code`, `exit` e `suggestion` ficam dentro dele, então não espere uma segunda linha de erro


## OBRIGATÓRIO -- Tratamento de Erros
- Erros emitem JSON no stdout com `ok: false`, `code`, `exit`, `message`, `error_class`, `retryable` e `suggestion`
- Acrescente `-v` para receber `message`, `error_class`, `retryable` e `suggestion_lang`, que o envelope padrão descarta para manter toda falha pequena
- Campos opcionais aparecem só quando se aplicam: `path`, `suggestion`, `workspace`, `match_count`, `best_candidate`, `candidates`, `similar_paths`, `failed_pair_index`, `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results`, `conflict_count` (regiões em conflito de um merge de três vias) e `missing_flag` (a flag que o comando exigia)
- O resumo de sucesso de `search`, `replace`, `transform`, `case`, `scope`, `sparse`, `semantic-search` e `backup` que não conseguiu ler ou gravar alguns arquivos traz `files_failed` e `errors[]` (no máximo `summary.errors_max_entries` entradas (50 por padrão) de `path`, `code` e `message`); leia `files_failed` antes de confiar num resultado, e o contrato de família é o ADR-0077 (`docs/decisions/0077-answer-family-and-walk-honesty.md`)
- Valores `null` de topo não são mais emitidos no stdout (11 campos): uma chave ou está presente com valor ou está ausente
- Todo evento de sucesso de um verbo com efeito colateral traz `ok: true`, `target_resolved` (o caminho absoluto que rodou) e `target_source`, que é `argv` quando a linha de invocação nomeia o arquivo, `stdin` quando o patch ou o manifesto o nomeia, e `workspace` no padrão `.` de `count` e `list`; confira o caminho antes de confiar no efeito
- O registro `rollback` do `apply` e do `batch --transaction` publica sempre `target_resolved` como array, mesmo quando um arquivo só voltou, porque quem lê não sabe a contagem antes de ler o campo
- Valores de `error_class`: `permanent`, `transient`, `conflict`, `precondition_failed` (o manifesto usa também `none` para `SUCCESS` e para os sinais)
- `retryable` é true só para as classes `transient` e `conflict`
- `suggestion` segue o locale da UI e `suggestion_lang` nomeia o idioma em que foi gerada; `message` e `code` ficam em inglês
- Campo `workspace` aparece apenas em erros `WORKSPACE_JAIL` e reporta a raiz do workspace resolvida
- As 35 variantes de erro carregam `suggestion` acionável, exceto `BrokenPipe` (SIGPIPE), que não é acionável pelo usuário
- A sugestão de `WorkspaceJail` é context-aware: quando `--workspace` já está definido ela diz "use a path inside the workspace (<root>)"; se arquivos de content-source (`--old-file`/`--new-file`) estiverem fora do jail, estagie-os com `scratch init` sob o workspace
- A sugestão de `BinaryFile` recomenda `read --stat` para leituras somente de metadados
- A sugestão de `FileImmutable` menciona `chattr -i` (Unix) e `fsutil` (Windows)
- A sugestão de `NoMatches` orienta ampliação do padrão e revisão dos filtros `--include`/`--exclude`
- `code` é uma string livre em `docs/schemas/error-output.schema.json`; a tabela acima é a lista para ramificar

### API de Sugestão Context-Aware (v0.1.4)
- Nova API Rust: `ErrorJson::from_error_with_context(err, &ErrorContext)` aceita proveniência de workspace
- A struct `ErrorContext` tem `workspace_provided: bool` e `workspace: Option<PathBuf>`
- `ErrorJson::from_error(err)` legacy ainda funciona e produz o mesmo output que a nova API com contexto padrão
- Consumidores programáticos podem chamar `from_error_with_context` para influenciar o texto da sugestão


## OBRIGATÓRIO -- Estratégia de Retry
- Leia `retryable` e `error_class` do envelope sob `-v`; os grupos abaixo são as linhas da tabela acima
- NUNCA tente novamente erros `permanent` ou `precondition_failed` sem corrigir a causa

### Erros Transientes (retryable: true)
- `DISK_FULL` (exit 28) -- aguarde espaço e tente novamente
- `QUOTA_EXCEEDED` (exit 30) -- aguarde reset de cota e tente novamente
- `IO_ERROR` (exit 74) -- tente novamente com backoff exponencial

### Erros de Conflito (retryable: true)
- `STATE_DRIFT` (exit 82) -- releia o arquivo, obtenha novo checksum, tente com `--expect-checksum` atualizado; `case`, `transform`, `scope` e `codemod` também o levantam para arquivo que mudou desde a leitura, então rode de novo e nunca assuma que a gravação aconteceu
- `LOCK_TIMEOUT` (exit 83) -- o lock advisory automático não foi obtido em `lock.acquire_timeout_ms` (padrão 5000); tente com backoff ou aumente a chave
- `COPY_BACK_BLAKE3_FAILED` (exit 92) -- o copy-back in-place perdeu integridade de checksum; releia o arquivo e tente de novo
- `CROSS_DEVICE` (exit 73) -- atomwrite trata internamente via cópia-depois-deleta

### Erros Permanentes (retryable: false)
- `NO_MATCHES` (exit 1) -- não é falha; amplie o padrão ou os filtros
- `ARGUMENT_PARSE_ERROR` (exit 2) -- corrija a linha de comando usando `suggestion`
- `FILE_NOT_FOUND` (exit 4) -- verifique se o path existe antes de tentar novamente
- `PERMISSION_DENIED` (exit 13) -- não tente novamente sem corrigir permissões
- `INVALID_INPUT` (exit 65) -- corrija a entrada e tente novamente; regex, glob, padrão AST ou regra YAML inválidos também caem aqui, nunca em 255
- `MATCH_FAILED` (exit 65) -- leia `best_candidate` e ajuste `--old`, ou reduza `--fuzzy-threshold`
- `MATCH_AMBIGUOUS` (exit 65) -- leia `best_candidate` e `match_count`, amplie `--old` ou use `--replace-all` (edit); nunca chute
- `CONFIG_INVALID` (exit 78) -- corrija a configuração, ou instale um build com a feature ausente (por exemplo `--features watch`), e tente novamente
- `TRANSACTION_ROLLBACK_FAILED` (exit 80) -- o rollback do `batch --transaction` falhou; os backups foram mantidos, inspecione-os
- `MERGE_CONFLICT` (exit 84) -- resolva os markers de conflito do `semantic-merge`
- `EDIT_LOOP_PARTIAL` (exit 87) -- `edit-loop --partial` gravou os pares que casam; corrija ou descarte os que falharam
- `CANCELLED` (exit 130) -- a execução foi cancelada antes de terminar; o evento `cancelled` diz o motivo
- `BROKEN_PIPE` (exit 141) -- o leitor fechou o pipe e a saída está incompleta; reconfira o estado
- `INTERNAL_ERROR` (exit 255) -- reporte como bug; não acionável pelo usuário

### Pré-condição Falhou (retryable: false)
- `DOCTOR_CHECKS_FAILED` (exit 1) -- `doctor --strict` achou checks reprovados ou com aviso; corrija e rode de novo
- `BINARY_FILE` (exit 65) -- use modo `--stat` para ler metadados sem conteúdo
- `FILE_TOO_LARGE` (exit 65) -- aumente `--max-filesize` ou processe um arquivo menor
- `CHECKSUM_VERIFY_FAILED` (exit 81) -- o hash passado para `verify`, `hash --verify` ou `read --verify-checksum` não confere; releia o arquivo
- `FIFO_DETECTED` (exit 85) -- pule este arquivo ou use redirecionamento de stdin
- `DEVICE_FILE` (exit 86) -- pule este arquivo ou use redirecionamento de stdin
- `SYNTAX_ERROR_DETECTED` (exit 88) -- `--syntax-check` rejeitou o conteúdo; corrija a sintaxe
- `EXDEV_FALLBACK_DISABLED` (exit 91) -- `--strict-atomic` recusou o fallback de cópia cross-device; leve o alvo para o mesmo sistema de arquivos do workspace, ou retire a flag
- `ORPHAN_JOURNAL` (exit 93) -- um journal WAL órfão foi detectado; inspecione com `wal-stats` e `wal-heal`
- `TIMEOUT` (exit 124) -- o prazo de `--timeout-secs` passou; aumente-o ou estreite o path
- `WORKSPACE_JAIL` (exit 126) -- não tente novamente com o mesmo path externo; estagie arquivos de content-source com `scratch init` sob o workspace, ou defina `--workspace` para a raiz permitida (por exemplo skill do host sob home em multi-root)
- `SYMLINK_BLOCKED` (exit 127) -- não tente novamente com symlinks desabilitados
- `IMMUTABLE_FILE` (exit 128) -- remova a flag de imutabilidade primeiro (`chattr -i` no Unix, `fsutil` no Windows)

### Não São Erros (classe none)
- `SUCCESS` (exit 0), mais `SIGHUP` (129), `SIGINT` (130) e `SIGTERM` (143): o processo foi parado por um sinal e terminou limpo, sem tempfile, journal ou lock sobrando


## OBRIGATÓRIO -- Flags Globais
- 33 flags globais (`atomwrite --help`, `atomwrite commands --include-globals`); todo subcomando as aceita, antes ou depois do nome do verbo, exceto `--print-schema` (coloque-a ANTES do comando). O `--json` oculto é aceito e ignorado (a saída é sempre NDJSON)
- `--workspace <PATH>` -- SEMPRE passe, absoluto, para restringir operações à raiz do projeto (o jail é ancorado nesse valor)
- `--config <PATH>` -- arquivo de configuração explícito; ele substitui os arquivos global e do workspace em `config get` e `config list`; falha se ausente ou malformado
- `-v` / `--verbose` -- repetível (`-v` info, `-vv` debug, `-vvv` trace), tracing no stderr
- `-q` / `--quiet` -- repetível (`-q` error, `-qq` off)
- `--color <auto|always|never>` -- controla saída colorida; padrão `auto`
- `--no-color` -- desabilita saída colorida (equivalente a `--color never`)
- `--ready-file <PATH>` -- grava o PID depois que os handlers de sinal são instalados; só para harness de teste, agentes one-shot omitem
- `--no-gitignore` -- não respeita regras do .gitignore
- `--hidden` -- inclui arquivos e diretórios ocultos
- `--follow-symlinks` -- segue links simbólicos
- `--threads <N>` / `-j <N>` (alias `--max-concurrency`) -- omitido ou `0` é automático: todas as CPUs lógicas até um teto embutido de 16, limitado pela RAM; `N` são exatamente N workers, também acima de 16
- `--max-filesize <BYTES>` -- padrão 1 GiB; arquivo maior é REJEITADO (`read`, `edit`, `hash` e `write` saem 65 `FILE_TOO_LARGE`), e no `write` o mesmo teto limita o payload do stdin, então payload grande demais é recusado antes de drenar o pipe; o `search` tem seu próprio `--max-filesize` local (padrão 10 MiB) que PULA arquivos maiores e os conta em `files_skipped`
- `--timeout-secs <SECONDS>` (alias `--timeout`) -- timeout global de operação; padrão 120; `0` desabilita; prazo esgotado → exit 124. Use para limitar buscas longas, batches e operações replace
- `--no-progress` -- desabilita eventos NDJSON de progresso em operações longas
- `--json-schema` -- emite o JSON schema da saída do subcomando e sai
- `--print-schema [NAME]` -- sem nome lista os 55 nomes de schema, com nome emite aquele schema; deve vir antes do nome do comando; nome desconhecido sai 65
- `--non-interactive` -- declara execução hospedada por agente: nunca pergunta, nunca lê confirmação do stdin
- `--confirm-side-effect` -- reconhece um efeito colateral onde o contrato do comando pede; aceita nos verbos que mudam bytes em disco e recusada com exit 65 `INVALID_INPUT` em todo outro verbo, para a flag nunca valer como no-op bem-sucedido
- `--allow-truncate` -- opt-in exigido por `--max-output-bytes`; cada uma é rejeitada sem a outra (exit 2)
- `--locale <en|pt-BR>` -- substitui o locale de exibição; a precedência é `--locale`, a preferência XDG (`atomwrite locale`), o locale do SO, `en`; qualquer outra tag sai 2 e lista `available`; use apenas `--locale` (sem knobs de env de produto)
- `--no-auto-heal` -- pula o heal de WAL no startup (padrão: limiar 3600 s, orçamento 100 ms)
- `--select` / `--fields <KEYS>` -- projeta chaves top-level do NDJSON (CSV)
- `--filter <EXPR>` -- mantém eventos que casam `key=value`, `key==value` ou `key!=value` (AND repetível, só top-level)
- `--limit <N>` -- teto de trabalho: a lista de alvos é cortada antes da ação e a varredura para no registro que atinge o teto, então `--limit 1` toca exatamente um alvo (só long; sem `-n` global; `replace` usa `-n`)
- `--sort-by <KEY>` -- ordena os eventos em buffer por uma chave, `path-line` ou `line-path`
- `--dedupe <KEYS>` -- mantém o primeiro evento de cada grupo duplicado (CSV de chaves)
- `--aggregate <KEY[:OP[:FIELD]]>` -- uma linha de resumo por grupo; `OP` é `count` (padrão), `sum`, `min` ou `max`
- `--join <KEY:SUFFIX>` -- anexa a tabela NDJSON lida do stdin a cada evento sob `SUFFIX`
- `--truncate-content <KEY:LEN>` -- corta os valores string de `KEY` em `LEN` caracteres
- `--redact` -- troca valores com forma de segredo por `[redacted]`
- `--normalize-encoding` -- normaliza todo valor string para Unicode NFC
- `--diff-against <PATH>` -- troca os eventos ao vivo por uma linha `stream_diff` (`added`, `removed`, `unchanged`) contra um arquivo NDJSON base dentro do jail
- `--max-output-bytes <BYTES>` -- limita todo valor string, `checksum` incluído, numa fronteira UTF-8; exige `--allow-truncate`; nunca use as duas quando o `checksum` ou o `content` inteiro for necessário
- Nota: eventos de erro sempre emitem mesmo sob `--filter`; `--select`/`--fields` opcional pode afinar chaves; `--limit` só em eventos de sucesso


## OBRIGATÓRIO -- Configuração
- O ajuste persistente vive em 43 chaves de dois arquivos TOML: o `config.toml` XDG global (modo 0600; imprima o caminho com `config path --scope global`) e o `.atomwrite.toml` do workspace na raiz do workspace
- Precedência: flags, depois o `.atomwrite.toml` do workspace, depois o `config.toml` global (os dois arquivos são mesclados chave a chave), depois o padrão embutido. Não há variáveis de ambiente de produto
- Mude uma chave só com o verbo `config`: `config path|list|get|set|unset|doctor`; o `set` preserva comentários, valida o arquivo inteiro e grava por tempfile-fsync-rename sem backup
- `--scope global|workspace` é obrigatório em `path`, `set` e `unset` (sem padrão, exit 2 sem ele); `list` e `get` não aceitam `--scope`
- `list` e `get` mostram `value`, `origin` (`default`, `global`, `workspace` ou `explicit` quando `--config` é usada) e `default`
- O evento terminal traz `target_resolved` e `target_source` (`argv`): confira que o caminho é o arquivo do escopo pretendido
- Chave desconhecida ou valor fora da faixa sai 65 e não grava nada; um `config.toml` que não é TOML válido sai 78 e nunca é reescrito
- `log.file`, `log.rotation`, `storage.home` e `locale` são só globais (escopo `workspace` sai 65)
- `defaults.line_ending`, `defaults.backup`, `defaults.retention` e `defaults.max_filesize` são chaves comuns do `config`, lidas e gravadas como qualquer outra; `backup` distingue ausente de `false`, então a chave ausente mantém o padrão de cada verbo

| Chave | Tipo | Padrão | Faixa ou valores | Escopos |
|---|---|---|---|---|
| `fuzzy.mode` | string | `auto` | `auto`, `aggressive`, `off` | global, workspace |
| `fuzzy.threshold` | float | `0.7` | 0 a 1 | global, workspace |
| `fuzzy.threshold_aggressive` | float | `0.5` | 0 a 1 | global, workspace |
| `fuzzy.threshold_context` | float | `0.8` | 0 a 1 | global, workspace |
| `fuzzy.threshold_jw` | float | `0.85` | 0 a 1 | global, workspace |
| `fuzzy.soft_floor_delta` | float | `0.05` | 0 a 1 | global, workspace |
| `fuzzy.max_content_bytes` | integer | `268435456` | 1 a 9223372036854775807 | global, workspace |
| `write.confirm_large_bytes` | integer | `102400` | 1 a 9223372036854775807 | global, workspace |
| `log.file` | boolean | `false` | `true`, `false` | global |
| `log.rotation` | string | `daily` | `daily`, `hourly` | global |
| `storage.home` | string | nenhum | um diretório | global |
| `locale` | string | nenhum | `en`, `pt-BR` | global |
| `defaults.backup` | boolean | none (true para reescritas, false para delete) | `true`, `false` | global, workspace |
| `delete.backup` | boolean | nenhum (cai para `defaults.backup`) | `true`, `false` | global, workspace |
| `defaults.line_ending` | string | `auto` | `auto`, `lf`, `cr-lf`, `cr` | global, workspace |
| `defaults.max_filesize` | integer | `1073741824` | inteiro | global, workspace |
| `defaults.retention` | integer | `5` | inteiro | global, workspace |
| `search.context` | integer | `0` | inteiro | global, workspace |
| `search.max_columns` | integer | `500` | 1 ou mais | global, workspace |
| `search.max_filesize` | integer | `10485760` | 1 ou mais | global, workspace |
| `search.smart_case` | boolean | `true` | `true`, `false` | global, workspace |
| `search.format` | string | `ndjson` | `ndjson`, `text` | global, workspace |
| `read.max_output_bytes` | integer | `24000` | 1 ou mais | global, workspace |
| `read.number` | boolean | `false` | `true`, `false` | global, workspace |
| `output.profile` | string | `full` | `full`, `lean` | global, workspace |
| `apply.max_ellipsis_lines` | integer | `200` | 1 ou mais | global, workspace |
| `watch.debounce_ms` | integer | `200` | integer | global, workspace |
| `watch.idle_exit_ms` | integer | `500` | inteiro | global, workspace |
| `write.auto_rotate_max_age_secs` | integer | `86400` | 1 ou mais | global, workspace |
| `write.content_risk_patterns` | list | os 14 padrões embutidos | lista de strings | global, workspace |
| `write.shrink_block_percent` | integer | `50` | 1 a 99 | global, workspace |
| `concurrency.max_threads` | integer | `16` | 1 a 256 | global, workspace |
| `concurrency.ram_fraction_percent` | integer | `50` | 1 a 100 | global, workspace |
| `search.max_inflight_bytes` | integer | `8388608` | 4194304 a 1073741824 | global, workspace |
| `summary.errors_max_entries` | integer | `50` | 1 a 1000 | global, workspace |
| `lock.acquire_timeout_ms` | integer | `5000` | 100 a 3600000 | global, workspace |
| `retry.base_ms` | integer | `50` | 1 a 60000, sem passar de `retry.max_ms` | global, workspace |
| `retry.max_ms` | integer | `400` | 1 a 60000 | global, workspace |
| `retry.max_elapsed_ms` | integer | `2000` | 1 a 3600000 | global, workspace |
| `extract.max_sample_rows` | integer | `10` | 0 a 100000 | global, workspace |
| `extract.max_text_chars` | integer | `400` | 1 a 10000000 | global, workspace |
| `extract.max_sheets` | integer | `32` | 1 a 10000 | global, workspace |
| `extract.max_slides` | integer | `64` | 1 a 10000 | global, workspace |

```bash
WS="$PWD"
atomwrite --workspace "$WS" config get fuzzy.threshold
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace
atomwrite --workspace "$WS" config unset fuzzy.threshold --scope workspace
```


## PROIBIDO -- Armadilhas Comuns
- NUNCA interprete stderr como dados; contém apenas logs de tracing
- NUNCA assuma que exit code 1 é erro fatal; significa zero matches em search, replace, transform ou scope (só o `doctor --strict` usa exit 1 para checks reprovados, com o código `DOCTOR_CHECKS_FAILED`)
- NUNCA omita `--workspace` ao executar como agente
- NUNCA omita `--dry-run` antes de operações batch destrutivas
- NUNCA use expressões sem aspas com `calc`; o shell vai interpolar
- NUNCA ignore `checksum_before` e `checksum_after` em respostas de edit/replace
- NUNCA tente novamente erros `permanent` ou `precondition_failed` sem corrigir a causa
- NUNCA passe `delete --confirm` / `--yes` / `-y` (rejeitados fail-closed; use `--plan` ou omita as flags)
- NUNCA escreva alvos existentes grandes sem `--ack-overwrite` (default-deny acima de XDG `[write].confirm_large_bytes`)
- NUNCA assuma stdout vazio em `watch` idle — sempre termina com `type:watch_summary`
- NUNCA invente knobs de produto `ATOMWRITE_*` em runtime (somente CLI + XDG / `.atomwrite.toml` em `^0.1.38`)
- NUNCA estagie `--old-file`/`--new-file` em `/tmp` do SO fora de `--workspace` (use `scratch`)
- NUNCA invente `edit --strategy` (use apenas `--fuzzy auto|aggressive|off`)
- NUNCA passe `$VAR` de shell sem aspas em corpos quando o `$` literal deve sobreviver (use `<<'EOF'` ou staging em arquivo)
- NUNCA use edit/replace fuzzy para rename estrutural de símbolo (use transform/scope/codemod)
- NUNCA rode `--fuzzy aggressive` em árvore inteira sem `--dry-run` antes
- SEMPRE defina `--workspace` para a raiz permitida ao editar paths multi-root (ex.: skills do host sob home)
- Prefira patches baseados em arquivo para conteúdo com `//`, jq, aspas, multilinha
- Reescrita total intencionalmente menor: `write --allow-shrink`
- Prefira `--fuzzy auto` (cascata padrão para agentes); use `--fuzzy off` só para exact-only intencional (G-010)


## OBRIGATÓRIO -- Orçamento de Tokens
- Cada subcomando: 1 chamada bash, ~50-200 tokens de saída
- Modo batch: 1 chamada bash para N operações
- Checksum na resposta elimina 1 leitura de verificação por escrita
- Uma sessão típica de agente economiza 500+ chamadas versus operações individuais


## OBRIGATÓRIO -- Locking Otimista
- Leia um arquivo e capture seu `checksum` da resposta
- Passe o checksum via `--expect-checksum` na próxima escrita ou edição
- Se o arquivo mudou entre leitura e escrita, atomwrite retorna exit 82 (`STATE_DRIFT`)
- Releia o arquivo para obter o checksum atual e tente novamente
- Isso previne atualizações perdidas em workflows concorrentes de agentes


## Operações de Backup v0.1.21
- Por padrão, backups são DELETADOS após a operação completar com sucesso
- Use `--keep-backup` para preservar o backup após sucesso
- Backups de operações que FALHARAM são sempre preservados para inspeção
- `delete` mantém o `.bak.<timestamp>` depois de uma execução bem-sucedida, enquanto `replace` o apaga como os outros mutadores
- `cleanup_old_backups_in` mantém N backups apenas para casos com `--keep-backup`
- Novo sub-comando `prune-backups` para limpeza manual de backups legados


## Padrão Sequencial v0.1.21
- 5 edits sequenciais SEM re-captura = 4 falham com `STATE_DRIFT` (exit 82)
- 5 edits COM re-captura = todos passam (padrão canônico)
- 5 edits COM `--allow-sequential-drift` = todos passam com warning
- `edit-loop` aplica N pares em 1 invocação (sem STATE_DRIFT interno)


## Subcomandos v0.1.22

Dois novos sub-comandos fecham as frentes rejeitadas em planos anteriores (`gaps.md` daquela versão, linhas 82-83 e 201):

### `edit-loop` — N Pares em 1 Invocação
- Quando usar: aplicar lote de transformações textuais em um arquivo onde hoje você invocaria `edit` N vezes em loop shell
- Input: NDJSON via stdin com um objeto `{old, new}` por linha
- Comportamento: lê o arquivo UMA vez, aplica todos os pares em memória, escreve atomicamente UMA vez
- Vantagem vs shell loop: 1 invocação CLI em vez de N; 1 read + 1 write em vez de N+N; sem `STATE_DRIFT` interno entre pares

```bash
WS="$PWD"

# Aplicar 3 pares em 1 invocação (tudo ou nada: um par sem match sai 65 e não grava nada)
printf '%s\n' \
  '{"old":"v0_1_20","new":"v0_1_22"}' \
  '{"old":"foo","new":"bar"}' \
  '{"old":"baz","new":"qux"}' \
  | atomwrite --workspace "$WS" edit-loop src/version.rs

# Com backup preservado (linha do tempo forense)
printf '%s\n' '{"old":"foo","new":"bar"}' \
  | atomwrite --workspace "$WS" edit-loop --backup --keep-backup src/foo.rs

# Melhor esforço com --partial: grava os pares que casam e sai 87 (EDIT_LOOP_PARTIAL) se algum falhou
printf '%s\n' '{"old":"exists","new":"X"}' '{"old":"missing","new":"Y"}' \
  | atomwrite --workspace "$WS" edit-loop --partial src/foo.rs
```

- Schema NDJSON: `docs/schemas/edit-loop-output.schema.json` — campos `pairs_total`, `pairs_applied`, `pairs_unmatched`, `pair_results[].index`, `pair_results[].matched`
- ADR: `docs/decisions/0039-edit-loop-helper.md`

### `prune-backups` — Limpeza Manual de Legados
- Quando usar: operadores que atualizaram de v0.1.20 herdam siblings `.bak.YYYYMMDD_HHMMSS` que v0.1.21 não cria mais (e portanto não limpa mais automaticamente)
- Algoritmo: `PATHS...` são os ARQUIVOS alvo, não diretórios; para cada arquivo ele lista os siblings chamados `<arquivo>.bak.*` e aplica o filtro de idade ou contagem (idade primeiro, depois a contagem nos sobreviventes)
- Filtro obrigatório: passe `--max-age-secs` ou `--max-count`; sem um dos dois a execução sai 2
- Segurança: `--dry-run` é uma chave sem valor e vem DESLIGADA por padrão, então uma execução sem ele apaga; veja antes com `--dry-run` e nunca escreva `--dry-run false` (a palavra `false` é lida como mais um caminho)

```bash
WS="$PWD"

# Prévia: lista os backups de src/a.rs mais antigos que 24 horas (nada é removido)
atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run src/a.rs

# Remove os backups: omita --dry-run para aplicar
atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 src/a.rs

# Mantém só os 3 backups mais recentes de src/a.rs
atomwrite --workspace "$WS" prune-backups --max-count 3 src/a.rs

# Afirma que nenhum backup do arquivo sobrou depois da limpeza
atomwrite --workspace "$WS" prune-backups --max-age-secs 0 src/a.rs \
  && fd -c never -g 'a.rs.bak.*' src | wc -l | jaq -e '. == 0'
```

- Schema NDJSON: `docs/schemas/prune-backups-output.schema.json` — campos `path`, `reason`, `action`, `total`, `elapsed_ms`
- ADR: `docs/decisions/0040-prune-backups-subcommand.md`
- Nota operacional: limpeza retroativa automática foi explicitamente rejeitada (ver ADR-0038 Addendum); operadores devem rodar `prune-backups` sob demanda


## Limitações Conhecidas (v0.1.38)
- `--expect-checksum` é checado antes de o lock de escrita ser tomado, então uma janela de TOCTOU permanece entre a checagem e a escrita
- Um nome de alvo acima de 255 bytes faz o radical do sidecar de lock virar um digest de 16 caracteres desse nome, então o sidecar fica dentro do limite do sistema de arquivos em vez de falhar com `ENAMETOOLONG`
- Uma escrita in-place num diretório que proíbe criar o sidecar segue pelo lock do inode, e a execução só é recusada quando o alvo não abre para o lock e o sidecar também não pode ser criado
- O lock de escrita é automático no Unix e no Windows, e o lock do Windows não foi executado nesta versão
- O teto de RAM da contagem automática de threads existe só no Linux e não lê limites de cgroup
- O `--limit` global faz a varredura sair no registro que atinge o teto, e o resumo terminal traz `walker_stopped_early: true` só quando o próprio caminhante saiu, então total menor que a árvore nunca é lido como o tamanho da árvore
- `files_skipped` conta tanto os arquivos acima de `--max-filesize` quanto os binários que a execução padrão se recusa a buscar, então um arquivo ausente da resposta é nomeado num campo em vez de sumir
- Com `--sort-by` e as outras flags de formato com buffer o evento de falha entra no buffer, e `--filter`, `--select` e `--limit` nunca o descartam
- O `transform` multi-regra termina com um resumo da invocação inteira; regra que falha acrescenta um `rule_error` e o agregado continua o único registro terminal
- O verbo `wal-heal` nunca remove um journal `Started` deixado por processo morto (`kill -9`), e o sidecar de lock desse processo continua no disco
- O `count` falha fechado na primeira leitura com erro e ainda publica `files_failed`, `errors[]` e a contagem dos arquivos que conseguiu ler, com `ok:false`, `code:"PERMISSION_DENIED"` e `exit:13`
- Os verbos que respondem com parte da árvore (`search`, `replace`, `transform`, `scope`, `sparse`, `semantic-search` e `case`) mantêm `ok:true` e nomeiam os arquivos que não conseguiram ler, e o ADR-0077 fixa as duas famílias
- O relator de panic do Rust (`human-panic`) grava seu relatório como arquivo local no diretório temporário e nunca o transmite


## OBRIGATÓRIO -- Regra Zero e Portões
- Regra Zero: rode um comando contra o binário real antes de dizer que ele funciona, e relate a falha com o exit code e o `code`, nunca com "deveria funcionar"
- Precedência das instruções: a mensagem do dono no turno atual vence, depois um `AGENTS.override.md` ao lado do arquivo que você edita, depois o `AGENTS.md` aninhado mais próximo, depois este arquivo
- Edite os arquivos do projeto com o próprio atomwrite (`edit`, `write`, `replace`), estagie patches em `$WS/.atomwrite/scratch/` e passe `--workspace "$WS"` em toda operação de arquivo
- Todo portão roda na máquina local: o projeto não tem automação remota, nem CI, nem arquivos de workflow
- Envolva cada comando longo em `timeout <SECONDS>` com inteiro positivo, para que um build ou teste travado não bloqueie a sessão
- NUNCA faça commit, tag ou publicação sem ordem do dono na mensagem atual

```bash
# Build
timeout 900 cargo build --release --locked
timeout 1800 cargo build --release --features full

# Format e lint
cargo fmt --check
timeout 900 cargo clippy --locked --all-targets --all-features -- -D warnings

# Docs
RUSTDOCFLAGS="-D warnings" timeout 600 cargo doc --no-deps --locked

# Test
timeout 1800 cargo test --locked
timeout 600 cargo test --test docs_consistency --test docs_exit_codes

# Features: prova que os builds slim core e core+watch compilam
timeout 900 cargo xtask check-features

# Audit e deny: antes do bump de versão, não depois de escrever a documentação
timeout 300 cargo audit
timeout 300 cargo deny check

# Package: lista os arquivos que seriam publicados (docs/AGENTS.md deve estar na lista)
timeout 600 cargo package --list --locked

# Instala o build que você acabou de testar
cargo install --path . --force
```

- Build: o `cargo build --release --locked` padrão é o portão; o perfil `full` acrescenta todas as linguagens e o `watch` (o verbo `watch` exige `--features watch`)
- Test: `cargo test --no-default-features --features core` compila mas NÃO fica verde (44 testes em 11 alvos precisam das features de AST), então use `cargo xtask check-features` para o perfil slim
- Coverage: não há portão de cobertura; a suíte de integração em `tests/` é a métrica (`docs/TESTING.md`)
- Package: o tarball deve conter `docs/AGENTS.md` e `docs/AGENTS.pt-BR.md`; confira a lista antes de qualquer release
- Publish: decisão humana; um agente nunca roda `cargo publish`


## OBRIGATÓRIO -- Caminhos por Plataforma (XDG)
- Todo caminho abaixo sai de `ProjectDirs::from("", "", "atomwrite")` do crate `directories` (`src/storage.rs`); a chave `storage.home` os realoca para `{home}/config`, `{home}/data`, `{home}/cache` e `{home}/state`, enquanto o `config.toml` de bootstrap que guarda `storage.home` fica no caminho de config da plataforma
- Pergunte ao binário em vez de adivinhar: `atomwrite config path --scope global`, `atomwrite config path --scope workspace` e `atomwrite locale` (campo `preference_path`) imprimem os arquivos reais

| Item | Linux (medido) | macOS (não validado) | Windows (compila no `cargo check`, nunca executado) |
|---|---|---|---|
| `config.toml` global | diretório XDG de config, `atomwrite/config.toml` (`~/.config/atomwrite/config.toml`) | `~/Library/Application Support/atomwrite/config.toml` | `%APPDATA%\atomwrite\config\config.toml` |
| Arquivo do workspace | `<workspace>/.atomwrite.toml` | igual | igual |
| Preferência de locale | `atomwrite/locale` no diretório de config | mesmo diretório | mesmo diretório |
| Arquivo de log (`log.file = true`) | diretório XDG de state, `atomwrite/logs/atomwrite.<data>.jsonl` | cai para o diretório de dados | cai para o diretório de dados |
| `completions bash --install` | diretório XDG de dados, `bash-completion/completions/atomwrite` | não validado | não validado |
| Parsers tree-sitter | diretório XDG de cache, `tree-sitter-language-pack/v1.22.1/` (bundle de 23.031.807 bytes e cerca de 4 MiB de bibliotecas extraídas, baixados no primeiro uso) | diretório de cache | diretório de cache |

- O Linux foi executado numa bancada cujo HOME e diretórios XDG ficavam dentro de um diretório temporário; `config path --scope global` imprimiu `<diretório de config>/atomwrite/config.toml` com `exists: false` antes do primeiro `config set`, e `config path --scope workspace` imprimiu `<workspace>/.atomwrite.toml`
- O arquivo global nasce com modo 0600 no Unix; o arquivo de log é 0600 numa pasta 0700
- As entradas de macOS e Windows foram lidas do código e da convenção do crate, não executadas; confirme-as com `config path --scope global` no host


## OBRIGATÓRIO -- Precedência do Locale
- Ordem: a flag `--locale`, depois a preferência persistida (`atomwrite locale --set en|pt-BR`, guardada no arquivo `locale` do diretório de config), depois o locale do SO (`sys-locale`), depois `en`
- Medido com o locale do SO `pt-BR`: `atomwrite locale` resolveu `pt-BR` com `source: system`; depois de `locale --set en` resolveu `en` com `source: persisted`; com `--locale pt-BR` resolveu `pt-BR` com `source: cli` enquanto `persisted` seguiu `en`; depois de `locale --clear` voltou a `pt-BR` e `system`
- `locale --set` e `locale --clear` relatam o estado da execução ATUAL e uma `note` diz que a mudança vale a partir da próxima invocação; rode `locale` de novo para ver o estado novo
- Qualquer tag além de `en` e `pt-BR` sai 2 e lista `available`; um locale que o SO informa mas o atomwrite não traz resolve para `en`
- Só o texto de `suggestion` é traduzido; `code`, `message` e toda chave JSON ficam em inglês
