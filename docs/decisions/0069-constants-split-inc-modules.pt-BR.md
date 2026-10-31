# ADR-0069 — Divisão do `constants.rs` e dos módulos grandes em partes `.inc.rs`

## Status
Aceito (2026-10-15)

## Contexto
- O `src/constants.rs` tinha crescido além de um tamanho legível, e o guarda de hardcode (ADR-0068) o tornou o único lugar legal de todo literal do código de produção.
- O `output_shape.inc.rs` tinha 1951 linhas, e `run_replace`, `cmd_search` e `run` eram longos. O alvo de 500 linhas por arquivo de produção (CHANGELOG R40) é seguido por margem: nesta rodada quatro arquivos ainda passam disso (`commands/mod.rs` 520, `commands/query.rs` 513, `commands/search_stream.inc.rs` 507 e `lib_dispatch.inc.rs` 503) e estão declarados no `gaps.md`. Os módulos de teste `tests.inc.rs` ficam fora desse limite.

## Decisão
- Dividir o `constants.rs` por responsabilidade em três módulos `include!`: `constants_concurrency.inc.rs` (threads, retry, orçamento do search), `constants_config.inc.rs` (faixas e relações das chaves do config) e `constants_literals.inc.rs` (os literais nomeados que saíram do código de produção, começando por unidades de tempo e pela conversão de calendário).
- Dar a `config`, `fuzzy`, `ndjson` e `replace` seus próprios arquivos `.inc.rs` (`config_tuning.inc.rs`, `config_cmd_keys.inc.rs`, `config_cmd_write.inc.rs`, `part3.inc.rs`, `replace_file.inc.rs`), dividir o `output_shape.inc.rs` em nove arquivos, e mover o veredito de fechamento do `replace` para o `replace_finish.inc.rs`.
- Os caminhos públicos não mudam: `crate::constants::NOME` continua resolvendo, porque as partes são incluídas no mesmo módulo.
- Asserts de compilação ficam junto dos valores (por exemplo `MAX_AUTO_THREADS <= CONCURRENCY_MAX_THREADS_CEILING`).

## Consequências
- Nenhuma mudança de comportamento.
- O guarda de hardcode ignora arquivos `constants*` pelo nome, então um módulo novo de constantes precisa manter o prefixo `constants`.
- Os literais do inventário (nomes de sidecar e de backup com um único `BACKUP_INFIX`, unidades de tempo, limiares de risco, tamanhos de buffer e de bloco, caminhos de sonda de `/proc` e de contêiner, valores de errno, o BOM, nomes de thread e as constantes de calendário do `time_fmt.rs`) vivem em `constants*`.
- O `list` não carrega mais a sua própria cópia do algoritmo de calendário: ele chama `atomic::epoch_to_utc`.
- O `locale.rs` já tinha sido dividido antes pelo mesmo padrão (`locale_persist.inc.rs`).
- O `src/constants.rs` ainda tem 447 linhas e o `CANONICAL_SUBCOMMAND_COUNT` (47) mora nele.

## Alternativas
- Manter um `constants.rs` só: rejeitada, grande demais para ler e revisar.
- Transformar cada parte em um `mod` separado: não feito, as partes são incluídas para manter os caminhos `crate::constants::NOME` existentes.

## Evidência
- `CHANGELOG.md:131-133,186-188` (Changed) e `gaps.md:235-236` (W13).
- `src/constants.rs` (447 linhas) com `src/constants_concurrency.inc.rs` (105 linhas), `src/constants_config.inc.rs` (71 linhas) e `src/constants_literals.inc.rs` (271 linhas), `src/config_tuning.inc.rs` (369 linhas), `src/commands/config_cmd_keys.inc.rs` (451 linhas), `src/commands/config_cmd_write.inc.rs` (317 linhas).
- Correção, 2026-10-28: os tamanhos acima são os medidos depois de a tabela de chaves da 0.1.37 crescer para 35 chaves, porque os números anteriores desta lista estavam vencidos.
- `src/constants_config.inc.rs:66` e `:62-70` (asserts), `tests/cli_v0137_hardcode_guard.rs:8-10` (ignora `constants*`).

## Gatilho para revisar
- Uma parte de constantes passar de 500 linhas.
- O guarda mudar a forma de reconhecer os arquivos de constantes.
