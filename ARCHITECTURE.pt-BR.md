[Read in English](ARCHITECTURE.md)


# Arquitetura


## Visão Geral
- atomwrite é um CLI Rust de binário único para operações atômicas de arquivo
- Projetado para agentes LLM que precisam de manipulação de arquivos segura e estruturada
- Toda escrita é atômica: tempfile, fsync, rename, fsync do diretório
- Toda resposta é NDJSON no stdout com checksums BLAKE3
- Todos os logs vão para stderr via tracing


## Mapa de Módulos

### Ponto de Entrada
- `src/main.rs` — entrada do binário: setup de signals, init de tracing, dispatch
- `src/lib.rs` — raiz da library: declarações de módulo e dispatcher `run()`
- `src/cli.rs` — clap `#[derive(Parser)]` com flags globais
- `src/cli_args.rs` — structs de argumentos por subcomando e enums de valor

### Pipeline Principal
- `src/atomic.rs` — pipeline de escrita atômica: tempfile + fsync + rename + fsync dir
- `src/checksum.rs` — cálculo de hash BLAKE3 para arquivos e slices de bytes (usa memmap2 para arquivos grandes)
- `src/file_io.rs` — leitura inteligente de arquivos com memmap2 automático acima de 1 MiB
- `src/platform.rs` — fsync específico de plataforma: F_FULLFSYNC em macOS via libc::fcntl

### Segurança e Validação
- `src/path_safety.rs` — jail do workspace: prevenção de path traversal, validação de symlinks, detecção de FIFO/device
- `src/signal.rs` — tratamento de SIGINT/SIGTERM via signal-hook com coordenação de shutdown gracioso
- `src/error.rs` — enum de erro de domínio com códigos de saída, classificação de erro e flag retryable
- `src/lock.rs` — locking de arquivo advisory via flock(2) no sidecar `.<target>.atomwrite.lock`

### Recuperação de Crash (v0.1.12, G114)
- `src/wal.rs` — escritor WAL sidecar: anexa entradas `Started` e `Committed` em `.atomwrite.journal.<target>.atomwrite.journal.json`. Fornece `recover_orphan_journals(dir)` para recuperação consultiva. 8 testes unitários.

### Verificação de Sintaxe (v0.1.12, G72)
- `src/syntax_check.rs` — verificação de sintaxe REAL via `tree-sitter-language-pack`. Substitui a heurística de balanceamento de colchetes do v0.1.11. Suporta 24 linguagens out-of-the-box. Faz fallback para heurística legada em extensões desconhecidas. 16 testes unitários.

### Saída
- `src/output.rs` — escritor NDJSON com tratamento de broken-pipe (SIGPIPE-safe)
- `src/ndjson_types.rs` — definições de tipos de saída com derivação de JSON Schema via schemars
- `src/constants.rs` — constantes nomeadas para tamanhos de buffer, thresholds e códigos de saída

### Utilitários
- `src/binary_detect.rs` — heurística de byte nulo para detecção de conteúdo binário
- `src/line_endings.rs` — detecção e normalização de LF/CRLF/CR
- `src/lang_utils.rs` — inicialização de locale e helpers i18n para rust-i18n
- `src/xattr_restore.rs` — salvar e restaurar xattrs (quarantine do macOS, selinux/capabilities do Linux)
- `src/reflink.rs` — helper de reflink (copy-on-write) via `reflink-copy`

### Handlers de Subcomando
- `src/commands/` — 28 implementações de subcomando, cada uma em seu próprio módulo
- Cada handler recebe args parseados, config global, escritor NDJSON e sinal de shutdown
- Todos os handlers seguem a mesma assinatura: `fn cmd_*(args, global, writer, shutdown) -> Result<()>`
- **Baseline v0.1.11 (22)**: read, write, edit, search, replace, hash, delete, count, diff, move, copy, list, extract, calc, regex, transform, scope, batch, backup, rollback, apply, completions
- **Adicionados em v0.1.12 (6)**: set, get, del, case, query, outline


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
  write/edit ──> [se WAL habilitada] ──> wal.rs (entrada Started)
                          │
                          └──> [após rename] ──> wal.rs (entrada Committed)
  query/outline ──> parse tree-sitter ──> DFS iterativo ──> eventos NDJSON
  set/get/del/case ──> toml_edit / heck ──> eventos NDJSON
```


## Decisões Chave

### BLAKE3 ao invés de SHA-256
- BLAKE3 é 5-14x mais rápido que SHA-256 para checksums de arquivo
- Performance single-threaded importa para latência de CLI
- Não usado para segurança criptográfica, apenas detecção de integridade

### NDJSON ao invés de JSON Array
- Streaming: cada resultado é emitido assim que computado
- Sem necessidade de bufferizar a saída inteira antes de escrever
- Pipe-friendly: ferramentas downstream processam linha por linha
- Erros emitem no mesmo formato com discriminador `error: true`

### tempfile + rename ao invés de In-Place Write
- Atômico: o arquivo alvo nunca fica meio escrito
- Sobrevive a queda de energia, OOM kill, SIGKILL
- Backup do original é uma cópia (não hardlink) para evitar corrupção de inode compartilhado
- **Fallback in-place (v0.1.12, G55)**: quando `nlink > 1` (Unix) ou alvo é um symlink, atomwrite troca para `ftruncate(0) + write_all + fsync_data` para preservar o inode. NDJSON ganha `write_strategy: "rename" | "inplace" | "copyback"`.

### Jail do Workspace
- Todos os caminhos validados contra a raiz do --workspace
- Previne path traversal via componentes `..`
- Bloqueia symlinks apontando para fora do workspace
- Rejeita FIFO e arquivos de dispositivo (não-atômicos por natureza)

### Tratamento de Signal via signal-hook
- SIGINT e SIGTERM definem flag atômico para shutdown cooperativo
- Segundo signal força exit imediato via process::exit
- SIGPIPE resetado para disposição default para comportamento Unix padrão de pipe
- Singleton compartilhado ShutdownSignal (v0.1.11) para que polling e main-thread is_shutdown() vejam a mesma flag

### G72 verificação de sintaxe REAL via tree-sitter (v0.1.12)
- Substitui a heurística de balanceamento de colchetes do v0.1.11 que tinha falsos positivos (indentação Python, template literals JS) e falsos negativos (`import` Python de módulo ausente)
- Usa `tree-sitter-language-pack` com features `download` + `dynamic-loading`
- 24 linguagens cobertas out-of-the-box; extensões desconhecidas fazem fallback para heurística legada
- Exit 88 com primeira linha/coluna/kind/mensagem de erro

### G114 sidecar WAL para recuperação de crash (v0.1.12)
- Caminho do sidecar: `.atomwrite.journal.<target>.atomwrite.journal.json`
- Anexa entradas `Started` (op_id, expected_new_checksum, pid, started_at_unix) e `Committed` (op_id, committed_at_unix)
- `recover_orphan_journals(dir)` é **consultivo** — lê sidecars e reporta órfãos sem tocar no FS
- Caller decide se deve fazer replay, abortar ou ignorar

### tree-sitter-language-pack com dynamic-loading (v0.1.12, ADR-0019)
- Parsers NÃO são bundled (seria 1+ GB)
- Baixados no primeiro uso, cacheados localmente em `~/.cache/tree-sitter-language-pack/parsers/`
- Footprint da instalação fica em torno de 5-10 MB
- Alternativa: `tree-sitter` como dep direta, mas adiciona 305 crates de parser ao tempo de compilação

### Arquitetura v14 Tier 3 (v0.1.12)
- `set/get/del` usam `toml_edit` (preserva comentários e ordem das chaves) para TOML e `serde_json` (canônico) para JSON
- `get/del` usam descida manual de `Table` via helpers `get_toml_path` e `remove_toml_path` (ADR-0024) em vez de `toml_edit::Document::get` que trata chaves dotted como literais
- `case` usa crate `heck` para 5 estilos de identifier-case
- `query/outline` usam DFS iterativo via pilha `Vec<Node>` para evitar stack overflow em arquivos profundos (em comparação com travessia recursiva via `TreeCursor`)

### Internacionalização
- Traduções embedded em tempo de compilação via rust-i18n
- Detecção de locale via sys-locale no startup
- Locales suportados: en (fallback default), pt-BR
- Override via flag `--lang` ou env var `ATOMWRITE_LANG`
- Precedência: flag --lang, env ATOMWRITE_LANG, locale do SO, fallback en
- stdout NDJSON NÃO é traduzido (contrato legível por máquina)
- Apenas mensagens stderr e sugestões de erro são locale-aware


## Estratégia de Erro
- Enum `AtomwriteError` com derives Display via thiserror
- Cada variante mapeia para um código de saída compatível com sysexits
- Classificação de erro: permanent, transient, conflict, precondition_failed
- Erros transient e conflict são marcados como retryable para loops de retry de agentes
- Todos os erros serializam para NDJSON no stdout com campos legíveis por máquina
- Campo `suggestion` em `ErrorJson` fornece orientação de recuperação acionável para cada variante de erro
- Struct `ErrorContext` (adicionado em v0.1.4) carrega `workspace_provided: bool` e `workspace: Option<PathBuf>` do parser CLI para a saída de erro
- `ErrorJson::from_error_with_context(err, &ErrorContext)` produz sugestões context-aware
- Sugestão de `WorkspaceJail` se adapta com base em se o usuário forneceu `--workspace` ou `ATOMWRITE_WORKSPACE`
- Legacy `ErrorJson::from_error(err)` delega para `from_error_with_context` com `ErrorContext::default()` (compatibilidade retroativa)
- 25 variantes de erro no total (20 baseline de v0.1.4 + 5 adicionadas em v0.1.12: `LockTimeout` 83, `SyntaxError` 88, `ExdevFallbackDisabled` 91, `CopyBackBlake3Failed` 92, `OrphanJournal` 93)


## Architecture Decision Records (ADRs)
- Veja `docs/decisions/README.md` para o índice completo de ADRs
- 7 ADRs foram adicionados em v0.1.12 (0019-0025), todos seguindo o formato Michael Nygard (Status, Context, Decision, Consequences, Alternatives, Trigger to revisit)
- 0019 — escolha de tree-sitter-language-pack
- 0020 — path do WAL sidecar e shape JSONL
- 0021 — v14 query/outline aceita apenas kind names, não S-expressions
- 0022 — G72 tree-sitter substitui heurística
- 0023 — G114 WAL é consultivo, não auto-replay
- 0024 — get/del TOML path usa descida manual de Table
- 0025 — positions é opt-in em query/tree apenas


## Arquitetura de Testes
- 445 testes em 43 suites de teste de integração + 150+ testes unitários dentro de `src/`
- Testes unitários são colocalizados com o código sob módulos `#[cfg(test)]`
- Testes de integração vivem em `tests/` e usam `assert_cmd` + `predicates` para testes shell-out
- Testes property-based via `proptest` para checksum e backup
- Gate de cross-compile via `tests/cross_compile_check.rs`
- Testes de snapshot via `insta` para saída NDJSON estável
