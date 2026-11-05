# ADR-0095 — os offsets de submatch do search são de byte, e o texto é renderizado de forma lossy


## Status
- Aceito (2026-11-03)
- Registra o contrato de `Submatch` no `search` e a correção local do panic com multibyte


## Contexto
- O `search` publica os offsets de submatch que `grep_regex::RegexMatcher::find_iter` produziu, e o matcher roda sobre bytes
- O `grep-regex 0.1.14` compila o padrão com `utf8(false)` e configura o meta regex com `utf8_empty(false)`, porque o searcher lê os bytes crus do arquivo
- O `grep-matcher 0.1.9` avança um byte depois de cada match vazio, então um padrão que pode casar vazio é pesquisado em todo offset de byte
- Um match de largura zero pode cair entre as unidades de código de um caractere multibyte, e um padrão orientado a byte como `(?-u)\xA7` pode casar um único byte de um deles
- O `extract_submatches` fatiava `&line[start..end]`, que exige limite de caractere, então o panic abortava o processo com exit 134 sob `panic = "abort"`
- O `--format text` e o NDJSON chegavam à mesma função, então os dois podiam abortar


## Decisão
- `Submatch.start` e `Submatch.end` continuam offsets de byte dentro da string `lines` publicada, porque é isso que o matcher devolve
- O texto de `match` é renderizado com `String::from_utf8_lossy` sobre a faixa de bytes, nunca por indexação de `&str`
- Um match de largura zero publica `match: ""` com `start == end`, inclusive dentro de um caractere multibyte
- Um match que não é UTF-8 válido publica o caractere de substituição daquele trecho em vez de derrubar o processo
- A função é total, então uma faixa fora da linha devolve trecho vazio em vez de um segundo panic
- Nenhum segundo matcher é compilado, porque o `RegexMatcherBuilder` público não expõe opção `utf8` e um duplicado bifurcaria a semântica
- Os offsets, as formas dos eventos e os códigos de saída não mudam


## Consequências
- O consumidor que lê `match` recebe render lossy para padrões orientados a byte, e texto exato para todo match em limite de caractere
- O consumidor que lê `start` e `end` mantém os mesmos offsets de byte que recebia antes
- O panic desaparece dos dois formatos, inclusive do `recipe search-replace-verify`, que chama a mesma busca
- Um padrão que pode casar vazio passa a responder em vez de abortar, então o conjunto de respostas é maior do que o crash era
- O campo `match` pode conter U+FFFD para um trecho dentro de caractere multibyte, que é a renderização honesta de bytes que não são caractere


## Alternativas
- Compilar um segundo matcher com `utf8_empty(true)` só para a extração de submatch, recusado porque o `RegexMatcherBuilder` não tem método `utf8` e dois matchers bifurcariam a semântica
- Pular os matches cujos offsets não são limite de caractere, recusado porque descarta um match que o matcher reportou e perde o valor de `--column` de um padrão orientado a byte
- Remover as flags orientadas a byte do padrão, recusado porque muda em silêncio o que o chamador pediu para casar
- Converter todo panic de produção em erro tipado, recusado como fora de escopo porque este defeito é local e o hook global é outro contrato


## Gatilho para revisitar
- Revisitar se o `grep-regex` expuser configuração de matcher segura para UTF-8
- Revisitar se o `Submatch` passar a publicar offsets relativos aos bytes crus do arquivo em vez da string lossy de `lines`
