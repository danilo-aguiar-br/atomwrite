# ADR-0059 — Um módulo de retry com full jitter e prazo global

## Status
Aceito (2026-10-15)

## Contexto
- O lock consultivo, o persist de tempfile do Windows e o laço de `EAGAIN` do banner de encerramento tinham cada um o seu laço de espera, com intervalo fixo e sem jitter (gaps.md W8).
- Nenhum deles tirava um teto de `--timeout-secs`, então um lock preso podia manter uma execução viva além do prazo.
- O laço de `EAGAIN` girava em `yield_now` a 100% de CPU quando o stderr era um pipe cheio.

## Decisão
- Toda espera por um recurso disputado passa por `src/retry.rs`. Uma `RetryPolicy` guarda uma base, um teto, `max_elapsed` e `max_attempts`.
- A espera de número `n` é uniforme em `[0, min(cap, base * 2^n)]` (full jitter, `fastrand`, dependência direta desde esta versão). A primeira espera vem antes da segunda tentativa, nunca há retry imediato.
- Há duas condições de parada: `max_attempts` esperas gastas, ou o orçamento de tempo gasto. O orçamento é o menor entre o `max_elapsed` da política e o tempo que resta de `--timeout-secs`.
- Um sleep é cortado em fatias de `RETRY_SLICE_MS` (15 ms), e a bandeira de encerramento é consultada entre as fatias, então SIGINT, SIGTERM, SIGHUP e o prazo são vistos dentro de uma fatia.
- A política do lock é base 50 ms, teto 400 ms, 512 tentativas e orçamento igual a `lock.acquire_timeout_ms` (padrão 5000 ms). Um cancelamento a encerra como `Cancelled`, um orçamento gasto a encerra como `LOCK_TIMEOUT` (exit 83). Quando o orçamento é o prazo global, a espera dá ao watchdog até `RETRY_GLOBAL_DEADLINE_GRACE_MS` (100 ms) para levantar a bandeira, então a execução termina em exit 124 e não em 83.
- O retry do persist (`src/atomic/persist_retry.rs`, Windows) usa o mesmo módulo com 8 tentativas e orçamento de 2000 ms, e agora também trata `ERROR_SHARING_VIOLATION` como transitório.
- O laço de `EAGAIN` espera com `poll(2)` e `POLLOUT` em fatias de 20 ms, até 200 ms.
- O formato é ajustável com `retry.base_ms`, `retry.max_ms` e `retry.max_elapsed_ms` (ADR-0058), e `retry.base_ms` não pode passar de `retry.max_ms` nem de `retry.max_elapsed_ms`.

## Consequências
- O retry nunca é o motivo de uma execução passar do prazo.
- Escritores que colidiram uma vez não colidem de novo em sincronia.
- O ramo do Windows (`persist_retry.rs`) foi checado só compilando uma cópia do seu corpo em Linux, porque o alvo Windows não pôde ser compilado aqui (gaps.md W16, P3).
- O lock tem um custo: um lock disputado agora espera com sleeps aleatórios, então a hora exata do despertar varia de execução para execução.

## Alternativas
- Manter três laços separados: rejeitada, eles não tinham jitter nem prazo e um deles queimava um núcleo.
- Tentar de novo imediatamente na primeira falha: rejeitada, o módulo impõe uma primeira espera.

## Evidência
- `CHANGELOG.md:65-66,168-172` e `gaps.md:174-176` (W8), `gaps.md:361` (C12).
- `src/retry.rs:3-24` (regras), `src/retry.rs:36-111` (`RetryPolicy`, `with_shape_overrides`, `with_budget_override`, `jittered`), `src/retry.rs:182-239` (`Backoff::wait_with`, `sleep_sliced`), `src/constants_concurrency.inc.rs:10-27` (fatia de 15 ms, tolerância de 100 ms, formatos do lock e do persist).
- `src/lock.rs`, `src/atomic/persist_retry.rs` (81 linhas).
- `tests/cli_v0137_retry_deadline.rs` (4 testes), `tests/cli_v0137_retry_guards.rs` (4 testes), mais os testes unitários de `src/retry.rs`.
- gaps.md C12 (V1 e V2): um lock preso com `--timeout-secs 2` sai com exit 124 em cerca de 2031 ms, `lock.acquire_timeout_ms=500` sai com exit 83 em 519 ms, SIGTERM sai com exit 143 em 14 ms.
- Binário 0.1.37: `config get retry.base_ms`, `retry.max_ms`, `retry.max_elapsed_ms` e `lock.acquire_timeout_ms` devolveram 50, 400, 2000 e 5000 (todos com `origin` `default`, exit 0).

## Gatilho para revisar
- Um novo recurso precisar de uma espera que não caiba em "uniforme em `[0, teto]`".
- O caminho de persist do Windows ser validado num host Windows real e se comportar diferente.
