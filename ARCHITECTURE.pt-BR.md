[Read in English](ARCHITECTURE.md)


# Arquitetura


## Visão Geral
- atomwrite é um binário Rust único para operações atômicas de arquivo
- Projetado para agentes LLM que precisam de manipulação segura e estruturada de arquivos
- Toda escrita é atômica: tempfile, fsync, rename, fsync do diretório
- Toda resposta é NDJSON no stdout com checksums BLAKE3
- Todos os logs vão para stderr via tracing


## Mapa de Módulos

### Ponto de Entrada
- `src/main.rs` -- entrada do binário: setup de sinais, init do tracing, dispatch
- `src/lib.rs` -- raiz da biblioteca: declarações de módulos e dispatcher `run()`
- `src/cli.rs` -- clap `#[derive(Parser)]` com flags globais
- `src/cli_args.rs` -- structs de argumentos por subcomando e value enums

### Pipeline Core
- `src/atomic.rs` -- pipeline de escrita atômica: tempfile + fsync + rename + fsync dir
- `src/checksum.rs` -- computação de hash BLAKE3 para arquivos e slices de bytes (usa memmap2 para arquivos grandes)
- `src/file_io.rs` -- leitura inteligente de arquivos com memmap2 automático acima de 1 MiB
- `src/platform.rs` -- fsync específico de plataforma: F_FULLFSYNC no macOS via libc::fcntl

### Segurança e Validação
- `src/path_safety.rs` -- jail de workspace: prevenção de path traversal, validação de symlinks, detecção de FIFO/device
- `src/signal.rs` -- tratamento de SIGINT/SIGTERM via signal-hook com coordenação de shutdown graceful
- `src/error.rs` -- enum de erros de domínio com exit codes, classificação e flag retryable

### Saída
- `src/output.rs` -- writer NDJSON com tratamento de broken-pipe (SIGPIPE-safe)
- `src/ndjson_types.rs` -- definições de tipos de saída com derivação de JSON Schema via schemars
- `src/constants.rs` -- constantes nomeadas para tamanhos de buffer, thresholds e exit codes

### Utilitários
- `src/binary_detect.rs` -- heurística de null-byte para detecção de conteúdo binário
- `src/line_endings.rs` -- detecção e normalização de LF/CRLF/CR

### Handlers de Subcomandos
- `src/commands/` -- 22 implementações de subcomandos, cada um em seu módulo
- Cada handler recebe args parseados, config global, um writer NDJSON e sinal de shutdown
- Todos seguem a mesma assinatura: `fn cmd_*(args, global, writer, shutdown) -> Result<()>`


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
             ├── [search/replace] ──> WalkParallel ──> engine ripgrep
             │                              │
             │                       canal crossbeam
             │                              │
             │                        eventos NDJSON
             │
             └── [read/hash/list] ──> ops diretas de fs ──> eventos NDJSON
                                                              │
                                                         stdout (NDJSON)
```


## Decisões Chave

### BLAKE3 em vez de SHA-256
- BLAKE3 é 5-14x mais rápido que SHA-256 para checksums de arquivo
- Performance single-threaded importa para latência de CLI
- Não usado para segurança criptográfica, apenas detecção de integridade

### NDJSON em vez de JSON Array
- Streaming: cada resultado é emitido assim que computado
- Sem necessidade de bufferizar toda a saída antes de escrever
- Pipe-friendly: ferramentas downstream processam linha por linha
- Erros emitem no mesmo formato com discriminador `error: true`

### tempfile + rename em vez de Escrita In-Place
- Atômico: arquivo alvo nunca fica meio escrito
- Sobrevive a queda de energia, OOM kill, SIGKILL
- Backup do original é uma cópia (não hardlink) para evitar corrupção de inode compartilhado

### Jail de Workspace
- Todos os paths validados contra raiz do --workspace
- Previne path traversal via componentes `..`
- Bloqueia symlinks apontando para fora do workspace
- Rejeita FIFOs e arquivos de dispositivo (não-atômicos por natureza)

### Tratamento de Sinais via signal-hook
- SIGINT e SIGTERM setam flag atômica para shutdown cooperativo
- Segundo sinal força saída imediata via process::exit
- SIGPIPE resetado para disposição padrão para comportamento Unix standard de pipe


## Internacionalização
- Traduções embutidas em tempo de compilação via rust-i18n
- Detecção de locale via sys-locale na inicialização
- Locales suportados: en (fallback padrão), pt-BR
- Override via flag `--lang` ou variável de ambiente `ATOMWRITE_LANG`
- Precedência: flag --lang, env ATOMWRITE_LANG, locale do SO, fallback en
- NDJSON no stdout NÃO é traduzido (contrato machine-readable)
- Apenas mensagens stderr e sugestões de erro são sensíveis ao locale


## Estratégia de Erros
- Enum `AtomwriteError` com thiserror derivando Display
- Cada variante mapeia para um exit code compatível com sysexits
- Classificação de erro: permanent, transient, conflict, precondition_failed
- Erros transient e conflict são marcados retryable para loops de retry de agentes
- Todos os erros serializam para NDJSON no stdout com campos machine-readable
