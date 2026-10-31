# ADR-0056 — Remover o teto de nível em tempo de compilação `release_max_level_info` do `tracing`

## Status
Aceito (2026-10-19)

## Contexto
- O `Cargo.toml` compilava `tracing` com `features = ["release_max_level_info"]`.
- Essa feature remove do binário release toda chamada `debug!` e `trace!` em tempo de compilação.
- A CLI mapeia `-vv` para `debug` e `-vvv` para `trace` (`runtime::cli_log_level`) e monta um `EnvFilter` com esse nível.
- No binário release o filtro pedia níveis que o binário não continha mais.
- O `tracing-subscriber` então imprimia um aviso colorido, fora de JSON, no stderr ("some trace filter directives would enable traces that are disabled statically").
- Medido no binário release anterior à mudança: `-vv read` e `-vvv read` escreviam esse aviso ANSI no lugar de registros JSON, e nenhum registro `DEBUG` existia.
- O canal stderr está documentado como um objeto JSON por linha, então o aviso quebrava o contrato do agente (BUG-3).
- A regra do projeto `rules-rust-tracing-desempenho-testes-observabilidade` favorece tetos em tempo de compilação para release. Este ADR registra o desvio deliberado.

## Decisão
1. Declarar `tracing = "0.1"` sem nenhuma feature `release_max_level_*`.
2. Manter o filtro em tempo de execução como único controle de nível (`-v`, `-vv`, `-vvv`, `-q`, `-qq`).
3. Ampliar `diagnostic_records_are_distinguishable_from_payload` (`tests/cli_v0137_agent_native.rs`) para rodar `-vv` e `-vvv`, exigir stderr só com JSON e exigir ao menos um registro com `level` igual a `DEBUG`.
4. Rodar esse teste em debug e com `--release`, porque só o perfil release aplicava o teto.

## Medição
- Dois binários release foram compilados do mesmo snapshot de código, em diretórios de target separados, diferindo só na feature do `tracing`.
- Carga: `search foo tree400 -j 8 --no-progress` na fixture de 10,7 milhões de linhas, saída para `/dev/null`.
- Sete execuções intercaladas (antes, depois, antes, depois, …), porque outros processos usavam o host de 72 núcleos.
- Tempo de parede em segundos: `antes` mín 40,47, mediana 42,44, máx 53,30. `depois` mín 39,77, mediana 43,12, máx 53,29.
- DERIVADO: (43,12 - 42,44) / 42,44 × 100 = 1,60 % na mediana, abaixo do gatilho de 2 % e dentro da dispersão de 12,83 s só das execuções `antes`.
- Tamanho do binário: 54 358 528 bytes antes, 54 375 744 bytes depois. DERIVADO: 54 375 744 - 54 358 528 = 17 216 bytes (0,03 %).
- O plano B (`release_max_level_debug`, com `-vvv` limitado a debug) portanto NÃO foi necessário.

## Consequências
- `-vv` e `-vvv` emitem só registros JSON, incluindo `DEBUG` (e `TRACE` no `-vvv`), em builds release.
- Pontos de chamada desligados ainda custam uma leitura atômica cada em tempo de execução. A medição não mostra custo fora do ruído.
- Um teto futuro também precisa alterar `cli_log_level`, ou o aviso volta.

## Alternativas
- `release_max_level_debug`: mantém o `-vv`, mas ainda quebra o `-vvv`. Rejeitada porque exige uma regressão medida para se justificar.
- Silenciar o aviso do subscriber: esconde o sintoma enquanto o binário continua sem atender o `-vvv`.

## Gatilho para revisar
- Regressão medida da mediana acima de 2 % fora da faixa de ruído em uma árvore representativa.
