# ADR-0028: G119 — Limpeza inteligente de sidecars WAL em 5 camadas autônomas

> **Histórico (pré-0.1.35):** knobs `ATOMWRITE_*` / env descritos abaixo estão **supplantados**. Config de runtime é só flags CLI + XDG `config.toml` / `atomwrite set|get`.


- **Status**: Accepted
- **Date**: 2026-06-13
- **Context**: G119 começou parcial na v0.1.15 com L2 (Drop guard), L3 (wal-heal) e L5 (wal-stats) entregues. Faltavam L1 (prevenção via `--wal-policy`) e L4 (heurísticas avançadas). Sem L1, 100% das mutações com `--strict-atomic` ou `ATOMWRITE_WAL=1` continuam deixando sidecar; sem L4, o engine de decisão é binário (drop ou keep), sem nuance para TTL/LRU/rate-limit/sentinela/arquivamento. A auditoria empírica de 2026-06-13 contou 60 sidecars `Committed` em 10 diretórios (18 raiz, 18 docs, 14 schemas, 2 workflows, 2 skill-en, 2 skill-pt, 2 src, 1 commands, 1 tests, 1 snapshots) totalizando 47.624 bytes. Cada um desses sidecars representa I/O desperdiçado em write trivial (set/del, set de chave TOML, edit de uma linha) que o L1 teria prevenido na fonte.

- **Decision**: Fechar G119 em v0.1.16 com as duas camadas que faltavam:
  1. **L1 (Prevenção)**: novo `enum WalPolicy { Auto, Always, Never }` em `src/wal.rs`, exposto via flag CLI `--wal-policy` em `write` e `edit`. Política `Auto` (default) pula o sidecar quando a operação é trivial — arquivo ≤ 1 MiB E não-Edit/Replace E diretório sob Git E tamanho ≤ 4 KiB. `Always` força o sidecar (legado). `Never` suprime mesmo com `--strict-atomic`. Heurística de detecção de Git: walk de até 16 níveis ancestrais procurando `.git`.
  2. **L4 (Heurísticas)**: novo submódulo `crate::wal::heuristics` com 5 funções componíveis (`h1_ttl`, `h2_lru_within_cap`, `h3_rate_limit`, `h4_sentinel`, `h5_archive`). Cada uma retorna `true` para PRESERVAR, `false` para limpar. `heuristics_should_preserve(target, committed_at_unix, count, rank)` agrega via OR. H1/H2/H3/H5 lêem env vars (`ATOMWRITE_WAL_KEEP_SECS`, `ATOMWRITE_WAL_MAX_COUNT`, `ATOMWRITE_WAL_RATE_LIMIT`, `ATOMWRITE_WAL_ARCHIVE_DAYS`) com defaults seguros (0, 100, 10, 7). H4 detecta sentinela `.atomwrite_no_wal` no diretório. H3 usa `AtomicU64` de janela de 60s (sem `unsafe`, sem env-var mutation em testes). Novo campo `wal_policy: &'static str` no envelope `WriteOutput` para telemetria.
  3. **`AtomicWriteOptions.wal_policy: WalPolicy`**: novo campo default `WalPolicy::Auto`. `atomic_write` chama `should_create_sidecar` (L1) antes do `journal_started_with_guard`; quando L1 vota `false`, guard inerte substitui o sidecar (custo O(0), zero I/O extra).
  4. **Telemetria `wal_policy` no envelope**: campo `wal_policy: "auto" | "always" | "never"` em `WriteOutput` para que o agente audite qual policy foi aplicada.

- **Consequences**:
  - **+** L1 previne 60-80% de sidecars para workloads típicos de agente LLM (writes pequenos em diretórios versionados): a raiz da poluição.
  - **+** L4 dá flexibilidade operacional: `ATOMWRITE_WAL_KEEP_SECS=60` em scripts locais, `ATOMWRITE_WAL_MAX_COUNT=50` em workspaces densos, sentinela local em `target/.atomwrite_no_wal` para desabilitar.
  - **+** H3 (rate limit) protege contra agentes em loop que geram 1000 sidecars/min.
  - **+** H5 (archive) preserva histórico de auditoria sem poluir a árvore ativa.
  - **+** Default `Auto` é seguro: sidecars continuam existindo para operações não-triviais (arquivo > 1 MiB, edit, replace, dir não-versionado).
  - **+** Telemetria permite detectar misconfiguração (`auto` produzindo muitos sidecars = heurística errada).
  - **-** `--wal-policy never` em uso indevido elimina audit trail de crash — operador deve documentar a decisão.
  - **-** L4 tem 5 env vars: complexidade cognitiva. Mitigado por defaults sensatos e pela regra "OR preserva" (qualquer heurística votando true é suficiente).
  - **-** Heurística Git-detector não distingue `git init` em subdir de checkout de pai em Git; pode subestimar (falso positivo → sidecar pulado desnecessariamente). Aceitável: sidecar pulado é mais barato que sidecar criado sem necessidade.

- **Alternatives considered**:
  1. Implementar L1 e L4 em releases separadas (v0.1.17 e v0.1.19) como o plano original previa. Rejeitado: o usuário exigiu fechamento em uma única release; fragmentar atrasa o benefício.
  2. Usar `ignore` crate para detecção Git em vez de walk manual. Rejeitado: dependência extra; a heurística precisa apenas de um boolean e a busca por `.git` é trivial.
  3. Tornar L1+L4 opt-in via flag global. Rejeitado: o default atual já é muito permissivo (sempre cria); o default seguro é `Auto` que PULA o sidecar para triviais.
  4. H3 com Mutex em vez de AtomicU64. Rejeitado: AtomicU64 é lock-free, suficiente para o contador coarse de 1 janela.

- **Trigger to revisit**: Se L1+L4 introduzirem novo gap (ex: agente reclama de `Auto` pulando sidecar necessário), adicionar flag `--wal-policy-strict` que sempre preserva. Se o overhead de leitura de env-var por chamada mostrar-se mensurável (>5% em write pequeno), cachear via `OnceCell` global.

## Atualização v0.1.17 — Fiação de L3 startup + L4 no Drop guard

A v0.1.16 entregou L1 + L4 como código de biblioteca, mas ambas estavam **desconectadas** dos pontos onde o sidecar é criado e removido. A v0.1.17 fecha a fiação:

- **L3 wired em `lib.rs::run`**: após `resolve_workspace()` e ANTES de despachar o subcommand, chamar `auto_heal_on_startup(&workspace, threshold_secs=3600, max_duration_ms=100)`. Threshold de 1h é o padrão operacional; budget de 100ms mantém custo de startup determinístico. O sidecar `Started` (potencial órfão) NUNCA é reaped automaticamente — é o sinal que merece atenção do operador. A flag global `--no-auto-heal` (ou env `ATOMWRITE_WAL_NO_AUTO_HEAL=1`) desabilita para loops locais de alta cadência e benchmarks.
- **L4 wired em `JournalGuard::drop`**: o `Drop` agora consulta `heuristics_should_preserve` antes de remover. Como o contexto do Drop não conhece o `workspace_committed_count` (varredura cara), passamos `u64::MAX` para forçar `h2_lru_within_cap` a retornar `false`. O OR-composição garante que `h1_ttl`, `h3_rate_limit`, `h4_sentinel`, `h5_archive` continuam votando livremente. Custo: O(1) por sidecar (apenas `metadata` da env + checagem de sentinel).
- **Novo campo `committed_at_unix: Option<u64>` em `JournalGuard`**: `release()` carimba o timestamp Unix atual; `drop()` lê o valor para alimentar H1 (TTL) e H5 (archive). Sem timestamp, essas duas heurísticas ficavam permanentemente desabilitadas no caminho do Drop.
- **Compatibilidade com testes existentes**: 2 testes em `tests/cli_v012_wal.rs` e 1 em `tests/cli_wal.rs` foram ajustados para passar `--no-auto-heal` quando o setup pré-condiciona sidecars stale. Sem o flag, o L3 reapa os seeds antes do subcommand rodar.

### Consequências da fiação
- **+** Cada invocação agora começa com working tree limpo: sidecars Committed/Aborted com mais de 1h morrem no startup. Em workspace com 60 sidecars stale, o primeiro comando apaga 60 sem custo perceptível (~5ms em `walk_journal_paths`).
- **+** A flag `--no-auto-heal` dá escape para ambientes sensíveis (automação local paralela, benchmarks, dry-run chains) sem código custom.
- **+** L4 no Drop é a primeira camada que consulta heurísticas POR-EVENTO (h4_sentinel, h3_rate_limit) em vez de por-batch. Operador pode desabilitar sidecar em diretórios sensíveis com `.atomwrite_no_wal`.
- **-** Adicionar `--no-auto-heal` em testes manuais é necessário para inspeção forense de sidecars stale (ver 0027/0030).
- **-** Custo de startup sobe ~5-10ms em workspaces com >100 sidecars (budget de 100ms protege contra regressão).
- **-** OR-composição do L4 + `u64::MAX` trick é sutil: revisor deve entender que h2 fica efetivamente off no Drop. Documentado inline em `wal.rs:540`.
