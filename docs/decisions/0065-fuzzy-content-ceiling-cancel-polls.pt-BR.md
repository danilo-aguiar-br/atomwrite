# ADR-0065 — Teto de conteúdo do fuzzy e polls cooperativos de cancelamento

## Status
Aceito (2026-10-15)

## Contexto
- Toda estratégia fuzzy custa O(linhas), então um arquivo enorme podia rodar por minutos além de `--timeout-secs` (gaps.md W4). Com um orçamento de 2 s e um padrão que nunca casa, o `replace` levou 6,16 s em 50 MB e 18,6 s em 150 MB, e o `edit` levou 5,73 s e 17,7 s.
- Os laços de `strategies`, `windows`, `collect`, `rank`, `cascade`, `util` e `escape_map` nunca consultavam o cancelamento. O `windows.rs` e o `rank.inc.rs` devolviam `None`, que a cascata transformava em `MatchFailed` (exit 65) mesmo com o prazo já vencido.
- As checagens de cancelamento eram copiadas em cada laço.

## Decisão
- `[fuzzy].max_content_bytes` (padrão 256 MiB, `FUZZY_MAX_CONTENT_BYTES`, chave de config `fuzzy.max_content_bytes`) é um teto acima do qual a cascata não roda.
- Acima dele, o `edit` sai com exit 65 `INVALID_INPUT` citando a chave e `--fuzzy off`, e o `replace` registra o arquivo em `files_failed`. Um match exato e o `--fuzzy off` continuam funcionando acima do teto.
- A `escape_normalized` é pulada acima de `FUZZY_ESCAPE_MAP_MAX_CONTENT_BYTES` (32 MiB) sem construir o seu mapa de offsets, e a cascata segue com as outras estratégias.
- Uma primitiva única de cancelamento, o `CancelCheck` (granularidade de bloco, `src/signal_cancel.inc.rs`), é dividida pelos laços de varredura. Os laços fuzzy usam o `CancelPoll` (um contador local mais uma máscara, `src/fuzzy/util.rs`), e o `check_cancel()?` roda depois de cada estratégia e antes do `MatchFailed`.
- Só um miss real (`MatchFailed`, `MatchAmbiguous`) vira zero matches. `Cancelled`, `InvalidInput` e erros internos chegam ao chamador em `edit`, `edit-loop`, `replace` e `batch` com o seu próprio código.
- Duas dicas localizadas dedicadas, `suggestion.fuzzy-content-cap` e `suggestion.fuzzy-pattern-cap`, explicam as recusas e não embutem mais o `reason` em inglês.
- No ramo Jaro-Winkler de linha única, só um candidato que passa o dual-gate conta como empate (gaps.md W52): um bloco ausente entre linhas parecidas é `MATCH_FAILED` com `best_candidate`, e um empate real continua `MATCH_AMBIGUOUS`.

## Consequências
- Com um orçamento de 2 s, as mesmas fixtures levam 2,02 s no `replace` e no `edit` (50 MB e 150 MB), todas com exit 124 e um envelope `cancelled` (binário debug, gaps.md W4).
- Um arquivo de 896,4 MB é recusado pelo teto: `edit --old XXNOPE` sai com exit 65 `INVALID_INPUT` em 1808 ms em release (V1), dentro da meta de 3,5 s. O arquivo recusado ainda é lido inteiro antes da recusa (RSS igual ao tamanho do arquivo, gaps.md W17, P3).
- Mudança de comportamento no `batch`: uma op `replace` cujo match fuzzy é ambíguo agora falha a op, onde antes era relatada como "sem matches".
- A dica pt-BR fica inteira em pt-BR e cita `atomwrite config set fuzzy.max_content_bytes <BYTES> --scope global` e `--fuzzy off`. A linha W23 do `gaps.md` ainda lista o P3 antigo, escrito antes de as dicas dedicadas existirem (o V1 FAIL-5 está fechado na W39).

## Alternativas
- Sem teto, só polls: rejeitada, em 900 MB o `edit` levou 4,19 s com exit 124 no binário debug, acima da meta de 3,5 s, porque o arquivo é lido, hasheado e dividido em linhas antes de a cascata começar. O teto recusa o arquivo em 1808 ms em release.
- Copiar a checagem de cancelamento em cada laço: rejeitada, substituída por uma primitiva.

## Evidência
- `CHANGELOG.md:18,58-60,79,148-150,224,234,261` e `gaps.md:165-168` (W4), `gaps.md:243` (W23), `gaps.md:247-248` (W17), `gaps.md:314-316` (W52).
- `src/constants.rs:205` (`FUZZY_MAX_CONTENT_BYTES`) e `:215-219` (32 MiB, com assert abaixo do teto), `src/config.rs:234-246`, `src/fuzzy/types.inc.rs:54`, `src/fuzzy/escape_map.inc.rs:65`, `src/fuzzy/util.rs:68-110` (`check_cancel`, `CancelPoll`), `src/signal_cancel.inc.rs:63` (`CancelCheck`).
- `tests/cli_v0137_cancel.rs` (10 testes, incluindo os testes de prazo fuzzy do gaps.md W4).
- Binário 0.1.37: `config get fuzzy.max_content_bytes` devolveu 268435456.

## Gatilho para revisar
- Um `stat` antes da leitura ser adicionado, para evitar o pico de leitura de um arquivo recusado.
- O padrão de 256 MiB ser mudado.
