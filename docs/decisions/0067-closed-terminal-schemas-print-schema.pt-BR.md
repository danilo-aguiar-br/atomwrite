# ADR-0067 — Schemas terminais fechados e um `--print-schema` fiel

## Status
Aceito (2026-10-15), lista de nomes substituída pela ADR-0078 (2026-10-31)

## Contexto
- Os schemas publicados descreviam um struct que o verbo não serializava. O `search-summary` e o `replace-summary` dividiam um tipo, então o schema de um resumo terminal era o schema do seu evento.
- O `diff` era um `anyOf` sem `type` e sem `required` (V1 FAIL-8), então um validador aceitava objetos que nenhum verbo emite.
- O schema `config` exigia uma chave `warnings` que `config path`, `config set` e `config unset` não emitem (V1 FAIL-2).
- Arquivos de `docs/schemas/` de alguns eventos tinham `additionalProperties: false` e nenhuma chave de veredito, então um evento real falharia na validação.

## Decisão
- Cada `--print-schema NOME` é gerado a partir do tipo Rust que o verbo realmente serializa (`schemars::schema_for!` por braço em `src/schema_dispatch.inc.rs`), e o `command_schema_name` mapeia cada subcomando ao seu nome, de modo que `--json-schema` e `--print-schema NOME` respondem da mesma tabela.
- Estes eventos terminais são fechados (`additionalProperties: false`, um `required` não nulo com as chaves sempre presentes): `search-summary`, `replace-summary`, `count`, `count-by-ext`, `count-by-size`, `backup-summary`, `list-summary`, `config`, `batch` e `doctor` (`CLOSED_TERMINAL_SCHEMAS`).
- O `diff` é `type: "object"` com um `oneOf` de quatro ramos fechados (`change`, `summary`, `unified`, `stat`), cada um com `required`.
- As chaves que o writer carimba (`ok`, `code`, `exit`, `target_resolved`, `target_source`) são propriedades opcionais de todo schema terminal.
- Um resumo que pode terminar na própria falha (`search-summary`, `replace-summary`, `batch`) acrescenta a regra "`ok: false` exige `code`, `exit`, `error_class`, `retryable`, `suggestion_lang` e `message`" (`FAILURE_REQUIRED`). O `doctor` carrega o mesmo detalhe como opcional.
- Seis nomes novos, `backup-summary`, `count-by-ext`, `count-by-size`, `list-summary`, `replace-summary` e `search-summary`, levam a lista a 58. Um nome desconhecido sai com exit 65 e a lista de nomes.

## Consequências
- O `atomwrite --print-schema` lista 55 nomes, e `docs/schemas/` tem 54 arquivos, 31 deles regenerados a partir do binário e 4 completados com as chaves carimbadas.
- Dois conjuntos de nomes convivem: o `--print-schema` serve nomes curtos (`read`, `edit`) enquanto `docs/schemas/` publica nomes descritivos (`read-output`, `edit-output`). É um risco residual declarado (CHANGELOG, Security).
- A validação nos testes é uma checagem só de tipo, de `required` e de nomes. Nenhum validador de JSON Schema é dependência do produto. Um teste valida 12 invocações reais (falhas de `search` e `replace` incluídas), e um teste negativo prova que o validador rejeita uma chave ausente, uma chave não declarada e uma falha sem `code`.
- O `cancelled` não carrega `ok: false` nem `code`, por desenho do `docs/schemas/README.md` (gaps.md W44).

## Alternativas
- Manter um tipo compartilhado para `search-summary` e `replace-summary`: rejeitada, as chaves deles diferem.
- Deixar os schemas abertos (`additionalProperties` sem valor): rejeitada, um validador então aceita qualquer objeto.

## Evidência
- `CHANGELOG.md:63-64,160,225-227,235` e `gaps.md:284-285` (W39), `gaps.md:305-307` (W48).
- `src/schema_terminal.inc.rs:11-22` (`CLOSED_TERMINAL_SCHEMAS`), `:25-41` (`never_emitted`, `always_emitted`), `:44-67` (`FAILURE_REQUIRED`, `ends_failure_with_summary`), `src/schema_dispatch.inc.rs:11-70` (`SCHEMA_NAMES`), `:219-270` (`command_schema_name`), `:272-310` (`writer_stamped_properties`, `with_envelope_keys`).
- `tests/cli_v0137_terminal_schema.rs` (9 testes).
- Binário 0.1.37: `atomwrite --print-schema` imprimiu 58 linhas NDJSON, e o `agent-surface` publica a mesma lista em `discovery.schema_names`.

## Gatilho para revisar
- Um validador de JSON Schema virar dependência, para os testes validarem o schema inteiro e não só `required` e nomes.
- Os dois conjuntos de nomes serem unificados.
