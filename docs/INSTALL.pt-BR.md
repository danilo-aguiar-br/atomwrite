# Guia de Instalação


[Read in English](INSTALL.md)

- Instruções completas para instalar atomwrite em Linux, macOS e Windows
- Versão alvo atual: v0.1.37 (avaliação de risco em todo verbo que grava; exit 69 `GRAMMAR_UNAVAILABLE` tipado; `write.content_risk_patterns` acessível por `config`; `[log]` do workspace respeitado; 47 subcomandos; builds slim/full)
- Seções ordenadas por plataforma, com pré-requisitos e solução de problemas


## O Que Há de Novo na v0.1.37
- Avaliação de risco: `edit`, `edit-loop`, `replace`, `case`, `transform` e `codemod` publicam `risk_assessment` quando o conteúdo casa um padrão de risco, o mesmo campo que o `write` já tinha
- Novo status de saída `69` `GRAMMAR_UNAVAILABLE` com classe `precondition_failed`: gramática ausente que não pode ser baixada nomeia o idioma em vez de reportar exit 255 `INTERNAL_ERROR`
- `write.content_risk_patterns` é valor de lista em `config get` / `config set`, então os padrões de risco são configuração XDG; a tabela tem 37 chaves
- `log.file` e `log.rotation` passam a ser aceitos também no escopo workspace, e a seção `[log]` do `.atomwrite.toml` agora liga o destino de log
- `batch --transaction` publica um veredito só: o evento `rollback` carrega `ok: false` com o `code` e o `exit` da falha que o causou
- O `atomwrite doctor` publica o check `grammar_cache` com quantas das 23 gramáticas que os verbos de AST podem precisar já estão em cache


## O Que Há de Novo na v0.1.37: Contrato de Agente
- Contrato de agente residual: `scratch init`, jail de content-source, globais `--select`/`--fields`, `--filter`, `--limit` (só long; sem `-n` global)
- Envelopes de erro sempre emitem (`ok:false`); `--filter` não silencia falhas
- SSOT: 47 subcomandos — `atomwrite agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`
- Verbo novo `config` (`path|list|get|set|unset`, 37 chaves, `--scope global|workspace` obrigatório em `path`, `set` e `unset`); `cargo xtask check-features` confere os conjuntos de features slim
- `atomwrite --version` imprime `atomwrite 0.1.37 (no-git) <target triple>` naquele release (a fonte é `no-git` quando nenhum repositório git rastreia este crate, senão o commit curto). Desde a v0.1.37 a string não carrega mais o sufixo `-dirty`, então um arquivo não rastreado não consegue mudá-la
- Pin `^0.1.37`. Suítes: `cargo test --test cli_v0137_config_verb --test cli_v0137_terminal_event --test cli_v0137_version`. Decisões: ADR-0058 em diante (índice em `docs/decisions/README.md`)
- O Rust mínimo é o 1.90 (`rust-version` no `Cargo.toml`, edition 2024). O `rust-toolchain.toml` do repositório fixa o canal 1.94 para contribuidores
- O subcomando `watch` não está no build padrão: acrescente `--features watch` ao `cargo install` (o binário padrão sai 78 com dica de rebuild)
- A feature `ast` baixa os parsers tree-sitter em tempo de execução no primeiro uso (veja Parsers Baixados em Tempo de Execução)
- Tamanhos de release medidos em 2026-10-31: slim 10,40 MiB e default 54,45 MiB (veja Builds Slim vs Full por Features)
- Novo neste guia: Arquivos e Diretórios (XDG), Completions de Shell, Páginas de Manual e Status de Verificação (o que não foi validado)


## 47 Subcomandos (SSOT)

Após instalar, descubra a superfície completa com:

```bash
atomwrite agent-surface
atomwrite commands --include-globals
atomwrite doctor --strict
```

Inventário compacto de todos os 47 nomes (`CANONICAL_SUBCOMMAND_COUNT`), cada um com um resumo de uma linha e um exemplo que foi executado contra o `atomwrite 0.1.37`:

- Preparação dos exemplos: `WS` é o caminho absoluto de um workspace vazio que contém `src/a.rs` (um `unwrap()` e um comentário), `src/b.rs` (a palavra `beta` uma vez), `src/c.rs` (com `user_id`), `src/tmp.rs`, `config.toml` (uma tabela `[package]` com `name` e uma tabela `[features]` com `x`), `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` e `rules.yaml` (uma lista YAML com `id`, `language`, `pattern` e `rewrite`)
- `HASH_A` é o `checksum` que `hash src/a.rs` imprime
- Os exemplos rodam na ordem abaixo: o `rollback` precisa do `backup` anterior e o `edit-loop` precisa do `edit` anterior
- Resultado: 46 dos 47 saem 0, e o `watch` sai 78 porque o build padrão não tem a feature `watch`

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
- `prune-backups` — Remove arquivos .bak antigos por idade ou quantidade. Exemplo: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run .`
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
- `config` — Lê e grava chaves de ajuste persistentes (37 chaves, escopos global e workspace). Exemplo: `atomwrite --workspace "$WS" config get fuzzy.threshold`


## O Que Há de Novo na v0.1.35
- Gaps A-* residuais; sobrescrita grande `--ack-overwrite`; watch_summary; markers semantic-merge ON por padrão
- Pin histórico `^0.1.35` (substituído por 0.1.36). Fuzzy one-shot / timeout 120 / exit 124 da 0.1.33–0.1.34 permanecem


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
- `tree-sitter-language-pack = "1.22.1"` com features `download` + `dynamic-loading`
- Parsers baixam no primeiro uso e não vêm empacotados; o download medido é de 23.031.807 bytes em Linux x86_64, o que dá 21,97 MiB (veja Parsers Baixados em Tempo de Execução)
- Sem `cargo build` manual ou compilação de fonte necessária para suporte a linguagens

### Notas Específicas do Windows
- v0.1.12 preserva o fix do Windows da v0.1.4: `cargo install atomwrite` funciona no Windows 10/11.
- Melhorias no `init_console` (de v0.1.4) garantem que UTF-8 e sequências ANSI funcionem no Windows Terminal e PowerShell 7+.
- `persist_with_retry` lida com `PermissionDenied` durante rename atômico com backoff exponencial (compensação do Windows Defender).

### Notas Específicas do Linux
- v0.1.12 exigia Rust 1.88 ou posterior (histórico). O mínimo atual é o Rust 1.90 (`rust-version` no `Cargo.toml`).
- `cargo install atomwrite` do crates.io é o caminho de instalação recomendado.
- Sem dependências de nível de sistema além do toolchain Rust padrão.

### Notas Específicas do macOS
- v0.1.12 preserva os fixes de build do macOS arm64 (Apple Silicon) e macOS x86_64 da v0.1.2.
- Gatekeeper pode exigir `xattr -d com.apple.quarantine $(which atomwrite)` no primeiro uso.
- `posix_fadvise` é corretamente gateado a `cfg(target_os = "linux")` apenas (no-op no macOS).

### Cobertura de Testes
- Atual (v0.1.37): 131 alvos de teste rodaram 1486 testes com 0 falhas e 9 ignorados (os testes `cross_compile_*`), conforme registrado no `CHANGELOG.md`
- ADRs em `docs/decisions/` (0058 em diante são as decisões da v0.1.37)
- Os schemas JSON em `docs/schemas/`, onde 54 arquivos descrevem os 55 nomes que `atomwrite --print-schema` lista
- Veja [docs/decisions/README.md](decisions/README.md) para decisões arquiteturais


## Linux

### Instalação Rápida (Ubuntu/Debian)

```bash
# Instalar Rust 1.90+ via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Instalar atomwrite v0.1.37 do crates.io
cargo install atomwrite --locked --version "^0.1.37"
# Ou a partir de um clone local:
# cargo install --path . --locked --force

# Verificar
atomwrite --version
```

### Instalação Rápida (Fedora/RHEL)

```bash
# Instalar ferramentas de build (o Rust da distribuição precisa ser 1.90 ou mais novo; confira `rustc --version`, senão use rustup como acima)
sudo dnf install rust cargo gcc

# Instalar atomwrite v0.1.37 do crates.io
cargo install atomwrite --locked --version "^0.1.37"
# Ou a partir de um clone local:
# cargo install --path . --locked --force
```

### Instalação Rápida (Arch)

```bash
# O Rust da distribuição precisa ser 1.90 ou mais novo; confira `rustc --version`, senão use rustup como acima
sudo pacman -S rust

# Instalar atomwrite v0.1.37 do crates.io
cargo install atomwrite --locked --version "^0.1.37"
# Ou a partir de um clone local:
# cargo install --path . --locked --force
```


## macOS

### Instalação Rápida

```bash
# Instalar Rust 1.90+ via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Instalar atomwrite v0.1.37 do crates.io
cargo install atomwrite --locked --version "^0.1.37"
# Ou a partir de um clone local:
# cargo install --path . --locked --force

# Liberar no Gatekeeper se solicitado
xattr -d com.apple.quarantine $(which atomwrite) 2>/dev/null || true
```


## Windows 10 / Windows 11

### Pré-Requisitos

1. Rust 1.90 ou posterior — instalar via [rustup.rs](https://rustup.rs)
2. Visual Studio Build Tools 2019 ou posterior com o workload "Desenvolvimento para desktop com C++" — necessário para linkagem. Baixar de [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
3. Windows Terminal ou PowerShell 7+ (Windows Terminal recomendado para renderização UTF-8)
4. Git for Windows se instalar do código fonte

### Instalação Rápida (do código fonte — preferido)

```powershell
git clone https://github.com/danilo-aguiar-br/atomwrite.git
cd atomwrite
cargo install --path . --locked --force
```

### Instalação Rápida (do crates.io)

```powershell
# Abrir PowerShell 7+ ou Windows Terminal
rustup default stable
rustup target add x86_64-pc-windows-msvc

# Instalar atomwrite v0.1.37 do crates.io
cargo install atomwrite --locked --version "^0.1.37"

# Verificar (esperar uma linha de texto: atomwrite 0.1.37 (no-git) x86_64-pc-windows-msvc)
atomwrite --version
```

### Solução de Problemas

### "error: linker 'link.exe' not found"

Instale Visual Studio Build Tools com o workload C++. O instalador está em
<https://visualstudio.microsoft.com/visual-cpp-build-tools/>. Após instalar,
reinicie o terminal para que o PATH atualizado seja carregado.

### "error[E0433]: failed to resolve: use of undeclared type `AtomwriteError`"

Este bug foi corrigido em v0.1.4. v0.1.12 é um substituto drop-in que também inclui 6 novos subcomandos, G72 verificação de sintaxe real, G114 sidecar WAL, e 5 novos códigos de erro. Certifique-se de
instalar v0.1.12 ou posterior:

```powershell
cargo install atomwrite --locked --version "^0.1.12"
```

Se não puder atualizar, compile do código fonte com o fix aplicado.

### "error: linking with `link.exe` failed: exit code: 1125"

Antivírus (Windows Defender, McAfee, etc.) está bloqueando o linker. Adicione
exceção para `%USERPROFILE%\.cargo` e `%USERPROFILE%\.rustup`, depois tente
novamente.

### Mojibake na saída do console

Console legado do Windows (`cmd.exe`) não suporta UTF-8 por padrão. Use
Windows Terminal (que ativa UTF-8 automaticamente) ou execute:

```powershell
chcp 65001
```

antes de invocar atomwrite. A função `init_console` do atomwrite também ativa
UTF-8 e sequências ANSI nos handles do console Windows quando executada de
um terminal configurado adequadamente.

### "operation not permitted" durante escrita atômica

O pipeline de escrita atômica repete `PermissionDenied` com full jitter (base 50 ms, teto 400 ms, as chaves `retry.*` do `atomwrite config`) até o orçamento de retry ou o `--timeout-secs` acabar.
Se um arquivo está aberto por outro processo (ex.: editor sem liberação
adequada, scanner antivírus, busca indexada), as tentativas podem esgotar.
Feche o arquivo na aplicação que o segura e tente novamente.


## Instalando Versões Específicas

```bash
# Recomendado: pin do crates.io para v0.1.37
cargo install atomwrite --locked --version "^0.1.37"

# A partir de um clone / árvore local
cargo install --path . --locked --force

# Pin histórico de exemplo (0.1.28): versão sem operador instala exatamente essa versão
cargo install atomwrite --locked --version 0.1.28

# Forçar reinstalação do pin do crates.io
cargo install atomwrite --locked --version "^0.1.37" --force

# Voltar para uma versão exata (um requisito ^0.1.36 resolveria de novo para a 0.1.37)
cargo install atomwrite --locked --version 0.1.36 --force
```

A flag `--locked` garante que o `Cargo.lock` seja respeitado, garantindo um build
reproduzível que corresponde ao que os maintainers testaram.


## Verificando Instalação

```bash
atomwrite --version
# Esperar uma linha de texto: atomwrite 0.1.37 (no-git) <target triple>
# A fonte é no-git quando nenhum repositório git rastreia o crate, senão o commit curto
```

Se o comando não for encontrado, garanta que `~/.cargo/bin` (Linux/macOS)
ou `%USERPROFILE%\.cargo\bin` (Windows) está no seu `PATH`.


## Compilando do Código Fonte (todas plataformas)

```bash
git clone https://github.com/danilo-aguiar-br/atomwrite.git
cd atomwrite
cargo build --release
./target/release/atomwrite --version
```

O binário de release fica em `target/release/atomwrite` (ou `atomwrite.exe`
no Windows).


## Builds Slim vs Full por Features (v0.1.37)

A v0.1.37 expõe features do Cargo para trocar tamanho AST por um binário slim de agente.

| Perfil | Comando | Tamanho aprox. | Notas |
|---|---|---|---|
| slim (`core` apenas) | `cargo install atomwrite --locked --version "^0.1.37" --no-default-features --features core` | 10,40 MiB | size-gate local ≤15 MiB |
| default (subconjunto AST) | `cargo install atomwrite --locked --version "^0.1.37"` | 54,45 MiB | `core` + `ast` + `lang-rust` + `lang-ts` + `lang-py` |
| default mais `watch` | `cargo install atomwrite --locked --version "^0.1.37" --features watch` | não medido | acrescenta o subcomando `watch` ao build padrão |
| `full` | `cargo install atomwrite --locked --version "^0.1.37" --features full` | não medido (maior que o default) | default + `lang-full` + `watch` |

Tamanhos medidos com `cargo build --release --locked` da v0.1.37 em 2026-10-31: slim 10.908.040 bytes (DERIVADO: 10908040 / 1048576 = 10,40 MiB) e default 57.096.064 bytes (DERIVADO: 57096064 / 1048576 = 54,45 MiB). O binário slim responde `atomwrite 0.1.37` e sai 78 no `outline`. Os perfis `watch` e `full` não foram medidos.

### Flags de feature
- `core` — read/write/edit/search/replace atômicos e demais comandos sem AST
- `ast` — stack tree-sitter / ast-grep (`transform`, `scope`, `query`, `outline`, `--syntax-check`)
- `lang-rust` / `lang-ts` / `lang-py` / `lang-go` — packs de linguagem para AST
- `lang-full` — todos os packs de linguagem acima
- `watch` — subcomando `watch` (`notify`)
- `full` — `default` + `lang-full` + `watch`
- `default` — `core` + `ast` + `lang-rust` + `lang-ts` + `lang-py` (`watch` é opt-in)
- `semantic-search` não tem gate de feature: é sempre compilado (Jaccard offline). A antiga feature `semantic` não existe mais
- `i18n-cjk`, `i18n-rtl`, `i18n-europe` e `i18n-full` são placeholders vazios, sem traduções por trás: não as ative
- `slow-tests` liga testes lentos para contribuidores e não tem efeito no binário

Comportamento do build slim:

- `outline`, `sparse outline`, `query`, `scope`, `transform` e `codemod` saem 78 (`CONFIG_INVALID`) com dica de rebuild, nunca um sucesso falso (medido num build release slim desta árvore, 10.348.424 bytes)
- O build padrão responde ao `watch` do mesmo jeito: exit 78 com `watch requires rebuild: cargo install atomwrite --features watch`
- `write --syntax-check` saiu 0 no build slim para um arquivo Rust válido, e se a verificação é pulada ali não foi examinado
- `cargo xtask check-features` roda `cargo check --locked --no-default-features` nos conjuntos `core` e `core,watch` e para na primeira falha. O `xtask` não está no pacote do crates.io, então rode a partir de um checkout do código-fonte

### Exemplos

```bash
# Instalação slim (agente)
cargo install atomwrite --locked --version "^0.1.37" --no-default-features --features core

# Instalação default/full com AST
cargo install atomwrite --locked --version "^0.1.37"

# Default mais o subcomando watch
cargo install atomwrite --locked --version "^0.1.37" --features watch

# Default mais watch e todas as linguagens
cargo install atomwrite --locked --version "^0.1.37" --features full

# Verificar instalação (47 subcomandos no --help)
atomwrite --version
atomwrite --help
```


## Parsers Baixados em Tempo de Execução
- O build padrão tem a feature `ast`. `outline`, `sparse outline`, `query` e `write --syntax-check` precisam de um parser tree-sitter, e os parsers NÃO vêm embutidos no binário
- Com o cache vazio, o primeiro desses comandos baixa o pacote de parsers da plataforma pelo `ureq`, que respeita as configurações de proxy do host. Medido: cada um dos quatro encheu um cache vazio, e `transform`, `codemod` e `scope` não tocaram nele
- Medido em Linux x86_64 com cache vazio em 2026-10-31: o pacote tem 23.031.807 bytes (DERIVADO: 23031807 / 1048576 = 21,97 MiB), as seis bibliotecas de parser baixadas para as linguagens daquela execução ocupam 3.991.360 bytes (DERIVADO: 3991360 / 1048576 = 3,81 MiB), e o diretório de cache soma 27.053.221 bytes (DERIVADO: 27053221 / 1048576 = 25,80 MiB)
- Origem: `github.com/xberg-io/tree-sitter-language-pack/releases/download/v1.22.1/parsers-linux-x86_64.tar.zst`, e o `manifest.json` da 1.22.1 lista 372 linguagens e um `sha256` para cada um dos 6 pacotes de plataforma
- Cache: `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` com `bundles`, `libs` e `manifest.json`. Ele pertence ao crate dos parsers e fica fora dos diretórios `atomwrite` da tabela abaixo
- As execuções seguintes leem o cache e não usam rede
- Para um host sem rede, rode `atomwrite --workspace "$WS" outline src/a.rs` uma vez num host conectado e copie o diretório do cache para o mesmo caminho no host offline
- Um build slim (`--no-default-features --features core`) não tem código de parser e não baixa nada
- O `human-panic` substitui a saída padrão de panic: quando o processo entra em panic, ele grava um relatório de falha como arquivo TOML local no diretório temporário (`report-<id>.toml`) e imprime o caminho. Nada é enviado para lugar nenhum


## Arquivos e Diretórios (XDG)
- O atomwrite não lê nenhuma variável de ambiente própria. Os caminhos seguem as convenções de plataforma do crate `directories`, e os crates `directories` e `sys-locale` leem `HOME`, `XDG_*` e `LANG` por conta própria (desvio consciente registrado no `CHANGELOG.md`)
- Precedência das configurações: flags de linha de comando, depois o `.atomwrite.toml` na raiz do workspace, depois o `config.toml` global, depois os padrões embutidos. Os dois arquivos são mesclados chave a chave
- Consulte e altere as configurações só pelo verbo: `atomwrite --workspace "$WS" config list`, `config get CHAVE` e `config set CHAVE VALOR --scope global|workspace` (37 chaves; `--scope` é obrigatório em `path`, `set` e `unset`)

| O quê | Linux (medido) | macOS (lido de `src/storage.rs`, não executado) | Windows (lido de `src/storage.rs`, não executado) |
|---|---|---|---|
| `config.toml` global (modo 0600 em Unix) | `$XDG_CONFIG_HOME/atomwrite/config.toml`, padrão `~/.config/atomwrite/config.toml` | `~/Library/Application Support/atomwrite/config.toml` | `%APPDATA%\atomwrite\config\config.toml` |
| Preferência de locale | `locale` no mesmo diretório | mesmo diretório | mesmo diretório |
| Arquivo de log, só com `[log] file = true` | `$XDG_STATE_HOME/atomwrite/logs/atomwrite.<data>.jsonl`, padrão `~/.local/state/atomwrite/logs` | sem diretório de state: usa o diretório de dados | sem diretório de state: usa o diretório de dados |
| Completions do `--install` | `$XDG_DATA_HOME/` mais o caminho do shell, padrão `~/.local/share/` | pai do diretório de dados, `~/Library/Application Support/` | pai do diretório de dados, `%APPDATA%\atomwrite\` |
| Cache de parsers | `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` | pertence ao crate dos parsers, não lido aqui | pertence ao crate dos parsers, não lido aqui |
| Configuração do workspace | `.atomwrite.toml` na raiz do workspace | igual | igual |

- O `storage.home` realoca os diretórios: `atomwrite config set storage.home DIR --scope global` faz o config global ser `DIR/config/config.toml`, e dados, cache e state vão para `DIR/data`, `DIR/cache` e `DIR/state`. O `config.toml` de bootstrap que guarda o `storage.home` fica no caminho padrão de config, o arquivo antigo não é movido, e `config unset storage.home --scope global` desfaz
- O escopo global aceita as 37 chaves; `log.file`, `log.rotation`, `storage.home` e `locale` são só globais, e o escopo workspace as rejeita com exit 65
- No macOS e no Windows os caminhos acima vêm da leitura do código e da documentação do `directories`, e não foram executados nesses sistemas


### Chaves de Config (37)
- `atomwrite config list` imprime as 37 chaves com `value`, `origin` e `default`; `origin` é `default`, `global`, `workspace` ou `explicit` (quando `--config CAMINHO` substitui os dois arquivos)
- `config list` e `config get CHAVE` não aceitam `--scope`; `config path`, `config set` e `config unset` exigem `--scope global|workspace` e saem 2 sem ele
- Valor fora da faixa aceita sai 65 `INVALID_INPUT` e não grava nada; chave desconhecida sai 65 e lista as válidas
- Os padrões e as faixas abaixo foram medidos na 0.1.37 com `config list` e `config set`

| Chave | Padrão | Valores aceitos | Escopos |
|---|---|---|---|
| `fuzzy.mode` | `auto` | `auto`, `aggressive`, `off` | global, workspace |
| `fuzzy.threshold` | 0,7 | 0 a 1 | global, workspace |
| `fuzzy.threshold_aggressive` | 0,5 | 0 a 1 | global, workspace |
| `fuzzy.threshold_context` | 0,8 | 0 a 1 | global, workspace |
| `fuzzy.threshold_jw` | 0,85 | 0 a 1 | global, workspace |
| `fuzzy.soft_floor_delta` | 0,05 | 0 a 1 | global, workspace |
| `fuzzy.max_content_bytes` | 268435456 | 1 a 9223372036854775807 | global, workspace |
| `write.confirm_large_bytes` | 102400 | 1 a 9223372036854775807 | global, workspace |
| `log.file` | `false` | `true`, `false` | global |
| `log.rotation` | `daily` | `daily`, `hourly` | global |
| `storage.home` | nenhum | um diretório | global |
| `locale` | nenhum | `en`, `pt-BR` | global |
| `defaults.backup` | `true` | `true`, `false` | global, workspace |
| `defaults.line_ending` | `auto` | `auto`, `lf`, `cr-lf`, `cr` | global, workspace |
| `defaults.max_filesize` | 1073741824 | inteiro | global, workspace |
| `defaults.retention` | 5 | inteiro | global, workspace |
| `search.context` | 0 | inteiro | global, workspace |
| `search.max_columns` | 500 | 1 ou mais | global, workspace |
| `search.max_filesize` | 10485760 | 1 ou mais | global, workspace |
| `search.smart_case` | `true` | `true`, `false` | global, workspace |
| `watch.debounce_ms` | 200 | inteiro | global, workspace |
| `watch.idle_exit_ms` | 500 | inteiro | global, workspace |
| `write.auto_rotate_max_age_secs` | 86400 | 1 ou mais | global, workspace |
| `write.content_risk_patterns` | os 14 padrões embutidos | lista de strings | global, workspace |
| `write.shrink_block_percent` | 50 | 1 a 99 | global, workspace |
| `concurrency.max_threads` | 16 | 1 a 256 | global, workspace |
| `concurrency.ram_fraction_percent` | 50 | 1 a 100 | global, workspace |
| `search.max_inflight_bytes` | 8388608 | 4194304 a 1073741824 | global, workspace |
| `summary.errors_max_entries` | 50 | 1 a 1000 | global, workspace |
| `lock.acquire_timeout_ms` | 5000 | 100 a 3600000 | global, workspace |
| `retry.base_ms` | 50 | 1 a 60000 (não acima de `retry.max_ms`) | global, workspace |
| `retry.max_ms` | 400 | 1 a 60000 | global, workspace |
| `retry.max_elapsed_ms` | 2000 | 1 a 3600000 | global, workspace |
| `extract.max_sample_rows` | 10 | 0 a 100000 | global, workspace |
| `extract.max_text_chars` | 400 | 1 a 10000000 | global, workspace |
| `extract.max_sheets` | 32 | 1 a 10000 | global, workspace |
| `extract.max_slides` | 64 | 1 a 10000 | global, workspace |

- O arquivo pode trazer mais campos que essas chaves (por exemplo `defaults.line_ending` e `defaults.max_filesize`, que são lidos mas não fazem nada, e campos de `[watch]`); o verbo `config` gerencia só as 37 chaves acima


## Completions de Shell
- `atomwrite completions SHELL` imprime o script no stdout para `bash`, `zsh`, `fish`, `powershell` ou `elvish`; o stdout é o próprio script e não NDJSON
- `atomwrite completions SHELL --install` grava o script sob o diretório de dados XDG e imprime um evento NDJSON, `{"type":"installed","path":"..."}`, com o arquivo gravado
- Arquivos gravados em Linux (medido com `XDG_DATA_HOME` definido), todos sob esse diretório: `bash-completion/completions/atomwrite`, `zsh/site-functions/_atomwrite`, `fish/vendor_completions.d/atomwrite.fish`, `elvish/lib/atomwrite.elv` e `powershell/Completions/_atomwrite.ps1`
- Exemplo: `atomwrite completions bash --install`, depois abra um shell novo (ou `source ~/.bashrc`)
- O Nushell não tem gerador no `clap_complete`: a CLI roda sob o Nushell com NDJSON no stdout, mas não há script de completion para ele


## Páginas de Manual
- As páginas de manual vêm das definições do clap: `atomwrite.1` mais uma página por subcomando (48 arquivos para os 47 subcomandos, medido)
- Elas NÃO fazem parte do pacote do crates.io, porque o crate `xtask` não está no tarball, e o `cargo install` não as instala
- Gere a partir de um checkout do código-fonte: `cargo xtask man --out-dir target/man` (o diretório padrão é `target/man`)
- Instale no Linux ou no macOS copiando os arquivos: `mkdir -p ~/.local/share/man/man1 && cp target/man/*.1 ~/.local/share/man/man1/`
- Depois rode `man atomwrite` ou `man atomwrite-edit`; o Windows não tem `man`, então leia `atomwrite --help` e `atomwrite SUBCOMANDO --help` lá
- `cargo xtask man` rodou na 0.1.37 e saiu 0; a cópia para `~/.local/share/man/man1` e a consulta com `man` não foram executadas


## Verificação de Saúde (G119)

Após instalar v0.1.15 ou posterior, execute os comandos de saúde do WAL para confirmar que os novos subcomandos estão cabeados corretamente. Estas operações são read-only ou de reparo escopado, seguras para smoke tests locais pós-instalação.

### Inspecionar Estado do WAL (read-only)

```bash
atomwrite --workspace . wal-stats
```

Envelope NDJSON esperado num workspace vazio (medido na 0.1.37):

```json
{"type":"wal_stats","total_journals":0,"by_state":{"started":0,"committed":0,"aborted":0,"malformed":0},"oldest_journal_age_secs":0,"total_size_bytes":0,"by_directory":[],"auto_heal_recommended":false,"estimated_reclaim_bytes":0,"ok":true}
```

Os campos `auto_heal_recommended` e `estimated_reclaim_bytes` apontam journals terminais (Committed ou Aborted) que o `wal-heal` pode remover. Um journal no estado `started` nunca é removido pelo `wal-heal`, porque pode pertencer a uma escrita que caiu: um `kill -9` ainda deixa um, por projeto.

### Reap de Journals Terminais

A camada G119 L3 adiciona `wal-heal` para remover journals terminais (Committed e Aborted). Nunca toca em entradas Started.

```bash
# Remover todos os journals terminais independentemente da idade (seguro: ignora Started)
atomwrite --workspace . wal-heal . --threshold-secs 0
```

Saída medida num workspace vazio: `{"type":"wal_heal","removed":0,"preserved":0,"malformed":0,"bytes_reclaimed":0,"threshold_secs":0,"ok":true,...}` com `target_resolved` e `target_source` depois.

Use este comando em smoke tests pós-instalação, hooks de pre-build locais, ou após uma varredura de recuperação de crash.


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

- `tests/cross_compile_check.rs` tem nove testes `cross_compile_*` (Windows GNU x64 e i686, Windows MSVC x64 e arm64, Linux musl x64, Linux aarch64 GNU e musl, Apple aarch64 e x64), todos `#[ignore]` por padrão
- Quatro deles passaram em 2026-10-31 (linux musl x64, linux aarch64 gnu, linux aarch64 musl e windows gnu x64), e o `cargo check` precisa do alvo do rustup mas não de linker estrangeiro, então a falta do `x86_64-w64-mingw32-gcc` só bloqueia um `cargo build` completo


## Status de Verificação
- Validado: Linux x86_64 (`x86_64-unknown-linux-gnu`), com a execução final da v0.1.37 registrada no `CHANGELOG.md`: 131 alvos de teste, 1486 testes passaram, 0 falharam e 9 ignorados
- Validado só por compilação: `cargo check --lib` passa para `x86_64-pc-windows-gnu`, então o par `cfg(windows)` (SIGPIPE, `restrict_to_owner`, `persist_retry.rs`) compila, e nenhum binário de Windows foi executado
- Não validado: todo caminho de execução em Windows e macOS, porque não houve máquina de nenhum dos dois sistemas, e os alvos Apple param no `cc-rs` por falta da toolchain C
- Compila: quatro testes `cross_compile_*` passaram em 2026-10-31, e os outros cinco exigem alvos que este host não tem (veja Validação Cross-Compile)
- Não validado: o lock advisory de escrita em filesystems sem `flock` (NFS) e no Windows, onde o lock é `LockFileEx` via `std::fs::File::try_lock` e não foi executado nesse sistema
- Não validado: o retry de rename do `persist_retry.rs` no Windows, que só foi conferido compilando uma cópia do corpo dele em Linux
- Não validado: um build com `--features watch`, e o build slim além dos comandos listados em Builds Slim vs Full por Features
- Não validado: os comandos de instalação para macOS e Windows deste guia, o comando do Gatekeeper e os caminhos da tabela XDG para esses sistemas


## v0.1.20 — Novidades

Esta release introduz uma nova camada de segurança chamada intention guards e renomeia a flag global `--lang` para `--locale` para desambiguar do seletor tree-sitter `--lang` usado por `scope` e `transform`.

### Intention Guards (5 flags OPT-IN)
- `--require-backup` — exige `--backup`: aborta quando falta e o arquivo alvo existe (não recebe valor)
- `--confirm` — flag legada de consciência no `write`, sem efeito sozinha: sobrescrita grande exige `--ack-overwrite`, e `delete --confirm` é rejeitado (use `--plan`)
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
- Histórico (na v0.1.20): 542 testes passando em 47 suítes de integração, 0 falhas
- 11 GAP-2026 fechados
- 3 targets de cross-compile Windows verdes
- Histórico (na v0.1.20): 19 ADRs em `docs/decisions/` (0019-0037)

### Migração `--lang` para `--locale`

```bash
# Descobrir todos os arquivos com --lang
rg -l -- '--lang\b' .

# Substituir em massa preservando outros matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Ou via ruplacer (só simula até você acrescentar --go)
ruplacer -- '--lang' '--locale' .
```


## v0.1.21 — Novidades

Esta release fecha 3 GAP-2026 items e altera o comportamento padrão de retenção de backups:

- `--allow-sequential-drift` em `edit` — flag opt-in para pipelines de edição sequencial
- `--backup` paridade 4/4 — `edit` e `rollback` agora aceitam `--backup` (eram hardcoded `false`)
- `--keep-backup` em 6 sub-comandos — flag opt-in para preservar o backup após sucesso
- Mudança de comportamento padrão — backups são DELETADOS após operações `--backup` bem-sucedidas; adicione `--keep-backup` para preservar
- 555+ testes passando, 1 novo ADR (0038)


## v0.1.22 — Novidades

Esta release adiciona 2 novos sub-comandos para limpeza e padrão de N-edits-em-1-invocação:

- `prune-backups [PATHS]...` — limpeza manual de siblings `.bak.YYYYMMDD_HHMMSS` legados
  - Flags: `--max-age-secs <SECONDS>`, `--max-count <N>`, `--dry-run` (só prévia; sem ele os backups antigos são removidos). Um entre `--max-age-secs` e `--max-count` é obrigatório, e os caminhos são os arquivos alvo cujos irmãos `.bak.*` são considerados
  - Saída NDJSON com linhas por backup e summary
- `edit-loop <PATH>` — aplica N pares `{old, new}` via NDJSON no stdin em 1 invocação
  - Flags: `--workspace`, `--partial`, `--allow-sequential-drift`, `--syntax-check`, `--line-ending`, `--backup`, `--keep-backup`, `--retention` (o `edit-loop` não aceita `--expect-checksum`, `--fuzzy` nem `--preserve-timestamps`)
  - Saída NDJSON com `pair_result` por par e summary
- 575+ testes passando, 2 novos ADRs (0039, 0040), 2 novos schemas NDJSON
- 33 sub-comandos totais (32 da v0.1.22 mais `verify` da v0.1.25)

### Instalar v0.1.22

```bash
# Do crates.io
cargo install atomwrite --locked --version "^0.1.22"

# Verificar
atomwrite --version

# Smoke test dos novos sub-comandos
atomwrite --workspace . prune-backups --max-age-secs 86400 --dry-run .
printf '%s\n' '{"old":"foo","new":"bar"}' \
  | atomwrite --workspace . edit-loop src/foo.rs
```
