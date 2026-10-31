# ADR-0058 — O verbo `config` e as chaves de ajuste tipadas

## Status
Aceito (2026-10-15)

## Contexto
- Os valores de ajuste (limiares fuzzy, timeout de lock, formato do retry, orçamento do search, limites do extract) eram literais no código, então mudar um deles exigia editar o fonte (gaps.md W13).
- O único outro jeito de mudá-los era editar o `config.toml` à mão, o que podia corromper o arquivo e deixava o valor sem checagem até a próxima execução.
- A regra de hardcode da v0.1.37 (ADR-0068) levou todo literal para `constants*`, então era preciso um jeito persistente de sobrescrevê-los fora do código.

## Decisão
- Criar o 47º subcomando, `config path|list|get|set|unset`, em `src/commands/config_cmd.rs`.
- Uma tabela de 37 chaves tipadas alimenta `list`, `get`, `set` e `unset`: `fuzzy.mode`, `fuzzy.threshold`, `fuzzy.threshold_aggressive`, `fuzzy.threshold_context`, `fuzzy.threshold_jw`, `fuzzy.soft_floor_delta`, `fuzzy.max_content_bytes`, `write.confirm_large_bytes`, `log.file`, `log.rotation`, `storage.home`, `locale`, `concurrency.max_threads`, `concurrency.ram_fraction_percent`, `search.max_inflight_bytes`, `lock.acquire_timeout_ms`, `retry.base_ms`, `retry.max_ms`, `retry.max_elapsed_ms`, `extract.max_sample_rows`, `extract.max_text_chars`, `extract.max_sheets` e `extract.max_slides`.
- Faixas e relações ficam em `src/constants_config.inc.rs` e são aplicadas tanto pelo `config set` quanto pela carga na partida (por exemplo `concurrency.max_threads` de 1 a 256, `search.max_inflight_bytes` de 4 MiB a 1 GiB, `retry.base_ms` não pode passar de `retry.max_ms`).
- `--scope global|workspace` é obrigatório em `path`, `set` e `unset` e não tem padrão: sem ele o verbo sai com exit 2 e não toca em nada. `get` e `list` não aceitam `--scope` e mostram o valor com o seu `origin` (`default`, `global` ou `workspace`).
- `log.file`, `log.rotation`, `storage.home` e `locale` são só globais. O escopo workspace as rejeita com exit 65, e uma chave desconhecida ou um valor fora da faixa também sai com exit 65 e não grava nada.
- O `config set` lê o arquivo, muda a chave preservando os comentários, valida o arquivo inteiro e só então grava por `atomic_write` (arquivo global com modo 0600, sem backup).
- O evento terminal é `type: "config"` e traz `target_resolved` e `target_source: "argv"`. O arquivo XDG global fica fora do jail, então o dispatch tem um ramo de carimbo próprio para ele.
- O `config` é respondido antes da carga de configuração, então um `config.toml` quebrado não o bloqueia. Um arquivo que não é TOML válido é lido com um aviso e nunca é reescrito (`set` e `unset` saem com exit 78 citando o caminho).
- O `config.toml` global e o `.atomwrite.toml` do workspace são mesclados chave a chave, com o workspace vencendo. Antes desta versão o arquivo local substituía o global inteiro. `--config CAMINHO` ainda substitui os dois em `get` e `list`.

## Consequências
- Uma fonte única de chaves, faixas e padrões. Um novo valor de ajuste é uma linha na tabela e uma constante.
- `defaults.line_ending`, `defaults.backup`, `defaults.retention` e `defaults.max_filesize` são chaves do `config` e mudam o padrão de todo verbo mutador (gaps.md W14, P3).
- A `storage.home` é circular (ela decide onde a configuração mora), então o `set` a grava direto no arquivo de bootstrap e avisa no envelope para onde o config global se mudou.
- O `locale --set` agora grava a preferência por `atomic_write` (modo 0600) em vez de uma gravação simples.
- Os dois limites do extract que já tinham flag continuam funcionando: `--max-sample-rows` e `--max-text-chars` ainda vencem a configuração.

## Alternativas
- Manter os valores como literais: rejeitada, mudar um ajuste exigia recompilar e a regra de hardcode não deixava lugar legal para eles.
- Mandar o usuário editar o `config.toml` à mão: rejeitada, nada checava o valor e um erro de digitação só aparecia na execução seguinte.
- Um escopo padrão para o `set`: rejeitada, uma gravação global silenciosa é o padrão errado para uma ferramenta que agentes chamam sem terminal.
- Um verbo `config doctor`: não construído, listado como P3 no gaps.md.

## Evidência
- `CHANGELOG.md:48-57,134-135,222-224` (Added e Changed da 0.1.37) e `gaps.md:235-239` (W13), `gaps.md:278-280` (W37), `gaps.md:308` (W49).
- `src/commands/config_cmd.rs:277` (checagem de só global), `src/commands/config_cmd_keys.inc.rs`, `src/commands/config_cmd_write.inc.rs`, `src/config_tuning.inc.rs`, `src/constants_config.inc.rs`.
- `src/lib_dispatch.inc.rs:206-222` (`run_config_verb`, respondido antes da carga de configuração) e `src/lib.rs:201`.
- `tests/cli_v0137_config_verb.rs` (23 testes) e `docs/schemas/config.schema.json`.
- Binário 0.1.37 numa bancada com `HOME` e `XDG_*` dentro do scratchpad: `config get retry.max_ms` imprimiu `value` 400, `default` 400, `origin` `default` (exit 0). `config set fuzzy.mode auto` sem `--scope` saiu com exit 2 e `ARGUMENT_PARSE_ERROR`. `config set search.max_inflight_bytes 1048576 --scope workspace` saiu com exit 65 e "must be 4194304..=1073741824". `config set log.file true --scope workspace` saiu com exit 65. `config set fuzzy.max_content_bytes 1000000 --scope global` saiu com exit 0 e `target_source` `argv`, e o modo do arquivo ficou 600.

## Gatilho para revisar
- Surgir uma chave que nenhum escopo consiga guardar, ou a necessidade de um terceiro escopo.
- `defaults.line_ending` ou `defaults.max_filesize` ganhar um consumidor e virar chave.
