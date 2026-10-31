# ADR-0072 — Superfície agent-native: moldagem de stream, `--print-schema` antes do clap, guardas de efeito colateral (E3, E4, E6)

## Status
Aceito (2026-10-15)

## Contexto
- Agentes gastavam tokens em pipes de `jq` e `sed` para cortar, ordenar e somar NDJSON, e em adivinhar nomes de schema.
- O truncamento destrutivo da saída não tinha opt-in explícito.
- O `--print-schema` dentro do clap não podia responder numa instalação que ainda não consegue analisar uma linha de comando completa.
- Uma flag de moldagem podia apagar justamente o aviso de que a resposta está incompleta.

## Decisão
- Moldar o stream dentro do processo, no canal de dados, com flags globais: `--sort-by` (`path-line`, `line-path` ou uma chave), `--dedupe CHAVE`, `--aggregate CHAVE[:OP[:CAMPO]]`, `--join CHAVE:SUFIXO`, `--diff-against ARQUIVO`, `--redact`, `--normalize-encoding` e `--max-output-bytes`. As flags `--select`, `--fields`, `--filter` e `--limit` da 0.1.36 (ADR-0055) continuam.
- O `--max-output-bytes` trunca todo valor string, o `checksum` inclusive, e exige `--allow-truncate` junto. Sem ele a execução sai com exit 2.
- O `--print-schema [NOME]` é respondido antes de o clap analisar o subcomando (`scan_print_schema`, `plan_print_schema`, `prescan_print_schema`). Sem nome ele lista todo schema, uma linha NDJSON cada. Um nome desconhecido sai com exit 65, a lista de nomes e um envelope de erro, nunca um documento vazio. O `--print-schema=NOME` é aceito exatamente como a forma separada, e a flag posta depois do comando sem valor não substitui o comando.
- `--non-interactive` e `--confirm-side-effect` são aceitas globalmente. O `--confirm-side-effect` reconhece um efeito colateral só onde o contrato do comando pede.
- O `agent-surface` publica uma entrada `discovery` gerada a partir de `SCHEMA_NAMES`, de modo que o manifesto e a porta nunca discordam.
- Erros são sempre emitidos: o `--filter` nunca esconde `ok: false`, e o registro terminal de cancelamento é escrito sem moldagem (ADR-0073).
- O `-n` continua reservado ao `replace`. As flags de moldagem usam `--limit`.
- Uma chave de topo com formato de credencial é redigida em todo envelope, com ou sem `--redact`, e o `--redact` também alcança o envelope de erro, com uma lista ampliada de prefixos de emissão.
- O `serde_json` é compilado com `preserve_order`, de modo que um envelope serializado por `Value` mantém a ordem de chaves que o seu struct declara. Custo medido: +2 ms sobre 47 ms para 3000 eventos (cerca de 4%).

## Consequências
- Uma superfície global de flags maior: 33 args globais em `atomwrite commands --include-globals`.
- O `--max-output-bytes` é destrutivo para o `checksum` (20 caracteres numa medição manual), então não deve ser usado quando o `checksum` ou o `content` inteiro for necessário.
- O `--aggregate` mantém `ok`, `code` e `exit` do evento terminal que falhou, e o `--select` não emite mais um evento que ele deixou vazio.
- O `--join` e o `--aggregate` sozinhos não tinham teste de execução quando o gaps.md foi escrito (gaps.md E4). O `--aggregate type:count` rodou à mão e devolveu `{"type":"entry","count":53}` mais o resumo.
- Com um passe de cauda ativo (`--sort-by` e afins) o evento de falha entra no buffer e o `--filter` ainda pode atingi-lo (gaps.md W25, P3).
- O `search` sem `--sort-by` não garante ordem entre execuções quando é multithread, igual ao binário antigo (gaps.md W44 B11, P3).

## Alternativas
- Deixar a moldagem para pipes de `jq` e `sed`: rejeitada, foi o que custou os tokens.
- Tratar o `--print-schema` dentro do clap: rejeitada, a descoberta precisa funcionar antes de uma linha de comando completa ser analisada.
- Truncamento silencioso pelo `--max-output-bytes`: rejeitada, ele exige o `--allow-truncate` explícito.

## Evidência
- `CHANGELOG.md:14-17,23,112-114,123,125-127,221,325-328` e `gaps.md:64-68,71` (E3, E4, E6), `gaps.md:277` (W36).
- `src/cli_global.inc.rs:140` (`print_schema`), `:154,167` (`non_interactive`, `confirm_side_effect`), `:177-181` (`allow_truncate` exige `max_output_bytes`), `:269-381` (flags de moldagem), `src/schema_dispatch.inc.rs:103-148` (`scan_print_schema`, `plan_print_schema`), `src/runtime_cli.inc.rs:132-140`, `src/main.rs:71-80`, `src/commands/agent_surface.rs:59-135` (`discovery`), `Cargo.toml:109`.
- `tests/cli_v0137_agent_native.rs` (22 testes, incluindo `print_schema_*` nas linhas 49-160, `max_output_bytes_requires_the_truncation_opt_in` na 357 e `non_interactive_is_accepted_globally` na 589).
- Binário 0.1.37: `commands --include-globals` lista 33 args globais, o `agent-surface` tem um objeto `discovery` com 54 `schema_names`, e o `--print-schema` lista 55 nomes.

## Gatilho para revisar
- Uma flag global colidir com uma flag local de um verbo.
- O passe de cauda ser mudado para o `--filter` nunca atingir um evento de falha.
