# 0041-allow-hyphen-values-edit (PT-BR) — Tradução

_Tradução automática do ADR original em inglês. Em caso de divergência, a versão EN prevalece._

# ADR-0041: allow_hyphen_values para 15 campos CLI que aceitam texto livre em 8 structs

- **Status**: Aceito
- **Data**: 2026-06-19
- **Contexto**: O Clap v4 trata por padrão qualquer token que começa com `-` como uma flag CLI. 15 campos em 8 structs de argumentos (`EditArgs`, `SearchArgs`, `ReplaceArgs`, `CalcArgs`, `RegexArgs`, `TransformArgs`, `ReadArgs`, `QueryArgs`) aceitam conteúdo de texto livre do usuário mas não tinham `allow_hyphen_values = true`. Isso causava `ARGUMENT_PARSE_ERROR` (exit 2) quando valores continham hífens iniciais — comum em bullet points Markdown (`- item`), números negativos (`-5 + 3`), entradas YAML (`- key: value`) e conteúdo de diff (`--- a/file`). Em pipelines de agentes LLM, a falha cascateava: exit 2 mascarado pelo pipe `jaq` (o `jaq` downstream recebe `null` e imprime `{"edits": null}` com exit 0), workaround do agente via `write` truncante, perda catastrófica de dados (arquivo de 22.597 bytes sobrescrito por 13 bytes). O problema foi observado primeiro na v0.1.22 mas existia desde a introdução de `--old`/`--new` na v0.1.12. Nenhum teste jamais passou um valor começando com `-` para qualquer um dos 15 campos afetados, então o bug era invisível à suíte de regressão. `CaseArgs.subvert` foi avaliado mas excluído: seu atributo `num_args = 2..` cria um parser guloso que consome flags seguintes como valores quando combinado com `allow_hyphen_values`, quebrando o parsing da flag `--to`.

- **Decisão**: Adicionar `allow_hyphen_values = true` a todos os 15 atributos `#[arg]` afetados em `src/cli_args.rs`. Os campos são organizados em três tiers por risco e frequência de uso:
  - **Tier 1 (EditArgs)**: `old`, `new`, `after_match`, `before_match`, `between` — campos de maior risco porque `edit` é o comando mutante mais comum em pipelines de agentes, e bullet points Markdown são a fonte mais frequente de hífens iniciais.
  - **Tier 2 (posicionais)**: `SearchArgs.pattern`, `ReplaceArgs.pattern`, `ReplaceArgs.replacement`, `CalcArgs.expression`, `RegexArgs.examples` — argumentos posicionais que aceitam conteúdo arbitrário do usuário incluindo números negativos, padrões regex com classes de caracteres como `[-a-z]`, e strings de exemplo.
  - **Tier 3 (nomeados)**: `TransformArgs.pattern`, `TransformArgs.rewrite`, `TransformArgs.inline_rules`, `ReadArgs.grep`, `QueryArgs.query` — flags nomeadas que aceitam padrões, expressões AST ou filtros regex que podem começar com `-`. Excluído: `CaseArgs.subvert` (`num_args = 2..` incompatível com `allow_hyphen_values`).

- **Consequências**:
  - **+** Agentes conseguem editar conteúdo Markdown com bullet points (`--old "- item antigo" --new "- item novo"`) sem exit 2. Este é o modo de falha mais comum reportado em pipelines de agentes.
  - **+** `calc "-5 + 3"` e `search "-deprecated"` funcionam sem exigir que o usuário insira um separador `--` ou workarounds de aspas.
  - **+** Conteúdo YAML com hífens iniciais pode ser editado, buscado e substituído diretamente.
  - **+** Números negativos em expressões `calc` são aceitos como valores, não rejeitados como flags desconhecidas.
  - **+** A correção elimina a cadeia de falhas cascateantes: exit 2 → mascarado por jaq → workaround do agente via write truncante → perda de dados.
  - **-** (aceitável) `--old --typo` agora trata `--typo` como o valor de `--old` em vez de reportá-lo como flag desconhecida. Isso é aceitável para campos de texto livre onde a intenção do usuário é sempre "isto é conteúdo, não uma flag". Para flags não-conteúdo (ex.: `--backup`, `--dry-run`), o comportamento padrão do Clap é preservado e flags desconhecidas ainda são reportadas.
  - **-** (aceitável) Um usuário que digita errado `--old --new "texto"` (esquecendo o valor de `--old`) receberá `--new` como valor de `--old` em vez de um erro útil. Este é um tradeoff inerente de `allow_hyphen_values` e é documentado nos docs do Clap v4. O modo de falha é detectável via o campo `pair_results` na resposta do edit.

- **Alternativas consideradas**:
  1. **Separador `--` antes de valores que começam com hífen.** Rejeitado: `--` termina o parsing de flags globalmente, o que quebra o modo multi-par (`--old "- a" --new "b" --old "- c" --new "d"` — o `--` impediria o segundo `--old` de ser parseado como flag). Também incompatível com o padrão de pipeline de agentes onde pares `--old`/`--new` são gerados programaticamente.
  2. **Convenção de aspas documentada em SKILL/CLAUDE.md.** Rejeitado: as aspas já são aplicadas pelos agentes (eles passam `--old "- texto"` com aspas), mas o Clap rejeita o valor ANTES das aspas serem processadas pelo shell. A correção deve ser no nível do atributo Clap, não no nível do shell. Além disso, alterar a documentação dos agentes não corrige workflows de agentes existentes que já usam as aspas corretamente.
  3. **Aplicar `allow_hyphen_values` apenas ao Tier 1 (EditArgs).** Rejeitado: o mesmo bug existe em todos os 15 campos, e uma correção parcial deixaria 10 campos vulneráveis à mesma falha cascateante. O custo de corrigir todos os 15 é idêntico (um atributo por campo) e o risco é simétrico.
  4. **Incluir `CaseArgs.subvert`.** Rejeitado: `subvert` usa `num_args = 2..` (guloso, aceita 2+ valores). Com `allow_hyphen_values = true`, o Clap consome flags seguintes (como `--to camel`) como valores do `--subvert`, causando falha em 5/7 testes do case com erro de "contagem ímpar". Identificadores raramente começam com `-`, então o risco é negligenciável.

- **Gatilho para revisitar**: Se usuários reportarem que `--old --typo` (nome de flag digitado errado aceito como valor) causa confusão em uso interativo, adicionar um `tracing::warn!` quando um valor casa com o padrão `--[a-z]` para alertar o usuário que seu valor parece um nome de flag. Se o Clap v5 introduzir um atributo `warn_on_hyphen_value` por campo, migrar para ele para melhor diagnóstico.


---

_Original em inglês: [`0041-allow-hyphen-values-edit.md`](0041-allow-hyphen-values-edit.md)_
