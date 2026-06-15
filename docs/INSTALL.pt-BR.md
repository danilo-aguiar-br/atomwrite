# Guia de Instalação

- Instruções completas para instalar atomwrite em Linux, macOS e Windows
- Versão alvo atual: v0.1.12 (corrige compilação Windows 10/11, adiciona sugestões de erro context-aware, 6 novos subcomandos, G72 verificação de sintaxe real, G114 sidecar WAL, 5 novos códigos de erro, reflink copy, fallback EXDEV)
- Seções ordenadas por plataforma, com pré-requisitos e solução de problemas


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças relevantes para instalação em v0.1.12.

### Instalação (Linux/macOS/Windows)

A release v0.1.12 é um substituto drop-in para v0.1.4. Instale com:

```bash
cargo install atomwrite --locked --version "^0.1.12"
```

O fix do Windows 10/11 de v0.1.4 é preservado (cargo install agora funciona). v0.1.12 adiciona:

- 6 novos subcomandos (`set`, `get`, `del`, `case`, `query`, `outline`) via implementação Tier 3 v14
- G72 verificação de sintaxe REAL via tree-sitter (24 linguagens via `tree-sitter-language-pack`)
- G114 sidecar WAL para recuperação de crash (consultivo, sem auto-replay)
- 5 novos códigos de erro (83 LockTimeout, 88 SyntaxError, 91 ExdevFallbackDisabled, 92 CopyBackBlake3Failed, 93 OrphanJournal)

### Nova Dependência

- `tree-sitter-language-pack = "1.8"` com features `download` + `dynamic-loading`
- Parsers baixam no primeiro uso (~5-10MB footprint total, não empacotados)
- Sem `cargo build` manual ou compilação de fonte necessária para suporte a linguagens

### Notas Específicas do Windows

- v0.1.12 preserva o fix do Windows da v0.1.4: `cargo install atomwrite` funciona no Windows 10/11.
- Melhorias no `init_console` (de v0.1.4) garantem que UTF-8 e sequências ANSI funcionem no Windows Terminal e PowerShell 7+.
- `persist_with_retry` lida com `PermissionDenied` durante rename atômico com backoff exponencial (compensação do Windows Defender).

### Notas Específicas do Linux

- v0.1.12 requer Rust 1.88 ou posterior (igual a v0.1.4).
- `cargo install atomwrite` do crates.io é o caminho de instalação recomendado.
- Sem dependências de nível de sistema além do toolchain Rust padrão.

### Notas Específicas do macOS

- v0.1.12 preserva os fixes de build do macOS arm64 (Apple Silicon) e macOS x86_64 da v0.1.2.
- Gatekeeper pode exigir `xattr -d com.apple.quarantine $(which atomwrite)` no primeiro uso.
- `posix_fadvise` é corretamente gateado a `cfg(target_os = "linux")` apenas (no-op no macOS).

### Cobertura de Testes

- 542 testes passando (445 na v0.1.12 + 2 na v0.1.14 + 8 G117 + 6 G118 na v0.1.15)
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas em `docs/schemas/`
- Veja [docs/decisions/README.md](README.md) para decisões arquiteturais

## Linux

### Instalação Rápida (Ubuntu/Debian)

```bash
# Instalar Rust 1.88+ via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Instalar atomwrite v0.1.18 do crates.io
cargo install atomwrite --locked --version "^0.1.12"

# Verificar
atomwrite --version
```

### Instalação Rápida (Fedora/RHEL)

```bash
# Instalar ferramentas de build
sudo dnf install rust cargo gcc

# Instalar atomwrite
cargo install atomwrite --locked
```

### Instalação Rápida (Arch)

```bash
sudo pacman -S rust
cargo install atomwrite --locked
```


## macOS

### Instalação Rápida

```bash
# Instalar Rust 1.88+ via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Instalar atomwrite
cargo install atomwrite --locked

# Liberar no Gatekeeper se solicitado
xattr -d com.apple.quarantine $(which atomwrite) 2>/dev/null || true
```


## Windows 10 / Windows 11

### Pré-Requisitos

1. **Rust 1.88 ou posterior** — instalar via [rustup.rs](https://rustup.rs)
2. **Visual Studio Build Tools 2019 ou posterior** com o workload "Desenvolvimento para desktop com C++" — necessário para linkagem. Baixar de [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
3. **Windows Terminal** ou **PowerShell 7+** (Windows Terminal recomendado para renderização UTF-8)
4. **Git for Windows** se instalar do código fonte

### Instalação Rápida (do crates.io)

```powershell
# Abrir PowerShell 7+ ou Windows Terminal
rustup default stable
rustup target add x86_64-pc-windows-msvc

# Instalar atomwrite
cargo install atomwrite --locked

# Verificar (esperar saída NDJSON)
atomwrite --version
```

### Instalação Rápida (do código fonte)

```powershell
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo install --path . --locked
```

### Solução de Problemas

#### "error: linker 'link.exe' not found"

Instale Visual Studio Build Tools com o workload C++. O instalador está em
<https://visualstudio.microsoft.com/visual-cpp-build-tools/>. Após instalar,
reinicie o terminal para que o PATH atualizado seja carregado.

#### "error[E0433]: failed to resolve: use of undeclared type `AtomwriteError`"

Este bug foi corrigido em v0.1.4. v0.1.12 é um substituto drop-in que também inclui 6 novos subcomandos, G72 verificação de sintaxe real, G114 sidecar WAL, e 5 novos códigos de erro. Certifique-se de
instalar v0.1.12 ou posterior:

```powershell
cargo install atomwrite --locked --version "^0.1.12"
```

Se não puder atualizar, compile do código fonte com o fix aplicado.

#### "error: linking with `link.exe` failed: exit code: 1125"

Antivírus (Windows Defender, McAfee, etc.) está bloqueando o linker. Adicione
exceção para `%USERPROFILE%\.cargo` e `%USERPROFILE%\.rustup`, depois tente
novamente.

#### Mojibake na saída do console

Console legado do Windows (`cmd.exe`) não suporta UTF-8 por padrão. Use
**Windows Terminal** (que ativa UTF-8 automaticamente) ou execute:

```powershell
chcp 65001
```

antes de invocar atomwrite. A função `init_console` do atomwrite também ativa
UTF-8 e sequências ANSI nos handles do console Windows quando executada de
um terminal configurado adequadamente.

#### "operation not permitted" durante escrita atômica

O pipeline de escrita atômica tenta até 3 vezes em caso de `PermissionDenied`.
Se um arquivo está aberto por outro processo (ex.: editor sem liberação
adequada, scanner antivírus, busca indexada), as tentativas podem esgotar.
Feche o arquivo na aplicação que o segura e tente novamente.


## Instalando Versões Específicas

```bash
# Mais recente
cargo install atomwrite --locked

# Versão específica
cargo install atomwrite --locked --version 0.1.4

# Forçar reinstalação
cargo install atomwrite --locked --force
```

A flag `--locked` garante que `Cargo.lock` é respeitado, assegurando build
reproduzível idêntico ao testado pelos mantenedores.


## Verificando Instalação

```bash
atomwrite --version
# Esperar saída NDJSON com versão, data de build e info de plataforma
```

Se o comando não for encontrado, garanta que `~/.cargo/bin` (Linux/macOS)
ou `%USERPROFILE%\.cargo\bin` (Windows) está no seu `PATH`.


## Compilando do Código Fonte (todas plataformas)


## Verificação de Saúde (G119)

Após instalar v0.1.15 ou posterior, execute os comandos de saúde do WAL para confirmar que os novos subcomandos estão cabeados corretamente. Estas operações são read-only ou de reparo escopado, seguras para uso em CI e smoke tests pós-instalação.

### Inspecionar Estado do WAL (read-only)

```bash
atomwrite --workspace . wal-stats
```

Envelope NDJSON esperado (truncado):

```json
{"type":"result","journals_total":0,"journals_started":0,"journals_committed":0,"journals_aborted":0,"stale_threshold_secs":86400,"reclaimable":0,"action":"stats","elapsed_ms":3}
```

O campo `reclaimable` conta journals terminais (Committed ou Aborted) mais antigos que o threshold de obsolescência. Um valor não-zero indica sidecars órfãos elegíveis para limpeza segura.

### Reap de Journals Terminais

A camada G119 L3 adiciona `wal-heal` para remover journals terminais (Committed e Aborted). Nunca toca em entradas Started.

```bash
# Remover todos os journals terminais independentemente da idade (seguro: ignora Started)
atomwrite --workspace . wal-heal --threshold-secs 0
```

Use este comando em smoke tests pós-instalação, hooks de pre-build em CI, ou após uma varredura de recuperação de crash.

### Recomendação para CI

Adicione uma checagem de `wal-stats` ao seu pipeline de CI antes de `cargo test`. Um `reclaimable` não-zero sinaliza acumulação de sidecars que deve ser inspecionada ou healada.

```bash
# Higiene pré-build em CI
atomwrite --workspace . wal-stats | jaq -e '.reclaimable == 0' || { echo "drift de WAL detectado"; exit 1; }
```


```bash
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo build --release
./target/release/atomwrite --version
```

O binário de release fica em `target/release/atomwrite` (ou `atomwrite.exe`
no Windows).


## Validação Cross-Compile (para contribuidores)

O projeto inclui um gate de cross-compile para detectar erros de compilação
exclusivos de Windows antes do release. Execute:

```bash
# Instalar targets Windows
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-pc-windows-msvc

# No Linux, instalar mingw-w64 para o target GNU
sudo dnf install mingw64-gcc mingw64-gcc-c++  # Fedora
sudo apt install mingw-w64                     # Ubuntu

# Executar gate de cross-compile
cargo test --test cross_compile_check -- --ignored
```

Isto é obrigatório para qualquer release que toque em caminhos de código
`#[cfg(windows)]`, conforme o fix do GAP 14 em v0.1.4 (preservado para referência histórica; instale v0.1.12 ou posterior).


## v0.1.20 — Novidades

Esta release introduz uma nova camada de segurança chamada **intention guards** e renomeia a flag global `--lang` para `--locale` para desambiguar do seletor tree-sitter `--lang` usado por `scope` e `transform`.

### Intention Guards (5 flags OPT-IN)

- `--require-backup <N>` — recusa a operação quando menos de `N` backups retidos existem para o alvo
- `--confirm` — emite um prompt de confirmação listando a mutação planejada em NDJSON antes de executar
- `--auto-rotate <N>` — rotaciona automaticamente o anel de backups para `N` entradas após uma escrita bem-sucedida
- `--risk-threshold <LOW|MEDIUM|HIGH>` — bloqueia operações cujo risco classificado atinge ou excede o threshold
- `--locale <en|pt-BR>` — renomeado de `--lang` para desambiguar do `--lang` tree-sitter

### Outras Adições

- `count --by-size` — lista os maiores arquivos da árvore com tamanhos e contagem de linhas
- `read --mode raw|envelope` — seleciona entre saída byte-stream e envelope NDJSON estruturado
- `search --no-begin-end` — desabilita a decoração implícita de âncoras `^` e `$` na saída regex
- `write --preserve-timestamps` — preserva o mtime do arquivo fonte ao sobrescrever
- `scope --lang rust` — alias explícito aceito para simetria ergonômica com `transform --lang`

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

# Ou via ruplacer
ruplacer --subvert --lang --locale
```
