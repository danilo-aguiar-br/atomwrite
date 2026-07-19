# ADR-0049: encanamento vivo de config para o `[defaults]` do `.atomwrite.toml`

> **Histórico (pré-0.1.35):** knobs `ATOMWRITE_*` / env descritos abaixo estão **supplantados**. Config de runtime é só flags CLI + XDG `config.toml` / `atomwrite set|get`.


- **Status**: Aceito
- **Date**: 2026-07-06
- **Context**: As chaves `backup`/`retention` do `[defaults]` do `.atomwrite.toml` eram parseadas por `load_config` mas nunca de fato consultadas por nenhum comando mutante (GAP-CONFIG-DEFAULTS-DEAD) — o resolver que existia na época recebia apenas `(backup: bool, no_backup: bool)` como argumentos e não tinha caminho para alcançar os valores parseados da config. Usuários que configuravam `[defaults]` no config do projeto viam-no silenciosamente ignorado.
- **Decision**: `load_config` agora é chamado exatamente uma vez, em `lib.rs::run()`, e o `DefaultsSection` resultante é encadeado e propagado a cada handler de comando mutante. `resolve_backup()` (ADR-0048) agora recebe `&DefaultsSection` como segundo argumento e consulta `defaults.backup`/`defaults.retention` como o terceiro nível de precedência: variável de ambiente `ATOMWRITE_BACKUP` > flags CLI (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > default embutido (`true` / `5`).
- **Consequences**:
  - **+** As chaves `backup`/`retention` do `[defaults]` do `.atomwrite.toml` agora são efetivas em todo subcomando mutante, fechando o GAP-CONFIG-DEFAULTS-DEAD.
  - **+** A config é carregada uma vez por invocação em vez de ser reparseada ou ignorada por handler, evitando I/O redundante e deriva entre handlers.
  - **-** Handlers que mutam conteúdo agora carregam um parâmetro extra `&DefaultsSection`; essa é uma mudança de assinatura única absorvida pela refatoração de `BackupOpts` (ADR-0048) no mesmo release.
- **Alternatives considered**:
  1. **Carregar a config independentemente dentro de cada handler.** Rejeitado: duplica I/O e arrisca reintroduzir deriva por handler, exatamente o modo de falha que este ADR fecha.
  2. **Manter `[defaults]` parseado-mas-não-usado e documentá-lo como reservado para um release futuro.** Rejeitado: a config já documentava as chaves como efetivas; deixá-las mortas é um descompasso de documentação/comportamento, não um adiamento.
- **Trigger to revisit**: Se `[defaults]` ganhar chaves adicionais além de `backup`/`retention`, confirmar que `DefaultsSection` e seu caminho de propagação sejam estendidos de forma consistente em vez de adicionar um caminho de resolução paralelo.
