# ADR-0068 — Testes herméticos e guardas de fonte

## Status
Aceito (2026-10-15)

## Contexto
- Depois do verbo `config` (ADR-0058), um teste que roda o binário passaria a depender do `config.toml` e do locale da máquina que o roda (gaps.md W38).
- O código de produção tinha acumulado literais de tempo e de sidecar, `let _ =` silenciosos que engolem erros, e panics de `expect`, `unreachable!` e regex estática (gaps.md W11, W12, W13).

## Decisão
- O `common::hermetic` (`tests/common/mod.rs`) dá a todo processo filho o seu próprio `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME` e `XDG_STATE_HOME` vazios, fixa `LANG` e `LC_ALL` em `en_US.UTF-8`, e remove `LANGUAGE` e `LC_MESSAGES`.
- O `XDG_CACHE_HOME` fica no cache real de propósito, porque a feature `ast` baixa parsers do tree-sitter ali em tempo de execução, e um cache vazio mandaria `outline`, `query` e `scope` para a rede (`real_cache_home`).
- Um teste que precisa do próprio config redireciona `HOME` e `XDG_*` depois de chamar o `hermetic`, e a chamada `env` posterior vence.
- Dois guardas de fonte quebram o build, ambos com lista de exceção vazia:
  - `cli_v0137_hardcode_guard` varre `src/` e rejeita `Duration::from_*` com literal numérico, o infixo `.bak.`, nomes de sidecar e de sonda, números de unidade de tempo (86400, 604800, 3600), `env::var*` e `Command::new`. Ele ignora arquivos `constants*`, arquivos de teste e comentários, e também falha se existir um arquivo `.sh`.
  - `cli_v0137_let_underscore_guard` rejeita `let _ =` no código de produção e `.unwrap()`, `.expect(`, `unreachable!(`, `panic!(`, `todo!(` e `unimplemented!(` fora de teste.
- Limpeza cuja falha não pode mudar o resultado passa por `src/best_effort.rs` (registrada em debug), e os outros descartes propagam quando mudam o resultado.

## Consequências
- Os testes não passam nem falham mais pela máquina do desenvolvedor.
- A hermeticidade é fato só quanto ao código do helper, porque o ambiente não foi envenenado de propósito (gaps.md W38). Os testes de `outline`, `query` e `scope` ainda dependem de um cache de parsers cheio (gaps.md W18, P3).
- 33 arquivos de teste usam o `hermetic` (`rg -l hermetic tests/*.rs`). O `CHANGELOG.md` e o gaps.md W38 dizem 31.
- 44 testes falham com `--features core` porque exigem a feature `ast` e não têm `#[cfg(feature = "ast")]` (gaps.md W19, P3).
- O `build.rs` chama o `git` e o guarda de fonte cobre só `src/` (gaps.md W44 B12, aceito).
- Um `.unwrap()` ou `let _ =` de produção agora quebra o build, então um novo precisa de um erro tipado ou de uma chamada a `best_effort`.

## Alternativas
- Envenenar o ambiente de propósito para provar o isolamento: não feito (gaps.md W38).
- Isolar também o `XDG_CACHE_HOME`: rejeitada, mandaria os testes de AST para a rede. No macOS o cache segue o `HOME` e não foi tratado, porque o macOS não foi executado.

## Evidência
- `CHANGELOG.md:73,80` e `gaps.md:225-234` (W11, W12), `gaps.md:235-239` (W13), `gaps.md:281-283` (W38).
- `tests/common/mod.rs:15-83` (`hermetic_home`, `real_cache_home`, `Hermetic`, `hermetic`).
- `tests/cli_v0137_hardcode_guard.rs:14-58` (regras e o `ALLOWED` vazio), `:146-168` (`violations_in`), `tests/cli_v0137_let_underscore_guard.rs:29` (`ALLOWED` vazio), `:177-216` (`PANIC_NEEDLES`, `ALLOWED_PANICS` vazio), `src/best_effort.rs` (133 linhas).
- `tests/cli_v0137_hardcode_guard.rs` (5 testes) e `tests/cli_v0137_let_underscore_guard.rs` (5 testes).

## Gatilho para revisar
- Um literal ou um panic precisar ser permitido. Ele entra então em `ALLOWED` com o motivo.
- A suíte rodar em hosts macOS e Windows.
