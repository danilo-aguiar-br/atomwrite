# Guia de Instalação

- Instruções completas para instalar atomwrite em Linux, macOS e Windows
- Versão alvo atual: v0.1.4 (corrige compilação Windows 10/11 e adiciona sugestões de erro context-aware)
- Seções ordenadas por plataforma, com pré-requisitos e solução de problemas


## Linux

### Instalação Rápida (Ubuntu/Debian)

```bash
# Instalar Rust 1.85+ via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Instalar atomwrite v0.1.4 do crates.io
cargo install atomwrite --locked --version "^0.1.4"

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
# Instalar Rust 1.85+ via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Instalar atomwrite
cargo install atomwrite --locked

# Liberar no Gatekeeper se solicitado
xattr -d com.apple.quarantine $(which atomwrite) 2>/dev/null || true
```


## Windows 10 / Windows 11

### Pré-Requisitos

1. **Rust 1.85 ou posterior** — instalar via [rustup.rs](https://rustup.rs)
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

Este era um bug em v0.1.3 (GAP 14) corrigido em v0.1.4. Certifique-se de
instalar v0.1.4 ou posterior:

```powershell
cargo install atomwrite --locked --version "^0.1.4"
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
`#[cfg(windows)]`, conforme o fix do GAP 14 em v0.1.4.
