# ADR-0061 — Um evento terminal por execução com falha (`publish_failure` e `VerdictPublished`)

## Status
Aceito (2026-10-15)

## Contexto
- O `NdjsonWriter::write_event_failed` tinha 7 chamadores e nenhum marcava o veredito como publicado, então o `main` escrevia o envelope de erro de novo. Uma execução com falha terminava com duas linhas terminais (gaps.md W6).
- O caso aparecia no `search` sem match, no `semantic-merge` com conflito e no `edit-loop --partial`. Um parser que conta um evento terminal por execução errava a conta.
- O `search`, o `transform` e o `replace` também engoliam erros de leitura, de abertura, de sonda e de caminhada, então saía `NO_MATCHES` para uma árvore que eles não conseguiram ler por inteiro (gaps.md W5).

## Decisão
- O `NdjsonWriter::publish_failure` grava o resumo terminal com `ok: false`, `code`, `exit` e todo o detalhe do erro (`error_class`, `retryable`, `suggestion`, `suggestion_lang`, `conflict_count`, `match_count`, `pairs_*`).
- Depois ele devolve o erro marcado com `VerdictPublished`, que o `main` lê para pular o segundo envelope. O `write_event_failed` foi removido.
- Um evento de falha ignora `--filter` e `--limit` e respeita `--select` e `--redact`.
- O `search`, o `transform` e o `replace` relatam arquivos ilegíveis como `files_failed` e `errors[]` no resumo, com `errors[]` limitado a `SUMMARY_ERRORS_MAX_ENTRIES` (50) enquanto `files_failed` guarda a contagem inteira. Zero matches com falha sai com o erro da primeira falha, e um match com falha sai com exit 0 e `files_failed` visível. Um arquivo grande demais, que não é UTF-8 ou binário continua sendo pulado por desenho.
- O envelope de sucesso traz `ok: true`, carimbado uma vez pelo writer. Um resumo não é mais seguido por um erro que o contradiz.
- O `EditPairFailed` agora tem o código `MATCH_FAILED`, com o mesmo exit 65.

## Consequências
- Exatamente um evento terminal por execução com falha, salvo um pipe quebrado e um veredito de `hash --verify` já publicado.
- Os schemas de resumo trazem o detalhe da falha como propriedades opcionais (`FAILURE_REQUIRED`, ADR-0067).
- O `verify` com checksum errado emite um evento com um veredito, em vez de dois eventos que se contradizem.
- Um passe de cauda (`--sort-by` e afins) ainda pode deixar o `--filter` atingir o evento de falha (gaps.md W25, P3), e o `transform` multi-regra ainda escreve o resumo `ok: false` do filho antes do `rule_error` (W24, P3).

## Alternativas
- Manter o `write_event_failed` e fazer cada chamador marcar o veredito: rejeitada, 7 chamadores já tinham esquecido.

## Evidência
- `CHANGELOG.md:42,61,156-160,262,265,300` e `gaps.md:184-192` (W5, W6).
- `src/output.rs:96-110` (`publish_failure`), `src/output_verdict.inc.rs:161-181` (`VerdictPublished`), `src/main.rs:147-153` (pula o segundo envelope), `src/schema_terminal.inc.rs:43-67`.
- `rg -n -F write_event_failed src` não devolve nenhuma linha.
- `tests/cli_v0137_terminal_event.rs` (17 testes) e `tests/cli_v0137_envelope.rs` (26 testes).

## Gatilho para revisar
- Um verbo precisar de dois eventos terminais por desenho.
- O passe de cauda ser refeito e o `--filter` nunca poder atingir um evento de falha.
