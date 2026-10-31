# ADR-0070 — O `watch` vira uma feature opt-in (E7)

## Status
Aceito (2026-10-15)

## Contexto
- O atomwrite é um binário one-shot: nasce, roda e morre. O binário padrão carregava o `notify` e um laço de eventos de longa duração de que esse modelo não precisa.
- O canal de eventos do `watch` era ilimitado.

## Decisão
- O `watch` fica fora do conjunto de features `default`: `default = ["core", "ast", "lang-rust", "lang-ts", "lang-py"]`. A feature `watch = ["dep:notify"]` o traz de volta, e o `full` a inclui.
- O verbo continua na superfície por um stub em `src/commands/watch.rs` (`#[cfg(not(feature = "watch"))]`) que falha fechado com exit 78 `CONFIG_INVALID` e a dica "watch requires rebuild: cargo install atomwrite --features watch (or cargo build --features watch)".
- O `agent-surface` continua listando o `watch` entre as 47 tools, com `available: false` e um `install_hint`, de modo que um build nunca esconde um verbo.
- Com a feature, o `watch` usa um canal limitado de 4096 eventos, onde era ilimitado.
- O `cargo xtask check-features` roda `cargo check --locked` nos conjuntos de features `core` e `core,watch` e para no primeiro erro.

## Consequências
- A superfície tem 47 verbos em todo build, mas o `watch` só funciona com `--features watch` ou `--features full`. A documentação não pode dizer que o build padrão tem `watch`.
- Sem a feature o verbo nunca inicia um laço, então o binário padrão continua one-shot.
- O `watch` compilado com a feature termina com um evento `watch_summary` (`src/commands/watch.rs:198`).

## Alternativas
- Manter o `watch` no `default`: rejeitada, ele carrega o `notify` e um laço de longa duração em toda instalação.
- Remover o verbo quando a feature está desligada: rejeitada, o SSOT de 47 nomes mudaria de build para build.

## Evidência
- `CHANGELOG.md:70,124,147` e `gaps.md:72` (E7).
- `Cargo.toml:302-318` (o comentário sobre o E7, `default`, `watch`, `full`), `src/commands/watch.rs:54,209,218` (código da feature) e `:289-300` (stub), `xtask/src/main.rs:105-133` (`check_features`).
- `tests/cli_v0137_artefato.rs:555` (`default_binary_does_not_carry_the_continuous_mode`).
- Binário 0.1.37 (instalado sem a feature): `atomwrite watch DIR --max-events 1` saiu com exit 78, `CONFIG_INVALID` e a dica de rebuild. O `agent-surface` imprimiu `available: false` para o `watch` com `install_hint` "cargo install atomwrite --features watch (or cargo build --features watch)".

## Gatilho para revisar
- Um modo de longa duração virar parte do contrato do produto.
- O `notify` sair da árvore ou o conjunto de features ser reorganizado.
