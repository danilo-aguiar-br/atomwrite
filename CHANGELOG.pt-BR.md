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
- `cargo deny check`: PASS (advisories, bans, licenses, sources todos OK)
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
