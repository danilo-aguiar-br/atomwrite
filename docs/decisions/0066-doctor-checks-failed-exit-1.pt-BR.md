# ADR-0066 — `DoctorChecksFailed`: `doctor --strict` sai com exit 1 e uma linha terminal só

## Status
Aceito (2026-10-15)

## Contexto
- O `doctor --strict` com uma checagem falha devolvia um erro sem tipo. Esse erro cai em exit 255 (`INTERNAL_ERROR`) e um segundo envelope de erro seguia o relatório (gaps.md W56, achado F-1 da revisão V3).
- A mesma revisão achou dois irmãos: um `write` interrompido também saía com exit 255, e o `batch` sem `--transaction` emitia o seu `summary` e depois um segundo envelope de erro.
- Um agente que vê 255 trata como crash, mas uma checagem de host que falha é um veredito resolvido, e não um bug.

## Decisão
- Criar `AtomwriteError::DoctorChecksFailed { failed, warned }`: código `DOCTOR_CHECKS_FAILED`, exit 1 (o status genérico de falha que o comando sempre devolveu), classe `precondition_failed`, porque o host precisa mudar antes de uma nova execução.
- O próprio `doctor_report` é a única linha terminal, publicado por `publish_failure` (ADR-0061). Ele carrega o detalhe da falha, e o erro carrega só os totais.
- O `ok` do relatório conta só `fail` no modo padrão e rejeita também `warn` sob `--strict`. O status `info` nunca falha. O detalhe da falha é opcional no schema do `doctor`, porque o relatório diz `ok: false` para uma checagem falha sem falhar a execução quando o `--strict` está ausente.
- Um `write` interrompido agora sai com o status real do sinal ou do prazo (143, 130 ou 124). O `batch` sem `--transaction` publica a falha no próprio `summary`.
- Os schemas `doctor` e `batch` são fechados (`additionalProperties: false`), e o nome que o `--print-schema` serve para o segundo é `batch` (ADR-0067).

## Consequências
- Agentes ramificam por exit 1 mais `code` em vez de tratar 255 como crash.
- O exit 1 agora é compartilhado por `NO_MATCHES` e `DOCTOR_CHECKS_FAILED`. O `code` desambigua os dois.
- Foi registrado como correção, e não como mudança de comportamento: o exit era 255 porque o erro não tinha tipo.

## Alternativas
- Manter o exit 255: rejeitada, é o status de erro interno e esconde um veredito normal.
- Um número de exit dedicado: não escolhida, o exit 1 é o que o comando sempre devolveu e o `code` carrega a distinção.

## Evidência
- `CHANGELOG.md:243-246` e `gaps.md:206-208` (W56).
- `src/error/variants.inc.rs:290-301` (variante e justificativa), `src/error/impl_methods.inc.rs:44` (exit), `:84` (classe), `:159` (código), `src/commands/doctor.rs:230-258` (tratamento do strict e `publish_failure`), `src/commands/agent_surface_exit.inc.rs:128,263`, `src/schema_terminal.inc.rs:57-67`.
- `tests/cli_v0137_doctor_strict.rs` (4 testes), `tests/cli_v0137_terminal_event.rs:540` (`batch_with_a_failed_operation_ends_with_one_failed_summary`), `tests/cli_v0137_stdin_cancel.rs:137` (`r2a_write_with_open_stdin_exits_143_on_sigterm`).

## Gatilho para revisar
- O exit 1 colidir com outro veredito que os agentes precisem separar sem ler o `code`.
