# ADR-0078 — Um schema por verbo, descrevendo o stream inteiro

## Status
Aceito (2026-10-31), substitui a lista de nomes da ADR-0067

## Contexto
- O emissor foi escrito sob a regra "um nome descreve o evento terminal", e os verbos ganharam registros antes e depois desse terminal, então o documento ficou atrás do stream.
- O `recipe` servia `RecipeResult` enquanto emitia `begin`, `match`, `end`, `plan`, `replace`, `progress`, `hash`, `cancelled`, `recipe_step` e `recipe_result`, e cada registro de filho reemitido trazia `step`.
- O `codemod` servia `CodemodSummaryEvent` enquanto emitia `codemod`, `rule_begin`, `rule_error` e `transformed` com `rule_id`, e no erro publicava o `summary` do filho como terminal próprio.
- `batch`, `semantic-search`, `sparse`, `semantic-merge`, `case` e `watch` serviam só o resumo, e `search`, `replace`, `list` e `backup` partiam o stream entre o nome do verbo e um nome `*-summary`.
- Nenhuma união declarava o envelope de falha, então uma execução que morria antes do primeiro registro não validava contra schema nenhum do próprio verbo.
- O `prune-backups` trazia `additionalProperties: false` e `required: ["ok"]` ao lado do `anyOf` da raiz, então um validador recusava todo registro por entrada.

## Decisão
### 1. Um nome por verbo
- O nome do verbo é o único documento do seu stream, e os nomes `*-summary` deixam de ser servidos.
- `search`, `replace`, `list` e `backup` absorvem o conteúdo do seu nome de resumo como ramo terminal da própria união.
- A lista servida cai de 59 para 55 nomes, apoiados por 54 arquivos, porque quatro arquivos de resumo saem junto com os nomes.

### 2. O envelope de erro é ramo de toda união
- O `union_of` anexa o ramo `ErrorJson` a toda união que monta, inclusive às que já existiam.
- O `diff` mantém a `oneOf` e ganha o envelope como quinto ramo, exclusivo porque nenhum registro do verbo carrega `error`.

### 3. O ramo terminal continua fechado
- O `close_terminal_schema` devolve cedo quando a raiz tem `anyOf` ou `oneOf`, porque a raiz de união não declara chave própria.
- Cada construtor fecha o próprio ramo terminal com `additionalProperties: false`, as chaves sempre presentes em `required` e o detalhe de falha que o `publish_failure` carimba.

### 4. As marcas de filho são declaradas
- O `tagged_record` declara a chave que o verbo pai acrescenta, que é `step` no `recipe` e `rule_id` no `codemod`.
- O `terminal_record` acrescenta as chaves de veredito que o escritor carimba na última linha, e o detalhe de falha quando a execução falhou.

### 5. Três cópias do laço de `$defs` viram um helper
- O `union_of` recolhe o `$defs` de cada ramo, remove o `$schema` de cada um e devolve o documento, então `extract`, `scope` e `record_with_own_summary` chamam ele em vez de repetir o laço.

## Consequências
- Um consumidor valida o stream inteiro com um schema só, em vez de montar a união à mão a partir de dois documentos.
- Um formato de registro que o verbo deixa de escrever é um ramo que o schema ainda lista, e isso é deliberado porque o verbo pode escrevê-lo em outro modo.
- `docs/schemas/README.md`, `docs/AGENTS.md` e os dois `CHANGELOG` dizem a contagem nova e a união de cada verbo.
- macOS e Windows seguem NÃO VALIDADOS, porque a prova desta rodada rodou só em Linux.
