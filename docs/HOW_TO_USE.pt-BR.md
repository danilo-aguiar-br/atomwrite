# Como Usar o atomwrite


[Read in English](HOW_TO_USE.md)

- Uma CLI substitui dezenas de chamadas de ferramenta de arquivo que seu agente faz hoje


## Objetivos
- Gastar menos tokens: um objeto NDJSON por chamada substitui o encadeamento de shell, e o `commands --brief` responde em 5.020 bytes onde a árvore completa custa 67.492
- Reduzir o custo financeiro, porque a conta acompanha os tokens e as idas e voltas, e uma chamada substitui as várias chamadas de shell que uma tarefa de arquivo pede
- Reduzir os erros com escrita atômica e checksum BLAKE3 em toda resposta, então um resultado velho ou parcial aparece em vez de ser presumido
- Reduzir as alucinações com discriminador `type`, schema fechado por verbo e código de saída que nomeia a falha
- Reduzir o tempo gasto em cada tarefa, porque a leitura, a busca e a escrita rodam num processo só em vez de numa corrente de ferramentas de shell
- Reduzir a latência com execução local e sem rede no caminho do arquivo, medida nas medianas de 5,6 ms no `read` e 6,7 ms no `write` de arquivo pequeno
- Multiplicar a velocidade das operações com `batch`, `edit-loop`, `replace`, `case`, `transform` e `codemod` resolvendo muitos arquivos numa chamada
- Reduzir o lead time entre a solicitação e o resultado verificado, porque a resposta já traz o checksum, o `target_resolved` e o próximo passo
- Tornar o agente mais rápido, mais veloz, mais eficaz e mais assertivo, já que o código de saída, o `code` e o `suggestion` dizem o que fazer sem tentar às cegas


## Tempo de Leitura e de Execução
- O tempo de leitura é de cerca de 53 minutos para o guia inteiro, DERIVADO 12296 palavras fora dos blocos de código a 230 palavras por minuto
- O Primeiro Comando em 60 Segundos leva 2 minutos para ler e rodar
- Todo exemplo rodou no Linux contra o atomwrite 0.1.38 dentro de um diretório home isolado
- Um write simples terminou em 8 ms no host da medição, então os exemplos respondem na hora
- A primeira chamada de AST baixa os parsers do tree-sitter, o que leva alguns segundos (veja Pré-requisitos)
- macOS e Windows não foram medidos para este guia
- Os exemplos usam um shell POSIX, e no PowerShell você envia o texto por pipe com `'text' | atomwrite ...`
- Quando terminar, o [COOKBOOK](COOKBOOK.pt-BR.md) traz receitas mais longas


## O Que Há de Novo na v0.1.38 (2026-11-04)
- O envelope de falha carrega `ok`, `code` e `exit`, e o `error: true` saiu porque duplicava `ok: false` mais `code`, custava 13 bytes em toda falha e nenhum consumidor decidia por ele sozinho; quem o lia passa a ler `.ok == false`
- O `read --format raw` para em 24.000 bytes e imprime uma linha no stderr com o `--lines` que lê o resto, e o `--all` desliga o teto
- O `read --format raw` de alvo único honra `--max-filesize` e sai 65 `FILE_TOO_LARGE` em vez de transmitir um arquivo acima do teto
- `--symbol NOME`, `--line N --enclosing`, `--lines` repetível e `--no-comments` escolhem a janela pela árvore sintática
- O `outline` lê um título Markdown como uma `section` que vai até o fim da própria seção, e o `read --symbol Part` devolve essa seção inteira com as subseções
- Um arquivo cuja assinatura de imagem está corrompida falha no `read` com exit 65 `INVALID_INPUT` em vez de devolver os bytes como texto
- Uma imagem chega como uma linha `data:`, o `--high-res` sobe a faixa para 2.576 pixels e 4.784 tokens visuais, e uma imagem já enviada nesta sessão responde com uma linha curta de token, a menos que `--force` seja dado
- `read doc.pdf` devolve o texto de cada página, `--pages` seleciona janelas, `needs_vision` nomeia a página sem texto, e `--ocr` reconhece caracteres numa imagem ou numa página renderizada em Rust puro
- O `search --format text` escreve uma linha `caminho:linha:coluna:texto` por match, e uma árvore em que todo arquivo era binário sai 89 `ALL_SKIPPED_BINARY` em vez de `NO_MATCHES`
- O `apply` casa linhas inteiras, aplica vários arquivos numa invocação, publica as linhas mudadas com `--show` e ordena os alvos antes de tomar os locks
- O lock mira o inode com `std::fs::File::try_lock`, e o `delete` toma o mesmo lock antes de calcular o hash e antes do unlink, então uma exclusão e uma gravação no mesmo alvo não se entrelaçam mais
- O `delete -r <dir>` domina a árvore nomeada inteira, incluindo entradas ocultas, ignoradas, links, fifo, socket e dispositivo, e um link nunca é seguido
- O `target_resolved` é sempre uma lista na ordem de invocação, porque um caminho pode conter a vírgula que antes unia vários alvos
- O `[defaults].backup = true` alcança o `delete` e o `batch delete`, e o `[delete].backup` vence sobre ele só no `delete`
- O `output.profile = "lean"` tira os digests, as contagens de bytes e o tempo de um registro de mutação bem-sucedida, o que levou um `apply` de três linhas de 433 bytes para 196
- O `config` lista 43 chaves, e o `atomwrite commands --brief` imprime uma linha por subcomando em 5.020 bytes contra 67.492 da árvore completa
- Fixe os agentes em `^0.1.38`. 47 subcomandos (SSOT)


## O Que Há de Novo na v0.1.37 (2026-10-31)
- Fixe os agentes em `^0.1.37` com `cargo install atomwrite --locked --version "^0.1.37"`
- A superfície tem 47 subcomandos (SSOT), e o novo é o `config`
- O `config` lê e grava 37 chaves persistentes com `path|list|get|set|unset|doctor`
- `--scope global|workspace` é obrigatório em `config path`, `config set` e `config unset`
- As flags globais de stream `--sort-by`, `--dedupe`, `--aggregate` e `--diff-against` moldam o canal de dados
- `--join KEY:SUFFIX` lê uma tabela NDJSON do stdin uma vez e anexa as linhas casadas a cada evento de dados
- As flags globais `--redact`, `--normalize-encoding`, `--max-output-bytes` e `--allow-truncate` protegem o que sai do processo
- `--max-output-bytes` e `--allow-truncate` são rejeitadas com exit 2 se as duas não vierem juntas
- `--print-schema [NAME]` lista os 55 nomes de schema ou emite um schema
- `--non-interactive` é aceita em todo comando, enquanto `--confirm-side-effect` é aceita nos verbos que mudam bytes em disco e recusada com exit 65 nos demais
- O `extract-artifact` lê cabeçalhos de PDF, OOXML e imagem e nunca emite bytes brutos do artefato
- Todo envelope de sucesso traz `ok: true`, e uma falha termina em um único evento terminal com `ok: false`
- Todo resumo que publica `files_failed` publica `partial` ao lado, e isso cobre `search`, `replace`, `transform`, `scope`, `count`, `list`, `sparse`, `semantic-search` e `case`
- `search`, `replace`, `transform`, `scope`, `case`, `sparse`, `semantic-search` e `backup` mantêm exit 0 quando responderam e algum arquivo falhou, enquanto `count`, `list`, `hash`, `copy`, `delete` e `prune-backups` saem com a primeira falha porque a resposta deles é um total sobre a árvore; o contrato é o ADR-0077 (`docs/decisions/0077-answer-family-and-walk-honesty.md`)
- O `edit-loop` é tudo ou nada, e o `edit-loop --partial` sai com 87 (`EDIT_LOOP_PARTIAL`)
- Um match fuzzy acima de `[fuzzy].max_content_bytes` (256 MiB) falha com exit 65, e `--fuzzy off` segue só exato
- Os exit codes para ramificar são 65, 78, 83, 84, 87, 124, 129, 141 e 143
- O lock de escrita é automático no Unix e no Windows, e um estouro de tempo sai com 83 (`LOCK_TIMEOUT`)
- O `watch` saiu do build padrão, e o binário padrão responde a ele com exit 78

### Mudanças Que Podem Quebrar um Consumidor
- Um resumo que falha carrega `ok`, `code` e `exit` nele mesmo, e nenhuma segunda linha de erro vem depois
- Leia `partial` e `files_failed` em todo resumo que os publica antes de confiar num resultado, e leia `errors[]` como as primeiras `summary.errors_max_entries` causas (50 por padrão)
- Varreduras canceladas terminam num envelope `cancelled` com `deadline_exceeded`, então a última linha pode ser um registro de cancelamento
- `--print-schema` deve vir antes do nome do comando, e um nome desconhecido sai 65 com um envelope de erro
- `replace`, `scope`, `codemod`, `case`, `delete`, `backup` e `prune-backups` exigem caminhos explícitos, ou saem 2
- `wal-heal` e `scratch init` recebem o caminho posicional, e `recipe run` exige `--path` ou `--target`
- O `transform` exige seus caminhos posicionais, e uma chamada sem eles é erro de uso
- O `edit-loop` com um par sem match sai 65 e não grava nada, onde antes gravava os pares que casavam
- Um par ausente relata `MATCH_FAILED` com `best_candidate`, onde antes relatava `INVALID_INPUT`
- O `doctor` perdeu os campos de CI e privilégio, e o `agent-surface` perdeu a chave de protocolo
- Uma chave desconhecida ou um arquivo inválido em `config.toml` ou `.atomwrite.toml` sai 78
- O `.atomwrite.toml` do workspace e o `config.toml` global são mesclados chave a chave
- O `watch` exige um build com `--features watch`, ou a chamada sai 78


## Pré-requisitos
- Toolchain Rust 1.90 ou superior (edição 2024)
- Linux, macOS e Windows são suportados
- Instale pelo crates.io e confirme a versão

```bash
cargo install atomwrite --locked --version "^0.1.38"
atomwrite --version
```

- Espere uma linha que começa com `atomwrite 0.1.37`
- O build padrão tem as features `core`, `ast`, `lang-rust`, `lang-ts` e `lang-py`
- `watch` e `lang-go` são opt-in, e `--features full` liga tudo
- O binário padrão responde ao `watch` com exit 78 e a dica `cargo install atomwrite --features watch`
- Os parsers do tree-sitter não vêm compilados, e são baixados na primeira vez que uma linguagem é usada
- `outline`, `query`, `scope`, `transform`, `codemod`, `sparse outline` e `write --syntax-check` disparam o download
- O download é um pacote de 23.031.807 bytes para linux-x86_64 (DERIVADO: 23031807 / 1048576 = 21,97 MiB), e ele se desdobra em 3.991.360 bytes em 6 bibliotecas compartilhadas (DERIVADO: 3991360 / 1048576 = 3,81 MiB), medido com o cache frio em 2026-10-31
- Os parsers ficam em `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/`, e `~/.cache/...` é o padrão no Linux
- O pacote vem do release `v1.22.1` de `xberg-io/tree-sitter-language-pack`, cujo manifesto lista 372 linguagens, e o cliente HTTP segue o proxy do sistema
- Para um host sem rede, preencha antes o diretório de cache num host conectado (veja a ADR-0057)
- O `atomwrite doctor` publica o check `grammar_cache` com quantas das 23 gramáticas que os verbos de AST podem precisar já estão em disco, então o operador sabe a condição antes da execução e não no meio dela
- Gramática ausente que não pode ser baixada é exit 69 `GRAMMAR_UNAVAILABLE` com classe `precondition_failed`, e nunca exit 255, porque o host está sem rede e a ferramenta não está quebrada
- O `human-panic` grava o relatório de panic como arquivo local no diretório temporário e não envia nada a ninguém
- Veja [INSTALL.pt-BR.md](INSTALL.pt-BR.md) para as ferramentas de build do Windows e a solução de problemas


## Primeiro Comando em 60 Segundos
- Escolha um diretório de trabalho e guarde o caminho absoluto dele uma única vez
- `--workspace` ancora o jail, então um valor errado move o jail em silêncio

```bash
mkdir -p demo && cd demo
WS="$(pwd)"
printf 'hello world\n' | atomwrite --workspace "$WS" write src/hello.txt
```

- Espere uma linha NDJSON no stdout com `type` igual a `write` e `ok` igual a `true`
- A linha traz `path`, `bytes_written`, `checksum`, `elapsed_ms`, `wal_policy` e `platform`
- Ela traz também `target_resolved` e `target_source`, que provam qual caminho foi gravado

```json
{"type":"write","status":"success","path":"src/hello.txt","bytes_written":12,"checksum":"dc5a4edb8240b018124052c330270696f96771a63b45250a5c17d3000e823355","ok":true,"target_source":"argv"}
```

- Este é um trecho, e a linha real tem mais campos
- Leia de volta com metadados e checksum

```bash
atomwrite --workspace "$WS" read src/hello.txt
```

- Espere `type` `read` com `content`, `lines`, `bytes`, `checksum`, `permissions`, `modified` e `ok`
- O stdout é sempre NDJSON, o stderr só leva diagnósticos, e o exit code vem primeiro
- Toda escrita sobrevive a um crash, porque passa por tempfile, fsync e rename


## Comandos Principais
- Cada comando tem um resumo, suas flags principais, um exemplo testado e a saída esperada
- Os exemplos compartilham um workspace de exemplo, então rode-os em ordem dentro de `demo`

### Workspace de Exemplo
- Crie os arquivos que os próximos exemplos usam

```bash
mkdir -p src tmp
printf 'fn main() {\n    // TODO: wire config\n    let v = load().unwrap();\n    println!("{}", v);\n}\n\nfn load() -> Option<i32> {\n    Some(1)\n}\n' > src/main.rs
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n' > src/lib.rs
printf 'fn alpha() {}\n' > src/a.rs
printf 'fn beta() {}\n' > src/b.rs
printf '[package]\nname = "demo"\nversion = "0.1.0"\n\n[database.pool]\nmax = 4\n' > Cargo.toml
```

### write
- Cria ou sobrescreve um arquivo de forma atômica a partir do stdin, na ordem tempfile, fsync, rename, fsync-dir
- Seus dados chegam ao disco ou a operação falha de forma limpa, e o sucesso traz um `checksum` BLAKE3

```bash
printf 'fn main() {}\n' | atomwrite --workspace "$WS" write src/scratch.rs
printf 'longer content for the file\n' | atomwrite --workspace "$WS" write --backup --keep-backup src/hello.txt
printf 'checked write\n' | atomwrite --workspace "$WS" write --dry-run src/dry.txt
```

- Espere `type` `write` nos dois primeiros e `type` `plan` com `would_modify` no dry run
- O backup é feito por padrão e removido após o sucesso, e `--keep-backup` mantém o irmão `.bak.<timestamp>`
- Um write que falha sempre mantém o backup, `--no-backup` desliga os backups e `--backup` com `--no-backup` sai 2
- `--expect-checksum HASH` grava só se o checksum atual casar, ou sai 82 (`STATE_DRIFT`)

```bash
CS="$(atomwrite --workspace "$WS" hash src/hello.txt | jaq -r .checksum)"
printf 'longer content, checked write\n' | atomwrite --workspace "$WS" write --expect-checksum "$CS" src/hello.txt
```

- `--append` e `--prepend` acrescentam ao arquivo, e `--line-ending lf|crlf|cr|auto` normaliza os fins de linha
- `--durability full|fast|auto` escolhe a força do fsync, e o padrão `auto` usa full só para arquivos de config
- `--wal-policy auto|always|never` controla o sidecar de journal, e `always` o força até em writes triviais
- `--syntax-check` analisa o resultado com tree-sitter e sai 88 (`SYNTAX_ERROR_DETECTED`) em caso de erro

```bash
printf 'fn ok() {}\n' | atomwrite --workspace "$WS" write --syntax-check src/ok.rs
printf 'tail line\n' | atomwrite --workspace "$WS" write --append src/hello.txt
```

- Uma reescrita que encolhe o arquivo em mais de 50% exige `--allow-shrink`, ou sai 65
- Um alvo existente acima de 100 KiB (`[write].confirm_large_bytes`) exige `--ack-overwrite`, ou sai 65
- Um stdin vazio sai 65 a menos que você passe `--allow-empty-stdin`
- `--preserve-timestamps` mantém o mtime antigo, e por padrão o mtime é atualizado para as ferramentas de build recompilarem
- Outras flags são `--max-size`, `--require-backup`, `--auto-rotate`, `--risk-threshold PERCENT`, `--require-large-ack` e `--retention N`
- `--no-checksum-when-empty` deixa um stdin vazio pular o `--expect-checksum`
- `--require-backup` e `--auto-rotate` não recebem valor, e `--risk-threshold` é um percentual com padrão 255 (desligado)
- O sucesso relata `platform.durability`, `platform.rename_method` e `platform.backup_method`
- Um SIGTERM durante um write remove o tempfile e sai 143

### read
- Lê um arquivo com `content`, `lines`, `bytes`, `checksum`, `permissions` e `modified`
- O evento traz também `target_resolved` e `target_source`, o mesmo par que os verbos de escrita publicam, então o agente confirma o caminho que leu
- Arquivos binários são detectados, e o conteúdo deles é omitido

```bash
atomwrite --workspace "$WS" read --stat src/main.rs
atomwrite --workspace "$WS" read --lines 1:3 src/main.rs
atomwrite --workspace "$WS" read --head 2 src/main.rs
atomwrite --workspace "$WS" read --tail 2 src/main.rs
atomwrite --workspace "$WS" read --line 3 --context 1 src/main.rs
atomwrite --workspace "$WS" read --grep unwrap src/main.rs
atomwrite --workspace "$WS" read --format raw --head 2 src/main.rs
atomwrite --workspace "$WS" read --format raw src/main.rs src/cli.rs
atomwrite --workspace "$WS" read --format raw 'src/*.rs'
```

- Espere `type` `read` com `mode` `stat` ou `full`, e `lines_total` quando você lê uma fatia
- `--format ndjson` é o padrão, e `--format raw` escreve os bytes no stdout sem envelope
- `--bytes` fica para pipelines antigos e significa o mesmo que `--format raw`
- A resposta padrão para em `read.max_output_bytes` numa linha completa, alinhada ao fim do último item do outline, e publica `truncated` mais uma faixa `next` pronta
- `--format raw` imprime a próxima faixa no stderr e separa faixas não contíguas com `--`, então o leitor sabe onde uma faixa termina
- Vários operandos concatenam na ordem do argv, e operando que nomeia um caminho existente é lido como está
- Operando com `*`, `?` ou `[` que não existe é expandido pelo binário num nível do diretório dele, em ordem de byte, descartando nome oculto
- Arquivo ausente, diretório ou glob sem resultado imprime uma linha no stderr e os outros alvos ainda chegam ao stdout, com o status da primeira falha
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` e `--verify-checksum` aceitam um alvo só e saem 65 quando vários são nomeados
- `**` não é suportado, e só o nome final pode ser padrão: curinga no nome de um diretório sai 65 em vez de expandir pela metade
- `--verify-checksum HASH` sai 81 (`CHECKSUM_VERIFY_FAILED`) com um envelope de erro quando não casa
- `--grep REGEX` mantém só as linhas que casam, e regex inválida sai 65
- `--no-comments` exige `--number`, porque tirar linhas sem manter o número do próprio arquivo apontaria o `apply` seguinte para a linha errada, e sem isso a chamada sai 65
- Imagem é respondida como uma linha `data:` no stdout, porque essa é a forma que o canal de visão aceita
- O tipo da imagem vem dos bytes mágicos, e o tamanho segue a regra de blocos de 28 pixels
- `--max-tokens N` reduz a imagem até a contagem arredondada de blocos caber em N, e `--crop X,Y,W,H` recorta a região antes da redução
- `--high-res` leva o teto para 2576 pixels e 4784 tokens visuais, e a faixa padrão fica em 1568 de cada
- A orientação EXIF é aplicada, e uma imagem acima de 100 megapixels é recusada antes do decode
- Arquivo cujos magic bytes nomeiam uma imagem mas cujo cabeçalho está quebrado é recusado com exit 65 `INVALID_INPUT`, e o nome fica em `target_resolved`
- Imagem HEIC e HEIF é decodificada como qualquer outra imagem, e a imagem parada é convertida depois da decodificação, então o canal de visão recebe um formato que ele lê
- `--stat` em imagem reporta largura, altura, formato, visual tokens e contagem de quadros do GIF sem enviar pixels
- Imagem já entregue nesta sessão é respondida com a linha de token `[image delivered earlier in this session: <hash>]`, porque uma segunda cópia dos mesmos pixels custa a imagem inteira de novo
- `--force` envia a imagem de novo dentro da mesma sessão, e é o único jeito de repetir uma entrega que o cache de sessão pularia
- `read doc.pdf` devolve o texto de cada página, e `--pages N` ou `--pages A:B` escolhe janelas, repetível numa chamada só
- A página que não tem texto é nomeada em `needs_vision`, porque página digitalizada não é documento vazio
- `--page N` renderiza uma página e responde com o mesmo `data:` URI que a imagem usa, então digitalização nunca chega ao modelo como cabeçalho sozinho
- `--dpi N` escolhe a resolução do render, e `--stat` informa o tamanho da renderização sem enviá-la
- `--ocr` reconhece os caracteres de uma imagem ou de uma página renderizada e responde com o texto em `content`, exatamente onde o arquivo de texto o põe
- Os modelos de reconhecimento são baixados uma vez para o diretório de cache, então a primeira execução com `--ocr` precisa de rede e as seguintes não
- A página renderizada vai para reconhecimento a 300 dpi por padrão, porque letra miúda a 150 dpi fica grossa demais para separar os caracteres
- `--crop` melhora o reconhecimento numa imagem larga, porque o motor lê melhor uma linha que ocupa mais do quadro
- `--ocr` num arquivo que tem texto sai 65 em vez de fingir, porque a flag lê pixels e aquele arquivo tem caracteres

### edit
- Edita um arquivo por texto exato, por número de linha ou por marcador, e a escrita é atômica
- Uma linha de sucesso traz `checksum_before`, `checksum_after`, `lines_before`, `lines_after` e `mtime_preserved`

```bash
atomwrite --workspace "$WS" edit src/main.rs --old 'wire config' --new 'wired config'
atomwrite --workspace "$WS" edit src/a.rs --old 'alpha' --new 'alpha_one' --old '{}' --new '{ 1 }'
printf '    // inserted\n' | atomwrite --workspace "$WS" edit src/main.rs --after-line 1
printf 'one\ntwo\nthree\nfour\n' | atomwrite --workspace "$WS" write tmp/five.txt
printf 'TWO\n' | atomwrite --workspace "$WS" edit tmp/five.txt --range 2:2
atomwrite --workspace "$WS" edit tmp/five.txt --delete-range 1:1
```

- Espere `type` `edit`, e pares repetidos de `--old`/`--new` acrescentam `pairs_total` e `pair_results`
- Os modos de posição leem o texto novo do stdin: `--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`
- Um terminal no stdin faz esses modos saírem 65 na hora, em vez de esperar
- `--delete-range N:M` não precisa de stdin, e `--multi` aplica várias edições NDJSON numa escrita só
- O `--multi` lê um objeto JSON por linha do stdin, nunca um array JSON, e aplica tudo numa única escrita atômica
- Cada objeto nomeia seu `op`: `exact`, `insert-after`, `insert-before`, `replace-range` ou `delete-range`
- O campo de conteúdo segue o op: `content` nos três ops de linha e no `replace-range`, `old` e `new` no `exact`, e nenhum no `delete-range`
- Campo que o op escolhido não lê sai 65 e nada é escrito, então um `new` perdido nunca vira exclusão silenciosa
- Os números de linha valem para o arquivo ORIGINAL, e duas operações que tocam a mesma linha são recusadas antes de qualquer aplicação

```bash
printf "%s\n" \
'{"op":"replace-range","start":2,"end":2,"content":"DOIS"}' \
'{"op":"insert-after","line":4,"content":"cinco"}' \
'{"op":"delete-range","start":6,"end":6}' \
| atomwrite --workspace "$WS" edit tmp/linhas.txt --multi
```

- O exemplo troca a linha 2, insere depois da linha 4 e apaga a linha 6, tudo sobre a mesma numeração original
- `--fuzzy auto|aggressive|off` trata casos aproximados, `off` significa só exato, e `auto` é o padrão
- `--fuzzy-threshold 0.0-1.0` ajusta a cascata, e um resultado pode trazer `fuzzy`, `strategy` e `similarity`
- Um miss sai 65 com `MATCH_FAILED`, `match_count` 0 e, quando existe, um `best_candidate`
- Vários matches exigem `--replace-all`, e a linha de sucesso passa a relatar `match_count`
- Uma indentação realinhada acrescenta `indent_adjusted: true`
- `--partial` aplica os pares que casam e relata o resto, com exit 1 quando nenhum par foi aplicado
- `--expect-checksum HASH` protege contra drift, e `--allow-sequential-drift` aceita uma cadeia de edições suas
- `--old-file` e `--new-file` leem o texto de arquivos que vivem dentro do workspace (veja Padrões Avançados)
- Nunca faça pipe do `edit` para o `jaq` sem checar `${PIPESTATUS[0]}`, pois o envelope de erro vai ao stdout

### search
- Busca conteúdo de arquivos em paralelo com o motor do ripgrep
- Cada arquivo recebe um evento `begin`, seus eventos `match` e um evento `end`, e um `summary` fecha a execução

```bash
atomwrite --workspace "$WS" search 'TODO' src/
atomwrite --workspace "$WS" search --regex 'fn\s+\w+' src/
atomwrite --workspace "$WS" search --count 'fn' src/
atomwrite --workspace "$WS" search --files 'fn' src/
atomwrite --workspace "$WS" --filter type=match search -F 'println!' src/
atomwrite --workspace "$WS" search --target files lib src/
```

- Espere um evento `match` com `path`, `line_number`, `lines`, `byte_offset` e `submatches`
- Exit 1 com `NO_MATCHES` significa zero matches, e não é um crash
- `--regex` (`-e`), `--fixed` (`-F`), `--word` (`-w`), `--case-insensitive` (`-i`) e `--smart-case` (`-S`) escolhem o modo do padrão
- `--context N` (`-C`), `--max-count N` (`-m`), `--invert`, `--multiline` (`-U`) e `--pcre2` (`-P`) refinam o casamento
- Padrão que começa com `-` vai depois de `--`, como no `rg`: `search -F -- '-deprecated' src/`; sem isso o token é flag desconhecida e a execução sai 2
- `--count` (`-c`) e `--files` (`-l`) mudam a forma dos eventos, e `--target content|files|both` escolhe o que é buscado
- `--include` (`-g`) e `--exclude` filtram por glob, e `--sort path|modified|created|none` ordena os arquivos
- `--max-filesize BYTES` pula arquivos maiores (padrão 10 MiB, contados em `files_skipped`), e `--max-columns N` corta linhas longas
- `--binary` busca também em arquivos binários, e `--include-fifo` abre named pipes, que o padrão pula
- `--no-begin-end` suprime os eventos begin e end de arquivo sem match, o que mantém limpa uma busca vazia
- `--offset N` e `--limit N` paginam os eventos de match
- Leia `partial` e `files_failed` no resumo antes de confiar num resultado, e raiz nomeada na invocação que não existe reprova a execução com exit 4 mesmo tendo casado, em todo resumo que os publica

### replace
- Substitui texto em vários arquivos com escrita atômica, e o argumento de caminho é obrigatório
- Strings fixas usam a cascata fuzzy depois de um miss exato, e `--regex` mantém o motor de regex

```bash
atomwrite --workspace "$WS" replace --dry-run alpha_one omega src/
atomwrite --workspace "$WS" replace --preview alpha_one omega src/
atomwrite --workspace "$WS" replace alpha_one omega src/a.rs
atomwrite --workspace "$WS" replace --fuzzy auto --fuzzy-threshold 0.85 --progress-every 25 --dry-run omega alpha_one src/
```

- Espere `type` `plan` por arquivo no dry run, `type` `preview` com um diff, e `type` `replaced` numa escrita
- Toda execução termina num `summary` com `files_modified` e `total_replacements`
- Zero matches saem 1 com `NO_MATCHES`, e um resumo final traz `ok: false`
- `--dry-run` e `--preview` mostram o plano, então use-os antes de qualquer mudança em massa
- `--literal` (`-F`) desliga a regex, `--word` (`-w`) casa palavras inteiras e `--preserve-case` adapta a caixa
- `--max-replacements N` (`-n`) limita os applies fuzzy, com padrão 1 quando omitido e teto rígido de 10000
- Os applies fuzzy são one-pass da esquerda para a direita, e o texto inserido nunca é varrido de novo
- Quando o texto novo contém o texto antigo, um apply único é forçado, então expandir uma seção é seguro
- `--include`, `--exclude`, `--expect-checksum` e `--preserve-timestamps` funcionam como no `edit`
- `--progress-every N` emite heartbeats de progresso em árvores grandes, e `--no-progress` os silencia
- Um `replace` bem-sucedido apaga o `.bak` como qualquer outro mutador, e `--keep-backup` é o único jeito de mantê-lo


## Comandos Utilitários

### hash
- Calcula o checksum BLAKE3 de arquivos ou do stdin
- Leia o campo `checksum`, e nunca o antigo campo `value`

```bash
atomwrite --workspace "$WS" hash src/main.rs
printf 'data\n' | atomwrite --workspace "$WS" hash --stdin
atomwrite --workspace "$WS" hash --recursive --exclude '*.bak*' src/
H="$(atomwrite --workspace "$WS" hash src/lib.rs | jaq -r .checksum)"
atomwrite --workspace "$WS" hash --verify "$H" src/lib.rs
```

- Espere `type` `hash` com `algorithm` `blake3`, `checksum`, `bytes` e `ok`
- `--verify HASH` responde com `type` `hash_verify` e `verified`, e sai 81 quando não casa

### delete
- Apaga arquivos ou árvores, não mantém cópia sem `--backup`, que cria um `.bak.<timestamp>`, e pode imprimir só o plano
- Apagar nunca pede confirmação, então use `--plan` antes

```bash
printf 'x' > tmp/old.txt
atomwrite --workspace "$WS" delete --plan tmp/old.txt
atomwrite --workspace "$WS" delete tmp/old.txt
mkdir -p tmp/d && printf '1\n' > tmp/d/f
atomwrite --workspace "$WS" delete --recursive tmp/d
```

- Espere `type` `plan` com `would_modify`, depois `type` `deleted` com `checksum_before`, depois um `summary`
- Um diretório sem `--recursive` (`-r`) sai 65 e nomeia a flag que falta
- `delete -r <dir>` sem filtro de estreitamento é dono da árvore nomeada inteira e remove entrada oculta, ignorada, link, fifo, socket e dispositivo, e o link nunca é seguido
- O run estreitado por `--include`, `--exclude`, `--older-than` ou `--limit` sai 0 com `dirs_left` e `entries_left`, e o run sem filtro que deixa um diretório sai com o código de I/O medido e o caminho sobrevivente em `errors[]`
- `--hidden`, `--no-gitignore` e `--follow-symlinks` são no-op no `delete` e só emitem uma linha de diagnóstico
- `--dry-run` funciona como `--plan`, e `--older-than 1d` filtra por idade com os sufixos s, m, h, d e w
- `--include` (`-g`) e `--exclude` filtram por glob, e `--no-backup` é aceito e desliga a cópia que `[defaults].backup = true` pediria, porque o delete só copia quando a chamada pede
- `delete --confirm` e `delete --yes` (`-y`) são rejeitados com exit 65, então rode de novo sem eles
- `--backup` com raiz de diretório é recusado antes de qualquer mudança, porque a cópia nasceria dentro da árvore, então rode `copy -r` antes ou passe `--no-backup`
- A cópia de um delete nunca é removida sozinha, então limpe-a com `prune-backups`; `--keep-backup` só acrescenta um aviso e `--backup` é quem cria a cópia

### count
- Conta linhas, linhas em branco e bytes, por extensão ou por tamanho

```bash
atomwrite --workspace "$WS" count src/
atomwrite --workspace "$WS" count --by-extension src/
atomwrite --workspace "$WS" count --by-size --top 3 src/
```

- Espere `type` `count` com `mode` `lines`, `by_extension` ou `by_size`
- `--include` (`-g`) e `--exclude` filtram a varredura

### diff
- Compara dois arquivos, com saída unificada, estatística ou mudança por mudança
- Um caminho `-` lê aquele lado do stdin

```bash
atomwrite --workspace "$WS" diff src/a.rs src/b.rs
atomwrite --workspace "$WS" diff --stat src/a.rs src/b.rs
atomwrite --workspace "$WS" diff --unified --context 5 src/a.rs src/b.rs
atomwrite --workspace "$WS" diff --algorithm patience src/a.rs src/b.rs
```

- Espere eventos `change` e um `summary` final com `identical` e `similarity_ratio`
- `--stat` emite `type` `diff` com `insertions` e `deletions`, e `--unified` emite o patch em `content`
- `--algorithm` aceita `myers`, `patience` ou `lcs`

### move
- Move ou renomeia um arquivo ou diretório de forma atômica
- Entre dispositivos ele recorre a copiar e apagar, e relata `cross_device`

```bash
atomwrite --workspace "$WS" copy src/lib.rs src/lib_copy.rs
atomwrite --workspace "$WS" move src/lib_copy.rs src/lib_moved.rs
```

- Espere `type` `moved` com `source`, `target`, `checksum`, `cross_device` e `atomic`
- Um destino existente exige `--force` (`-f`) ou `--backup`, ou a chamada sai 65
- `--preserve-hardlinks` mantém os hard links, e `--dry-run` mostra o plano

### copy
- Copia arquivos ou árvores e verifica o checksum BLAKE3 depois da escrita

```bash
atomwrite --workspace "$WS" copy --force src/lib.rs src/lib_copy.rs
atomwrite --workspace "$WS" copy --recursive --preserve src/ tmp/srccopy
```

- Espere `type` `copied` com `checksum` e `verified` igual a `true`
- `--recursive` (`-r`) copia árvores, `--preserve` mantém metadados e `--preserve-xattr` mantém atributos estendidos
- `--no-reflink` força uma cópia de bytes, e o reflink é o padrão onde o filesystem o suporta
- Os backups usam `reflink_or_copy` e nunca um hard link do arquivo vivo

### list
- Lista uma árvore de diretórios com um evento NDJSON por entrada

```bash
atomwrite --workspace "$WS" list --long --depth 1 src/
atomwrite --workspace "$WS" list --count-by-ext src/
atomwrite --workspace "$WS" list --include '*.rs' --depth 1 src/
```

- Espere `type` `entry` com `path` e `kind`, mais `size` e `modified` com `--long`
- O `summary` final traz `files`, `dirs`, `symlinks` e `total_bytes`
- `--all` inclui as entradas ocultas, e `--exclude` filtra por glob

### extract
- Extrai campos de NDJSON lido do stdin, ou colunas de texto puro
- Use chaves posicionais ou `--field` repetível, e nunca os dois na mesma chamada
- `--select` é uma flag global, então `extract --select` não é válido

```bash
printf 'a b c\n' | atomwrite extract 0 2
printf '{"a":1,"b":2}\n' | atomwrite extract --field a
atomwrite --workspace "$WS" search 'TODO' src/ | atomwrite extract --field path --field line_number
printf '{"a":1,"b":2}\n' | atomwrite --select a extract --field a
```

- Espere `{"type":"text","values":["a","c"]}` para colunas de texto e `{"a":1}` no caso NDJSON
- `--delimiter` (`-d`) define o separador de colunas, e `--stdin` lê o fluxo de entrada

### extract-artifact
- Extrai sinal delimitado de cabeçalhos de PDF, OOXML (documentos, planilhas, slides) e imagem
- Ele nunca devolve os bytes brutos do artefato

```bash
printf '\211PNG\r\n\032\n\000\000\000\rIHDR\000\000\000\001\000\000\000\001\010\006\000\000\000\037\025\304\211' > img.png
atomwrite --workspace "$WS" extract-artifact img.png
```

- Espere `type` `artifact` com `kind` `image`, `format` `png`, `width` e `height`
- `--max-sample-rows` e `--max-text-chars` vencem as chaves de config `extract.*`

### calc
- Avalia contas e conversões de unidades com o motor fend

```bash
atomwrite calc "2 hours + 30 minutes to seconds"
atomwrite calc "sqrt(144) + 3^2"
atomwrite calc "10 GiB to MB"
```

- Espere `type` `calc` com `expression` e `result`, como `9000 seconds`
- Coloque a expressão entre aspas, porque o shell a dividiria, e `--stdin` a lê da entrada

### regex
- Gera uma expressão regular a partir de strings de exemplo, com o motor grex

```bash
atomwrite regex "2024-01-15" "2025-12-31" "2026-06-01"
atomwrite regex abc-12 xyz-99 id-7 --digits
atomwrite regex --digits --words "user_123" "admin_456"
```

- Espere `type` `regex` com `regex`, `examples` e `anchored`
- `--digits` (`-d`), `--words` (`-w`), `--spaces` (`-s`), `--repetitions` (`-r`) e `--case-insensitive` (`-i`) alargam o padrão
- `--no-anchors` tira `^` e `$`, e `--stdin` lê os exemplos da entrada


## Comandos Avançados

### scope
- Seleciona código pela gramática e age sobre ele, com `--delete`, `--replace-with` ou `--action`
- Exige `--language` (`-l`, alias `--lang`) e exatamente um entre `--query` e `--pattern`

```bash
atomwrite --workspace "$WS" scope src/main.rs --language rust --query comments --delete --dry-run
atomwrite --workspace "$WS" scope --query fn --action upper -l rust --dry-run src/lib.rs
atomwrite --workspace "$WS" scope --pattern 'a + b' --action upper -l rust --dry-run src/lib.rs
```

- Espere `type` `scoped` com `scopes_matched`, `bytes_before` e `bytes_after`, depois um `summary`
- O `summary` traz `total_matches` como a soma de `scopes_matched` da execução, ao lado de `files_visited` e `files_matched`
- O `files_matched` conta os arquivos que casaram algo, e arquivo que falhou ou foi pulado nunca conta
- `--action` aceita `upper`, `lower`, `titlecase`, `squeeze`, `symbols` e `normalize`
- As queries incluem `comments`, `strings`, `fn`, `pub-fn`, `async-fn`, `struct`, `enum`, `trait`, `impl`, `mod` e `use`
- Rode sempre `--dry-run` antes de `--delete`, e tire a flag para aplicar

### transform
- Busca e reescreve código por padrão de AST com o motor ast-grep
- `$NAME` captura um nó, `$$$ARGS` captura vários, e `-p`/`--pattern` e `-r`/`--rewrite` são usados juntos

```bash
atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/
atomwrite --workspace "$WS" transform --dry-run -p 'println!($$$ARGS)' -r 'tracing::info!($$$ARGS)' -l rust src/
```

- Espere `type` `transformed` com `matches` e `replacements`, depois um `summary`
- Exit 1 significa zero matches, e os caminhos são obrigatórios
- `--rules FILE` e `--inline-rules YAML` aplicam uma lista de regras, no formato mostrado em `codemod`, e o resumo único soma as falhas de cada regra em `files_failed`, `partial` e `errors[]` ao lado dos contadores
- Arquivo que falha conta uma vez em `files_failed` e uma vez em `errors[]`, qualquer que seja o número de regras, porque cada regra varre a árvore inteira
- `files_visited`, `files_matched` e `total_replacements` continuam somando por regra, porque cada regra é uma passagem completa
- `--verify-parse` confere se o resultado ainda faz parse, e `--include` e `--exclude` filtram arquivos

### batch
- Roda operações de um manifesto NDJSON, com `write`, `replace`, `delete`, `edit`, `hash`, `move` e `copy`
- `--transaction` torna tudo ou nada, e uma falha desfaz

```bash
printf '%s\n' '{"op":"write","target":"batch.txt","content":"x"}' '{"op":"hash","target":"batch.txt"}' | atomwrite --workspace "$WS" batch --transaction
printf '%s\n' '{"op":"write","target":"batch.txt","content":"y"}' | atomwrite --workspace "$WS" batch --dry-run
```

- Espere um `batch_op` por linha e um `summary` final com `succeeded`, `failed` e `committed`
- `--file PATH` lê o manifesto de um arquivo, e o manifesto é lido uma linha por vez
- As operações `move` e `copy` aceitam `"force":true` para sobrescrever
- `--input-schema` imprime o JSON Schema do manifesto, e `--batch-size N` define quantas operações ficam em buffer antes do resumo

### apply
- Aplica um patch do stdin: diff unificado, blocos SEARCH/REPLACE, markdown com cerca ou arquivo inteiro
- A detecção é automática, e `--format auto|unified|search-replace|full|markdown` força um formato
- Um corpo sem marcador nenhum é recusado com exit 65, e só `--format full` substitui o arquivo inteiro, então patch malformado deixou de virar sobrescrita silenciosa
- O `--format full` percorre as mesmas duas barreiras do `write`: `--ack-overwrite` acima de `[write].confirm_large_bytes` e `--allow-shrink` abaixo de `[write].shrink_block_percent`
- `--strict-atomic` sai 91 em vez de cair para cópia entre dispositivos, e `--no-syntax-check` pula a checagem de parse que recusa patch que quebra o arquivo
- Um cabeçalho `---`/`+++` sem hunk `@@` é recusado com exit 65, porque aplicá-lo como arquivo inteiro sobrescreveria o alvo com o texto do patch

```bash
printf 'new body\n' | atomwrite --workspace "$WS" apply --format full tmp/applied.txt
printf -- '--- a/src/b.rs\n+++ b/src/b.rs\n@@ -1 +1 @@\n-fn beta() {}\n+fn beta_v2() {}\n' > tmp/fix.patch
atomwrite --workspace "$WS" apply --dry-run src/b.rs < tmp/fix.patch
atomwrite --workspace "$WS" apply src/b.rs < tmp/fix.patch
```

- Espere `type` `plan` no dry run e `type` `applied` com `format_detected`, `hunks_applied` e `backup_path` quando a execução fez cópia
- Um diff unificado deve casar o contexto e as linhas removidas dentro de 64 linhas, ou sai 65 com `MATCH_FAILED`
- Um bloco SEARCH/REPLACE ausente falha o lote inteiro, e nada é gravado
- O argumento de alvo é opcional: patch que nomeia os próprios arquivos é aplicado numa chamada, e todo arquivo é checado antes da primeira gravação
- Patch SEARCH/REPLACE nomeia o arquivo com uma linha de caminho logo acima do primeiro bloco daquele arquivo, e o patch unified usa o próprio par de cabeçalho para a mesma função
- A recusa de um arquivo o nomeia em `target_resolved`, e todo arquivo aplicado recebe o próprio registro com o próprio caminho e checksums
- Duas seções para o mesmo arquivo são recusadas, e gravação que falha depois de outro arquivo gravado restaura os arquivos já escritos e publica um registro `rollback` com a contagem

### set
- Grava um valor num caminho com pontos em um arquivo TOML ou JSON e mantém comentários e ordem das chaves
- O tipo é inferido do valor, então `9` vira inteiro e `true` vira booleano

```bash
atomwrite --workspace "$WS" set Cargo.toml package.version 0.2.0
atomwrite --workspace "$WS" set Cargo.toml database.pool.max 9
printf '{"scripts":{"build":"tsc"}}\n' > package.json
atomwrite --workspace "$WS" set package.json scripts.build "tsc -b"
```

- Espere `type` `set` com `key_path`, `old_value`, `new_value`, `format` e `comments_preserved`
- O `set` aceita as flags de backup e `--preserve-timestamps`, e não precisa de flag de tipo
- Use `config set` para as chaves de ajuste do próprio atomwrite

### get
- Lê um valor num caminho com pontos em um arquivo TOML ou JSON

```bash
atomwrite --workspace "$WS" get Cargo.toml package.version
atomwrite --workspace "$WS" get package.json scripts.build
```

- Espere `type` `get` com `value`, `found` e `format`
- Uma chave ausente sai 65 com `INVALID_INPUT`, e não devolve `found: false`
- Um arquivo que não é TOML ou JSON válido também sai 65

### del
- Remove uma chave num caminho com pontos em um arquivo TOML ou JSON

```bash
atomwrite --workspace "$WS" del Cargo.toml database.pool.max
atomwrite --workspace "$WS" del --force-missing Cargo.toml package.metadata.deprecated
```

- Espere `type` `del` com `removed_value` e `existed`
- `--force-missing` transforma uma chave ausente em sucesso com `existed` igual a `false`

### case
- Renomeia um identificador em arquivos de código e converte o nome novo para um estilo de caixa
- `--to` aceita `snake` (padrão), `camel`, `pascal`, `kebab` e `screaming-snake`
- Passe arquivos, porque um diretório sai 0 e não muda nada (`identifiers_total` é 0)

```bash
printf 'fn user_id() {}\nlet userId = 1;\n' > src/c.rs
atomwrite --workspace "$WS" case --subvert user_id account_id --to camel --dry-run src/c.rs
atomwrite --workspace "$WS" case --subvert user_id account_id --to camel src/c.rs
```

- Espere `type` `case_preview` no dry run e `type` `case` com `before` e `after` numa escrita
- Um `summary` final relata `identifiers_total` e `files_modified`
- Nomeie vários arquivos para renomear em um módulo, e rode `--dry-run` antes numa árvore grande

### query
- Percorre a AST do tree-sitter de um arquivo e emite os nós como NDJSON

```bash
atomwrite --workspace "$WS" query --kinds src/lib.rs
atomwrite --workspace "$WS" query --query function_item --positions src/lib.rs
atomwrite --workspace "$WS" query --tree src/lib.rs
```

- Espere eventos `query_kind`, `query_match` ou `query_summary` com `kind`, `text` e offsets de bytes
- `--query KIND` (`-Q`) filtra por tipo de nó, `--kinds` lista os tipos e `--tree` lista todos os nós
- `--positions` acrescenta linha e coluna, e `--language` define a linguagem quando a extensão não basta
- Uma query inválida sai 65, e um build slim sem `ast` sai 78

### outline
- Emite a estrutura de alto nível de um arquivo de código: funções, structs, enums, traits e módulos
- Exige um arquivo, e um diretório não é aceito

```bash
atomwrite --workspace "$WS" outline src/lib.rs
atomwrite --workspace "$WS" outline --kind function_item --positions src/lib.rs
```

- Espere eventos `outline_item` com `kind`, `name`, `signature`, `start_line` e `end_line`, depois um `outline_summary`
- `--kind` é repetível, e `--positions` acrescenta bytes e colunas
- Um build slim sem `ast` sai 78 com uma dica de rebuild


## Comandos de Segurança e Manutenção

### backup
- Cria uma cópia `.bak.<timestamp>` de um arquivo, com checksum BLAKE3

```bash
atomwrite --workspace "$WS" backup src/lib.rs --retention 3
atomwrite --workspace "$WS" backup --dry-run src/lib.rs
```

- Espere `type` `backup` com `backup_path` e `checksum`, depois um `summary` com `files_backed_up`
- `--retention N` mantém os N backups mais novos, `--output-dir DIR` grava em outro lugar e `--dry-run` mostra o plano

### rollback
- Restaura um arquivo a partir de um backup anterior, e `--verify` confere antes o checksum BLAKE3

```bash
atomwrite --workspace "$WS" rollback src/lib.rs --latest --verify --dry-run
atomwrite --workspace "$WS" rollback src/lib.rs --latest --verify
```

- Espere `type` `plan` no dry run e `type` `rollback` com `restored_from` e `verified`
- `--timestamp STAMP` restaura um backup específico, e um prefixo do stamp é aceito
- O snapshot de segurança pré-rollback é opt-in por `--backup`

### verify
- Confere um arquivo contra um hash BLAKE3 esperado, com exit 0 se casa e 81 se não
- O hash é posicional, então a chamada é `verify PATH HASH`

```bash
H="$(atomwrite --workspace "$WS" hash src/lib.rs | jaq -r .checksum)"
atomwrite --workspace "$WS" verify src/lib.rs "$H"
```

- Espere `type` `hash_verify` com `verified` igual a `true` e `ok` igual a `true`
- Quando não casa, o mesmo evento traz `verified: false`, `code` `CHECKSUM_VERIFY_FAILED` e `exit` 81

### edit-loop
- Aplica N pares antigo/novo num arquivo em uma escrita só, tudo ou nada
- Envie um array JSON ou um objeto JSON por linha no stdin

```bash
printf 'alpha\nbeta\ngamma\n' > tmp/loop.txt
printf '[{"old":"alpha","new":"ALPHA"},{"old":"beta","new":"BETA"}]\n' | atomwrite --workspace "$WS" edit-loop tmp/loop.txt
printf '{"old":"gamma","new":"GAMMA"}\n{"old":"absent","new":"X"}\n' | atomwrite --workspace "$WS" edit-loop --partial tmp/loop.txt
```

- Espere `type` `result` com `pairs_total`, `pairs_applied`, `pairs_unmatched` e `pair_results`
- Sem `--partial`, um par sem match sai 65 com `MATCH_FAILED` e `failed_pair_index`, e nada é gravado
- Com `--partial`, os pares que casam são gravados e a execução sai 87 (`EDIT_LOOP_PARTIAL`) se algum par falhou
- `--syntax-check`, `--line-ending` e `--allow-sequential-drift` também são aceitas

### prune-backups
- Remove os irmãos `.bak` dos arquivos-alvo nomeados, por idade ou por quantidade
- Nomeie os arquivos-alvo, e não um diretório, e passe `--max-age-secs` ou `--max-count`
- `--dry-run` é uma flag sem valor, então omita-a para aplicar

```bash
atomwrite --workspace "$WS" prune-backups --max-age-secs 0 --dry-run src/hello.txt
atomwrite --workspace "$WS" prune-backups --max-age-secs 0 src/hello.txt
atomwrite --workspace "$WS" prune-backups --max-count 3 src/lib.rs
```

- Espere eventos `pruned` com `reason` `age` ou `count`, depois um `summary` cujo `action` é `dry_run` ou `pruned`
- `--dry-run false` não aplica nada, porque `false` é lido como um caminho

### wal-stats
- Mostra o estado dos journals WAL sob o workspace, só leitura

```bash
atomwrite --workspace "$WS" wal-stats
```

- Espere `type` `wal_stats` com `total_journals`, `by_state`, `oldest_journal_age_secs` e `auto_heal_recommended`

### wal-heal
- Remove journals terminais mais antigos que um limite, e mantém os que ainda estão em `Started`
- Exige um caminho, e `--threshold-secs` tem padrão 3600

```bash
atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .
atomwrite --workspace "$WS" wal-heal --max-duration-ms 200 .
```

- Espere `type` `wal_heal` com `removed`, `preserved`, `malformed` e `bytes_reclaimed`
- `--max-duration-ms` limita o orçamento de relógio, e `--dry-run` mostra o plano
- Um crash (`kill -9`) pode deixar um journal em `Started`, e o `wal-heal` o preserva para você inspecionar
- A flag global `--no-auto-heal` pula o passe de heal na partida, que roda só em comandos que gravam
- Uma execução cancelada (SIGTERM, SIGINT ou prazo) não deixa journal para trás


## Comandos de Orquestração

### recipe
- Lista e roda receitas embutidas de vários passos: `search-replace-verify` e `edit-loop-syntax-check`
- O `recipe run` exige `--path` ou `--target`, e os passos da receita pulam arquivos `*.bak.*`

```bash
atomwrite --workspace "$WS" recipe list
atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern beta --replacement omega --path src --dry-run
```

- Espere eventos `recipe` na lista e um `recipe_result` final com `steps`, `dry_run` e `ok`
- Uma receita cuja busca não acha nada para no passo de replace com exit 1 e `failed_step_id`
- A `edit-loop-syntax-check` exige `--pairs-file FILE`, `--target FILE` e `--syntax-check LANGUAGE`
- `--fuzzy`, `--fuzzy-threshold`, `--include` e `--exclude` ajustam a receita search-replace

### sparse
- Lista, lê e faz outline de árvores grandes sob um orçamento de arquivos e bytes

```bash
printf 'src/lib.rs\nsrc/b.rs\n' > tmp/paths.txt
atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576 --all
atomwrite --workspace "$WS" sparse read --paths-file tmp/paths.txt --head 2
atomwrite --workspace "$WS" sparse outline --max-files 5 --all src/
```

- Espere eventos `sparse_entry`, `sparse_read` ou `outline_item` e um `sparse_summary` com `truncated`
- O `sparse list` aceita também globs `--include` e `--exclude`, e um glob inválido sai 65
- `--all` soma o dotfile à visita, então `files_scanned` cresce pelas entradas ocultas e a execução padrão as mantém de fora
- O `sparse outline` relata `sparse_outline_budget`, e um build slim sem `ast` sai 78

### semantic-merge
- Mescla três arquivos linha a linha (base, ours, theirs), e não entende AST
- Os marcadores de conflito são gravados por padrão, `--write-conflict-markers true|false` declara isso e `--no-conflict-markers` desliga

```bash
printf 'a\nb\nc\n' > tmp/base.txt
printf 'a\nB\nc\n' > tmp/ours.txt
printf 'a\nb\nC\n' > tmp/theirs.txt
atomwrite --workspace "$WS" semantic-merge --base tmp/base.txt --ours tmp/ours.txt --theirs tmp/theirs.txt --output tmp/merged.txt --fail-on-conflict
```

- Espere `type` `semantic_merge` com `status` `merged` e os três checksums de entrada
- Um conflito real tem `status` `conflict`, e `--fail-on-conflict` o transforma em exit 84 (`MERGE_CONFLICT`)
- `--expect-checksum` e as flags de backup protegem o arquivo de saída

### semantic-search
- Ranqueia arquivos e linhas por similaridade Jaccard offline com uma consulta, e está sempre compilado

```bash
atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01
atomwrite --workspace "$WS" semantic-search "fn" src/ --k 2 --index-dir .atomwrite/semantic-index
```

- Espere eventos `semantic_match` com `rank`, `score`, `path`, `line` e `snippet`, depois um `semantic_summary`
- O `backend` é `jaccard`, ou `inverted-index` quando `--index-dir` é usado
- `--include-backups` varre também os arquivos `.bak`, e zero hits não é um crash

### codemod
- Aplica uma lista YAML de regras de reescrita por AST numa árvore
- Cada regra é um item com `id`, `language`, `pattern` e `rewrite`

```bash
printf -- '- id: unwrap-to-try\n  language: rust\n  pattern: "$E.unwrap()"\n  rewrite: "$E?"\n' > rules.yaml
atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/
```

- Espere eventos `transformed` e um `codemod_summary` com `by_rule_id`
- Um arquivo de regras no estilo ast-grep, com `rule:` e `fix:`, sai 65
- `transform --rules` e `--inline-rules` leem o mesmo formato

### watch
- Transmite eventos do sistema de arquivos com debounce e sempre termina com um `watch_summary`
- Exige um build com `--features watch`, e o build padrão sai 78 com uma dica de rebuild

```bash
atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500
```

- No build padrão, espere `code` `CONFIG_INVALID` e exit 78
- Num build com `watch`, espere `watch_summary` como última linha, até num sinal
- Passe sempre `--max-events` ou `--idle-exit-ms` (a saída por ociosidade padrão é 500 ms), ou `--timeout-secs`
- `--debounce-ms MS`, `--checksum` e `--gitignore true|false` ajustam o fluxo

### stat
- Devolve só metadados, como alias de `read --stat`

```bash
atomwrite --workspace "$WS" stat src/lib.rs
```

- Espere `type` `read` com `mode` `stat`, mais `lines`, `bytes` e `checksum`


## Comandos de Descoberta e do Host
- Use estes antes de inventar listas de flags à mão

### doctor
- Diagnostica o host e relata checks de heal, concorrência, features e ambiente
- Ele não lê nenhuma variável de ambiente, e `--strict` sai 1 (`DOCTOR_CHECKS_FAILED`) quando um check falha

```bash
atomwrite --workspace "$WS" doctor --strict
```

- Espere `type` `doctor_report` com `ok` e um array `checks`
- No build padrão, um check informa que o `watch` está desabilitado

### commands
- Imprime a árvore completa de comandos em JSON, com ou sem os argumentos globais

```bash
atomwrite --workspace "$WS" commands --include-globals
```

- Espere `type` `commands_tree` com 47 `commands` e, com a flag, 33 `global_args`
- Aliases como `--max-concurrency`, `--timeout`, `--fields` e o `--json` oculto não aparecem na lista

### agent-surface
- Imprime o manifesto de ferramentas para agentes, e é a fonte única dos subcomandos

```bash
atomwrite --workspace "$WS" agent-surface --format json
```

- Espere `type` `agent_surface` com `tools`, `exit_codes`, `discovery`, `flag_contract` e `notes`
- `tools` tem 47 entradas, e `exit_codes` tem 42 entradas com 34 números distintos
- `discovery` lista os nomes de `--print-schema`, e o `watch` relata `available: false` no build padrão

### locale
- Mostra, grava ou limpa o idioma de exibição, e é distinto da `--locale` global
- A precedência é `--locale`, depois a preferência salva, depois o idioma do sistema, depois `en`

```bash
atomwrite locale
atomwrite locale --set pt-BR
atomwrite locale --clear
```

- Espere `type` `locale_report` com `resolved`, `source`, `available` e `preference_path`
- Só existem `en` e `pt-BR`, e qualquer outra tag sai 2 com a lista de tags disponíveis
- A preferência fica em `$XDG_CONFIG_HOME/atomwrite/locale`
- O `code` e a `message` do erro ficam em inglês, e a `suggestion` segue o locale, com `suggestion_lang` dizendo qual

### completions
- Imprime um script de completion de shell, ou o instala sob o diretório de dados XDG
- Os shells suportados são bash, zsh, fish, elvish e powershell

```bash
atomwrite completions bash > atomwrite.bash
atomwrite completions bash --install
```

- Espere o script cru no stdout na primeira chamada e um evento `installed` na segunda
- Esse evento é `{"type":"installed","path":"<arquivo de completion>","ok":true,"target_resolved":["<mesmo caminho>"],"target_source":"argv"}`
- O caminho vem do diretório de dados XDG do shell, como listado em Onde Ficam os Arquivos
- Um write que falha porque o leitor fechou o pipe sai 141 (`BROKEN_PIPE`), sem panic e sem relatório de falha
- Uma saída bem curta, como `--version | head -1`, pode terminar antes de o leitor fechar e então sai 0

### scratch
- Cria o diretório de staging sob o jail do workspace, e nunca sob o diretório temporário do SO
- O `scratch init` exige um caminho

```bash
atomwrite --workspace "$WS" scratch init .atomwrite/scratch
```

- Espere `type` `scratch_init` com `path`, `relative` e `already_existed`
- Estagie o conteúdo de `--old-file` e `--new-file` ali, porque caminhos fora do jail saem 126

### config
- Lê e grava chaves de ajuste persistentes no `config.toml` global ou no `.atomwrite.toml` do workspace
- `list` e `get` não precisam de escopo, e `set`, `unset` e `path` exigem `--scope global|workspace`

```bash
atomwrite --workspace "$WS" config get fuzzy.threshold
atomwrite --workspace "$WS" config list
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace
atomwrite --workspace "$WS" config path --scope workspace
atomwrite --workspace "$WS" config unset fuzzy.threshold --scope workspace
```

- Espere `type` `config` com `action`, `key`, `value`, `origin` e `default`
- `origin` é `default`, `global`, `workspace`, ou `explicit` quando `--config PATH` é usado
- As 43 chaves, suas faixas e escopos estão na seção Configuração


## 47 Subcomandos (SSOT)
- Fixe os agentes em `^0.1.38`, e leia `atomwrite agent-surface` para o manifesto vivo
- `CANONICAL_SUBCOMMAND_COUNT` é 47, e a árvore para máquinas é `atomwrite commands --include-globals`

| Comando | Resumo |
|---|---|
| `read` | Lê um arquivo com checksum, tamanho e linhas, ou só uma fatia |
| `write` | Cria ou sobrescreve um arquivo de forma atômica a partir do stdin |
| `edit` | Edita por texto exato ou aproximado, número de linha ou marcador |
| `delete` | Apaga arquivos ou árvores, dono da árvore nomeada inteira no `-r`, copiando um só quando a chamada pede (`--backup` ou `[defaults].backup = true`), ou imprime o plano |
| `copy` | Copia um arquivo ou árvore com verificação de checksum |
| `move` | Move ou renomeia um arquivo de forma atômica |
| `apply` | Aplica um diff unificado, SEARCH/REPLACE ou arquivo inteiro vindo do stdin |
| `search` | Busca conteúdo de arquivos em paralelo com o motor do ripgrep |
| `replace` | Troca texto em vários arquivos com escrita atômica e casamento aproximado |
| `transform` | Reescreve código por padrão de AST com o ast-grep |
| `hash` | Imprime o checksum BLAKE3 de arquivos ou do stdin |
| `count` | Conta linhas e arquivos por extensão ou por tamanho |
| `diff` | Compara dois arquivos como unificado, estatística ou mudanças |
| `list` | Lista uma árvore de diretórios com metadados |
| `extract` | Extrai campos de NDJSON do stdin |
| `extract-artifact` | Extrai sinal delimitado de cabeçalhos de PDF, OOXML e imagem |
| `scope` | Age sobre comentários, strings, funções e outras categorias de código |
| `regex` | Gera uma regex a partir de strings de exemplo |
| `calc` | Avalia contas e conversões de unidades |
| `completions` | Imprime ou instala completions de shell |
| `backup` | Cria backups com data e hora e checksums BLAKE3 |
| `rollback` | Restaura um arquivo a partir de um backup anterior |
| `batch` | Roda operações NDJSON do stdin, com opção de transação única |
| `edit-loop` | Aplica N pares antigo/novo numa escrita só, tudo ou nada |
| `prune-backups` | Remove arquivos `.bak` antigos por idade ou quantidade |
| `wal-stats` | Mostra o estado dos journals WAL, só leitura |
| `wal-heal` | Remove journals WAL terminais velhos |
| `verify` | Confere um arquivo contra um hash BLAKE3 esperado |
| `set` | Grava um valor num caminho com pontos em TOML ou JSON |
| `get` | Lê um valor num caminho com pontos em TOML ou JSON |
| `del` | Remove uma chave de um arquivo TOML ou JSON |
| `case` | Renomeia identificadores entre estilos de caixa em arquivos |
| `query` | Percorre a AST do tree-sitter e emite os nós |
| `outline` | Emite a estrutura de um arquivo de código |
| `semantic-merge` | Merge de três vias por linha com marcadores de conflito |
| `sparse` | Lista, leitura e outline com orçamento para árvores grandes |
| `recipe` | Lista e roda receitas embutidas de vários passos |
| `stat` | Só metadados, alias de `read --stat` |
| `agent-surface` | Manifesto de ferramentas para agentes |
| `watch` | Transmite eventos do sistema de arquivos, exige a feature `watch` no build |
| `codemod` | Aplica uma lista YAML de regras de reescrita por AST |
| `semantic-search` | Ranking offline por Jaccard de arquivos para uma consulta |
| `doctor` | Diagnóstico de prontidão do host |
| `scratch` | Cria o diretório de staging dentro do jail |
| `locale` | Mostra, define ou limpa o idioma de exibição |
| `commands` | Imprime a árvore completa de comandos em JSON |
| `config` | Lê e grava chaves de ajuste persistentes |


## Flags Globais
- As flags globais vão antes do subcomando, e `atomwrite commands --include-globals` lista as 33
- `--json` é aceita e ignorada, porque o stdout é sempre NDJSON
- Não existe `-n` global, porque o `replace` é dono do `-n`

| Flag | Padrão | Significado |
|---|---|---|
| `-v`, `--verbose` | desligada | Sobe o tracing no stderr: `-v` info, `-vv` debug, `-vvv` trace |
| `-q`, `--quiet` | desligada | Desce o tracing: `-q` error, `-qq` off |
| `--workspace PATH` | diretório atual | Raiz do jail, e todo caminho deve ficar dentro dela |
| `--config PATH` | nenhum | `.atomwrite.toml` explícito, e falha se ausente ou malformado |
| `--color auto\|always\|never` | `auto` | Controla a saída colorida |
| `--no-color` | desligada | Igual a `--color never` |
| `--ready-file PATH` | nenhum | Harness de teste: grava o PID quando os handlers de sinal estão prontos |
| `--no-gitignore` | desligada | Não respeita o `.gitignore` |
| `--hidden` | desligada | Inclui arquivos e diretórios ocultos |
| `--follow-symlinks` | desligada | Segue links simbólicos |
| `-j`, `--threads N` | automático | Alias `--max-concurrency`, omitido ou 0 usa todos os cores até 16, limitado pela RAM, e um N explícito vale acima de 16 |
| `--max-filesize BYTES` | 1 GiB | Rejeita arquivos maiores com exit 65, enquanto o `search` tem seu próprio limite de 10 MiB que pula |
| `--timeout-secs N` | 120 | Alias `--timeout`, 0 desliga o prazo, e um prazo esgotado sai 124 |
| `--no-progress` | desligada | Desliga eventos NDJSON de progresso em operações longas |
| `--json-schema` | desligada | Emite o JSON Schema da saída do subcomando e sai |
- `--print-schema [NAME]` | desligada | Lista os 55 nomes de schema, ou emite um, e vai antes do comando |
| `--non-interactive` | desligada | Declara execução hospedada por agente: nunca pergunta, nunca lê confirmação do stdin |
| `--confirm-side-effect` | desligada | Reconhece um efeito colateral onde o contrato do comando pede; num verbo que não escreve ela é recusada com exit 65 `INVALID_INPUT` em vez de virar no-op silencioso |
| `--allow-truncate` | desligada | Opt-in para truncar valores, exigida por `--max-output-bytes` |
| `--locale en\|pt-BR` | resolvido | Sobrescreve o idioma de exibição das sugestões |
| `--no-auto-heal` | desligada | Pula o passe de `wal-heal` na partida |
| `--select KEYS` | todas as chaves | Alias `--fields`, projeta os eventos em chaves de topo (CSV) |
| `--filter EXPR` | nenhum | Mantém eventos com `key=value`, `key==value` ou `key!=value`, AND repetível |
| `--limit N` | nenhum | Teto de trabalho: corta a lista de alvos antes da ação e para a varredura no registro que atinge o teto, então saem no máximo N registros de resposta; e o resumo terminal traz `walker_stopped_early: true` só quando a varredura foi mesmo cortada antes do fim da árvore |
| `--sort-by KEY` | ordem de chegada | Ordena eventos em buffer por uma chave, `path-line` ou `line-path` |
| `--dedupe KEYS` | desligada | Mantém o primeiro evento de cada grupo de valores de chave iguais |
| `--aggregate KEY[:OP[:FIELD]]` | desligada | Colapsa eventos por grupo, com OP `count`, `sum`, `min` ou `max` |
| `--join KEY:SUFFIX` | desligada | Lê uma tabela NDJSON do stdin uma vez (até 16 MiB) e anexa as linhas casadas sob SUFFIX |
| `--truncate-content KEY:LEN` | desligada | Corta valores string de KEY em LEN caracteres |
| `--redact` | desligada | Troca valores com cara de segredo por `[redacted]` |
| `--normalize-encoding` | desligada | Normaliza todo valor string para Unicode NFC |
| `--diff-against PATH` | desligada | Troca o stream por uma linha `stream_diff` com contagens de adicionados, removidos e iguais |
| `--max-output-bytes BYTES` | desligada | Limita todo valor string, `checksum` incluído, e exige `--allow-truncate` |

- `--select`, `--filter` e `--limit` mantêm os envelopes de erro, então uma falha com `ok: false` ainda aparece
- Ordenação, dedupe, aggregate e stream diff guardam a execução inteira em buffer e aplicam no fim
- `--max-output-bytes` corta todo valor string, então nunca o use quando o `checksum` ou o `content` inteiro importa
- `--diff-against` exige um arquivo-base dentro do workspace, e `--print-schema NAME` ou `--print-schema=NAME` funcionam
- Uma flag de shape do final nunca esconde o registro `cancelled` de uma execução interrompida

```bash
atomwrite --workspace "$WS" --sort-by path-line --select type,path,line_number --filter type=match search 'fn' src/
atomwrite --workspace "$WS" --aggregate kind:count list --depth 1 src/
atomwrite --workspace "$WS" --max-output-bytes 12 --allow-truncate --select path,checksum hash src/lib.rs
atomwrite --workspace "$WS" --select ok,error,code,match_count,suggestion --filter ok=false --limit 5 edit src/main.rs --old nothere --new x --fuzzy off
atomwrite --print-schema search
atomwrite --workspace "$WS" list --depth 1 src/ > tmp/base.ndjson
atomwrite --workspace "$WS" --diff-against tmp/base.ndjson list --depth 1 src/
printf '%s\n' '{"path":"src/lib.rs","owner":"alice"}' | atomwrite --workspace "$WS" --join path:owners --select type,path,owners list --depth 1 src/
```

- Espere eventos `match` ordenados, depois linhas de grupo como `{"kind":"file","count":11}`, depois strings cortadas, depois um envelope de erro
- A última chamada imprime o JSON Schema do resumo do `search`
- `--aggregate` com `sum` lê um campo numérico, como em `--aggregate kind:sum:size`
- `--diff-against` compara o stream com uma base salva, como mostrado a seguir
- A chamada com `--join` imprime `{"type":"entry","path":"src/lib.rs","owners":[{"path":"src/lib.rs","owner":"alice"}]}` e sai 0
- O `--join` roda uma vez depois da ordenação e antes de dedupe, select, filter e limit
- Um evento sem KEY, ou sem linha casada, fica como estava
- Eventos de falha e o `summary` nunca são alterados por um join
- Um join sai 65 (`INVALID_INPUT`) quando o stdin é um terminal ou a tabela passa de 16 MiB
- Também sai 65 para linha que não é objeto JSON, `KEY:SUFFIX` inválido ou tabela sem KEY string
- Comandos que leem stdin recusam com exit 65: `write`, `edit`, `edit-loop`, `apply`, `extract`, `batch` (sem `--file`), `hash --stdin`, `diff -`
- Use-o com comandos só de leitura, como `search`, `list` e `count`
- `.gitignore` ou `.ignore` no topo de uma raiz que não pode ser lido ou interpretado é reportado em `files_failed` e `errors[]` com o caminho do próprio arquivo, então a varredura nunca esconde arquivo de ignore quebrado


## Exit Codes e Classes de Erro
- Leia o exit code antes de parsear o stdout, porque um envelope de erro também vai para o stdout
- A tabela tem 42 linhas e 34 números de saída distintos, e vem do `atomwrite agent-surface`
- `atomwrite --help` imprime a mesma lista sob `Exit codes`
- O enum de erros tem 38 variantes, contadas com `rg -c '#\[error' src/error/variants.inc.rs`, e o `code` do envelope é uma string livre, não um enum fechado

| Exit | Código | Classe | Repetível |
|---|---|---|---|
| 0 | `SUCCESS` | none | não |
| 1 | `NO_MATCHES` | permanent | não |
| 1 | `DOCTOR_CHECKS_FAILED` | precondition_failed | não |
| 2 | `UNSUPPORTED_COMBINATION` | permanent | não |
| 2 | `ARGUMENT_PARSE_ERROR` | permanent | não |
| 4 | `FILE_NOT_FOUND` | permanent | não |
| 4 | `GLOB_NO_MATCHES` | permanent | não |
| 13 | `PERMISSION_DENIED` | permanent | não |
| 28 | `DISK_FULL` | transient | sim |
| 30 | `QUOTA_EXCEEDED` | transient | sim |
| 65 | `INVALID_INPUT` | permanent | não |
| 65 | `MATCH_FAILED` | permanent | não |
| 65 | `MATCH_AMBIGUOUS` | permanent | não |
| 65 | `BINARY_FILE` | precondition_failed | não |
| 65 | `FILE_TOO_LARGE` | precondition_failed | não |
| 69 | `GRAMMAR_UNAVAILABLE` | precondition_failed | não |
| 73 | `CROSS_DEVICE` | conflict | sim |
| 74 | `IO_ERROR` | transient | sim |
| 78 | `CONFIG_INVALID` | permanent | não |
| 79 | `SYMLINK_BLOCKED` | precondition_failed | não |
| 80 | `TRANSACTION_ROLLBACK_FAILED` | permanent | não |
| 81 | `CHECKSUM_VERIFY_FAILED` | precondition_failed | não |
| 82 | `STATE_DRIFT` | conflict | sim |
| 83 | `LOCK_TIMEOUT` | conflict | sim |
| 84 | `MERGE_CONFLICT` | permanent | não |
| 85 | `FIFO_DETECTED` | precondition_failed | não |
| 86 | `DEVICE_FILE` | precondition_failed | não |
| 87 | `EDIT_LOOP_PARTIAL` | permanent | não |
| 88 | `SYNTAX_ERROR_DETECTED` | precondition_failed | não |
| 89 | `ALL_SKIPPED_BINARY` | permanent | não |
| 91 | `EXDEV_FALLBACK_DISABLED` | precondition_failed | não |
| 92 | `COPY_BACK_BLAKE3_FAILED` | conflict | sim |
| 93 | `ORPHAN_JOURNAL` | precondition_failed | não |
| 124 | `TIMEOUT` | precondition_failed | não |
| 126 | `WORKSPACE_JAIL` | precondition_failed | não |
| 128 | `IMMUTABLE_FILE` | precondition_failed | não |
| 129 | `SIGHUP` | none | não |
| 130 | `CANCELLED` | permanent | não |
| 130 | `SIGINT` | none | não |
| 141 | `BROKEN_PIPE` | permanent | não |
| 143 | `SIGTERM` | none | não |
| 255 | `INTERNAL_ERROR` | permanent | não |

- Exit 0 é sucesso e exit 1 `NO_MATCHES` é uma resposta normal de `search`, `replace`, `transform`, `scope` e `semantic-search`
- Uma execução com falha termina em um único evento terminal com `ok: false`, que traz `suggestion`
- O envelope de erro padrão traz `ok`, `code`, `exit`, `path` quando existe, `suggestion`, `target_resolved` e `target_source`
- Passe `-v` para trazer de volta a prosa de diagnóstico: `message`, `error_class`, `retryable` e `suggestion_lang`
- O mesmo `-v` sobe o log do stderr para info, e o stdout continua com uma única linha terminal
- `MATCH_FAILED` acrescenta `match_count` e `best_candidate`, e `MATCH_AMBIGUOUS` acrescenta `best_candidate` e `match_count`
- As classes são `none`, `permanent`, `precondition_failed`, `transient` e `conflict`
- Repita só quando `retryable` for true, faça backoff em `transient` e releia o arquivo em `conflict`
- Exit 82 significa recarregar o checksum, e exit 124 significa aumentar `--timeout-secs` ou estreitar o caminho
- Exit 126 é o jail e exit 143 é um SIGTERM limpo
- Exit 129 é SIGHUP, e exit 130 é SIGINT ou uma execução cancelada
- Exit 141 significa que o leitor fechou o pipe, a saída está incompleta e o estado precisa ser conferido
- Exit 78 cobre um arquivo de config ruim, uma chave de config desconhecida e um verbo ausente do build
- Todo verbo que varre árvore lista arquivos ilegíveis em `files_failed`, `partial` e `errors[]`, então leia-os, e `errors[].path` é o marcador `<walk>` quando a varredura falhou sem nomear um arquivo próprio
- Um write que falha porque o leitor fechou o pipe sai 141 (`BROKEN_PIPE`), inclusive `completions`, `--print-schema`, `--help` e todo verbo

```json
{"ok":false,"ok":false,"code":"MATCH_FAILED","exit":65,"target_resolved":["src/a.rs"],"target_source":"argv","match_count":0,"suggestion":"inspect best_candidate in the error NDJSON and adjust --old, or lower --fuzzy-threshold"}
```

- O SIGPIPE tem um handler cooperativo que encerra a execução com exit 141 depois da limpeza
- A execução nunca deixa lock, journal ou tempfile para trás
- Isto é um desvio consciente da regra literal de voltar o SIGPIPE à ação padrão


## Configuração
- A precedência é flags primeiro, depois o `.atomwrite.toml` do workspace, depois o `config.toml` global, depois os defaults embutidos
- Os dois arquivos são mesclados chave a chave, então um arquivo de workspace esconde só as chaves que ele nomeia
- `--config PATH` substitui os dois arquivos, e o `config get` passa a relatar `origin` `explicit`
- O atomwrite não tem variáveis de ambiente de produto, então um valor vem de uma flag ou destes arquivos
- Mude uma chave com `atomwrite config set KEY VALUE --scope global|workspace`, e evite editar os arquivos à mão
- Um `set` mantém comentários, valida o arquivo inteiro e grava de forma atômica, com modo 0600 no global (Unix)
- O `config` continua funcionando quando o `config.toml` está quebrado, e um arquivo que não é TOML válido nunca é reescrito
- O evento terminal de `set` e `unset` traz `target_resolved`, então confira se o caminho é o arquivo que você queria

### As 43 Chaves
- Rode `atomwrite config list` para a tabela viva com tipo, padrão, valor, origem e escopos
- `log.file`, `log.rotation`, `storage.home` e `locale` são só globais, e o escopo workspace sai 65
- Uma chave desconhecida ou um valor fora da faixa sai 65 e não grava nada

| Chave | Tipo | Padrão | Valores aceitos | Escopos |
|---|---|---|---|---|
| `defaults.line_ending` | string | `auto` | `auto`, `lf`, `crlf` | global, workspace |
| `defaults.backup` | boolean | none (true para reescritas, false para delete) | `true`, `false` | global, workspace |
| `delete.backup` | boolean | nenhum (cai para `defaults.backup`) | `true`, `false` | global, workspace |
| `defaults.retention` | integer | 5 | 0 a 255 | global, workspace |
| `defaults.max_filesize` | integer | 1073741824 | 1 ou mais | global, workspace |
| `fuzzy.mode` | string | `auto` | `auto`, `aggressive`, `off` | global, workspace |
| `fuzzy.threshold` | float | 0.7 | 0 a 1 | global, workspace |
| `fuzzy.threshold_aggressive` | float | 0.5 | 0 a 1 | global, workspace |
| `fuzzy.threshold_context` | float | 0.8 | 0 a 1 | global, workspace |
| `fuzzy.threshold_jw` | float | 0.85 | 0 a 1 | global, workspace |
| `fuzzy.soft_floor_delta` | float | 0.05 | 0 a 1 | global, workspace |
| `fuzzy.max_content_bytes` | integer | 268435456 | 1 ou mais | global, workspace |
| `apply.max_ellipsis_lines` | integer | 200 | 1 ou mais | global, workspace |
| `write.confirm_large_bytes` | integer | 102400 | 1 ou mais | global, workspace |
| `write.shrink_block_percent` | integer | 50 | 1 a 99 | global, workspace |
| `write.auto_rotate_max_age_secs` | integer | 86400 | 1 ou mais | global, workspace |
| `write.content_risk_patterns` | lista de strings | a lista embutida de comandos de risco | qualquer string | global, workspace |
| `watch.idle_exit_ms` | integer | 500 | 0 ou mais (0 desliga) | global, workspace |
| `watch.debounce_ms` | integer | 200 | 1 ou mais | global, workspace |
| `log.file` | boolean | false | `true`, `false` | global |
| `log.rotation` | string | `daily` | `daily`, `hourly` | global |
| `storage.home` | string | nenhum | um diretório | global |
| `locale` | string | nenhum | `en`, `pt-BR` | global |
| `concurrency.max_threads` | integer | 16 | 1 a 256 | global, workspace |
| `concurrency.ram_fraction_percent` | integer | 50 | 1 a 100 | global, workspace |
| `search.context` | integer | 0 | 0 ou mais | global, workspace |
| `search.smart_case` | boolean | true | `true`, `false` | global, workspace |
| `search.max_filesize` | integer | 10485760 | 1 ou mais | global, workspace |
| `search.max_columns` | integer | 500 | 1 ou mais | global, workspace |
| `search.max_inflight_bytes` | integer | 8388608 | 4 MiB a 1 GiB | global, workspace |
| `search.format` | string | `ndjson` | `ndjson`, `text` | global, workspace |
| `read.max_output_bytes` | integer | 24000 | 1 ou mais | global, workspace |
| `read.number` | boolean | false | `true`, `false` | global, workspace |
| `output.profile` | string | `full` | `full`, `lean` | global, workspace |
| `summary.errors_max_entries` | integer | 50 | 1 a 1000 | global, workspace |
| `lock.acquire_timeout_ms` | integer | 5000 | 100 a 3600000 | global, workspace |
| `retry.base_ms` | integer | 50 | 1 a 60000, não acima de `retry.max_ms` | global, workspace |
| `retry.max_ms` | integer | 400 | 1 a 60000 | global, workspace |
| `retry.max_elapsed_ms` | integer | 2000 | 1 a 3600000 | global, workspace |
| `extract.max_sample_rows` | integer | 10 | 0 a 100000 | global, workspace |
| `extract.max_text_chars` | integer | 400 | 1 a 10000000 | global, workspace |
| `extract.max_sheets` | integer | 32 | 1 a 10000 | global, workspace |
| `extract.max_slides` | integer | 64 | 1 a 10000 | global, workspace |

- Uma flag ainda vence uma chave, como `extract-artifact --max-text-chars` vence `extract.max_text_chars`
- `fuzzy.max_content_bytes` é o teto acima do qual a cascata fuzzy não roda, e matches exatos ainda funcionam
- `write.confirm_large_bytes` é o tamanho acima do qual uma sobrescrita exige `--ack-overwrite`
- `search.format` decide se o `search` grava NDJSON ou texto quando `--format` está ausente, e a flag sempre vence a chave
- `output.profile = "lean"` remove os digests, as contagens de bytes e o tempo de um registro de mutação bem-sucedida
- `retry.*` molda a repetição com jitter das esperas de lock e de rename, e `--timeout-secs` ainda a limita
- `storage.home` realoca config e estado para `{home}/config` e `{home}/state`

```bash
atomwrite --workspace "$WS" config set fuzzy.threshold 0.9 --scope workspace
atomwrite --workspace "$WS" config get fuzzy.threshold
atomwrite --workspace "$WS" config set log.file true --scope global
```

- A primeira chamada grava o `.atomwrite.toml` no workspace, e a segunda relata `origin` `workspace`
- A terceira chamada liga o arquivo de diagnóstico, o que só o escopo global pode fazer

### Formato do Arquivo de Config
- Um arquivo encontrado precisa ser legível, TOML válido e sem chave desconhecida
- Caso contrário a execução sai 78 com `CONFIG_INVALID`
- A mensagem nomeia o arquivo, e um `version` maior que o valor suportado 1 também sai 78
- O carregador aceita `version` e 11 seções: `[defaults]`, `[write]`, `[watch]`, `[fuzzy]`, `[search]`, `[storage]`, `[log]`, `[concurrency]`, `[lock]`, `[retry]` e `[extract]`
- Alguns campos não têm chave de `config`, então você os define editando o arquivo

| Campo | Significado |
|---|---|
| `defaults.backup`, `defaults.retention` | Chave padrão de backup e retenção dos comandos que gravam; a chave ausente mantém o padrão de cada verbo |
| `write.shrink_block_percent` | Limite da proteção contra encolhimento, padrão 50 |
| `write.auto_rotate_max_age_secs` | Janela de idade do `--auto-rotate`, padrão 24 horas |
| `write.content_risk_patterns` | Padrões extras da avaliação de risco do write |
| `watch.idle_exit_ms`, `watch.debounce_ms` | Padrões do `watch` |
| `search.context`, `search.smart_case`, `search.max_filesize`, `search.max_columns` | Padrões do `search` |
| `defaults.line_ending`, `defaults.max_filesize` | Lidos pelo carregador mas não usados por nenhum comando |

```toml
[defaults]
backup = true
retention = 5

[fuzzy]
mode = "auto"
threshold = 0.85

[search]
max_filesize = 10485760
```

- Este é um `.atomwrite.toml` válido para a raiz do workspace, e um campo desconhecido como `defaults.workspace` sairia 78
- O `[log]` num arquivo de workspace é ignorado, porque o tracing inicia antes de ler o arquivo do workspace

### Onde Ficam os Arquivos
- Os caminhos do Linux foram medidos, e os de macOS e Windows vêm do código (`directories::ProjectDirs`) e não foram validados

| Item | Caminho no Linux |
|---|---|
| Config global | `$XDG_CONFIG_HOME/atomwrite/config.toml` (padrão `~/.config/atomwrite/config.toml`) |
| Config do workspace | `.atomwrite.toml` na raiz do workspace |
| Preferência de locale | `$XDG_CONFIG_HOME/atomwrite/locale` |
| Log de diagnóstico | `$XDG_STATE_HOME/atomwrite/logs/atomwrite.<date>.jsonl`, arquivo com modo 0600 em pasta 0700 |
| Parsers tree-sitter | `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` |
| Completions do Bash | `$XDG_DATA_HOME/bash-completion/completions/atomwrite` |
| Completions do Zsh | `$XDG_DATA_HOME/zsh/site-functions/_atomwrite` |
| Completions do Fish | `$XDG_DATA_HOME/fish/vendor_completions.d/atomwrite.fish` |
| Elvish e PowerShell | `$XDG_DATA_HOME/elvish/lib/atomwrite.elv` e `$XDG_DATA_HOME/powershell/Completions/_atomwrite.ps1` |

- O log de diagnóstico vem desligado, então nenhum arquivo é aberto e nenhuma thread de escrita sobe
- Quando ligado, ele rotaciona por dia ou por hora, mantém 7 arquivos e grava JSON lines
- Os diagnósticos no stderr continuam em JSON lines nos dois casos


## Padrões Avançados

### Locking Otimista Com Checksum
- Capture o checksum e depois grave só se o arquivo não mudou

```bash
CS="$(atomwrite --workspace "$WS" read src/lib.rs | jaq -r .checksum)"
printf 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\npub fn sub(a: i32, b: i32) -> i32 {\n    a - b\n}\n' | atomwrite --workspace "$WS" write --expect-checksum "$CS" src/lib.rs
```

- Espere `type` `write` com `checksum_before` igual a `$CS`
- Um arquivo alterado sai 82 (`STATE_DRIFT`), e você recarrega o checksum e decide de novo

### Edições Sequenciais em Um Arquivo
- Recapture o checksum antes de cada edição protegida, ou aceite o drift de propósito

```bash
CS="$(atomwrite --workspace "$WS" hash src/b.rs | jaq -r .checksum)"
atomwrite --workspace "$WS" edit --expect-checksum "$CS" src/b.rs --old 'beta' --new 'beta_one'
atomwrite --workspace "$WS" edit --expect-checksum "$CS" --allow-sequential-drift src/b.rs --old 'beta_one' --new 'beta_two'
```

- A primeira edição passa, e a segunda sairia 82 sem `--allow-sequential-drift`
- O drift aceito dessa forma é registrado como aviso no stderr

### Staging em Scratch Para Patches de Várias Linhas
- Texto com aspas, `$` ou barras é frágil no argv, então estagie-o em arquivos dentro do jail

```bash
atomwrite --workspace "$WS" scratch init .atomwrite/scratch
SCR=".atomwrite/scratch"
OLD=$'fn load() -> Option<i32> {\n    Some(1)\n}'
NEW=$'fn load() -> Option<i32> {\n    Some(2)\n}'
printf %s "$OLD" > "$SCR/old.txt"
printf %s "$NEW" > "$SCR/new.txt"
atomwrite --workspace "$WS" edit src/main.rs --old-file "$SCR/old.txt" --new-file "$SCR/new.txt"
```

- Espere `type` `edit` com `edits` 1
- Um arquivo de conteúdo sob o diretório temporário do SO sai 126 (`WORKSPACE_JAIL`)

### Achar, Prever, Trocar, Confirmar
- Localize primeiro, veja a prévia da mudança, aplique e depois confirme que nada sobrou

```bash
atomwrite --workspace "$WS" search 'beta' src/
atomwrite --workspace "$WS" replace --dry-run beta_two beta src/
atomwrite --workspace "$WS" replace beta_two beta src/b.rs
atomwrite --workspace "$WS" search 'beta_two' src/b.rs
```

- A última busca sai 1, o que significa que nenhum match sobrou no arquivo
- Um `replace` bem-sucedido apaga a cópia `.bak` que fez, e o `search` ainda varre arquivos `.bak`, então nomeie o arquivo

### Mudança Atômica em Vários Arquivos
- Um batch com `--transaction` confirma todas as operações ou nenhuma

```bash
printf '%s\n' '{"op":"write","target":"b1.txt","content":"x"}' '{"op":"copy","source":"b1.txt","target":"b2.txt"}' '{"op":"move","source":"b2.txt","target":"b3.txt"}' '{"op":"delete","target":"b3.txt"}' > tmp/manifest.ndjson
atomwrite --workspace "$WS" batch --file tmp/manifest.ndjson --transaction
```

- Espere quatro linhas `batch_op` e um `summary` com `committed` igual a `true`

### Desfazer uma Edição Ruim
- Faça um backup antes de uma mudança arriscada e restaure-o se o resultado estiver errado

```bash
atomwrite --workspace "$WS" backup src/lib.rs
printf 'broken\n' | atomwrite --workspace "$WS" write --allow-shrink src/lib.rs
atomwrite --workspace "$WS" rollback src/lib.rs --latest --verify
```

- Espere `type` `rollback` com `verified` igual a `true`
- O arquivo volta ao conteúdo que tinha antes do write ruim

### Saída Compacta Para Agentes
- Peça só as chaves de que precisa e filtre pelo resultado

```bash
atomwrite --workspace "$WS" --select ok,code,match_count,suggestion --filter ok=false --limit 5 edit src/main.rs --old nothere --new x --fuzzy off
```

- Espere uma linha como `{"ok":false,"code":"MATCH_FAILED","match_count":0,"suggestion":"..."}` e exit 65
- O `--filter` nunca esconde um envelope de erro, então uma falha com `ok: false` sempre chega


## Integração Com Shell Scripts
- Confira o exit code primeiro, e trate o exit 1 de comandos de busca como uma resposta
- Use `jaq` para os campos, e mantenha `${PIPESTATUS[0]}` quando um pipe segue o `atomwrite`

```bash
if atomwrite --workspace "$WS" search 'TODO' src/ > tmp/hits.ndjson; then
  echo "found"
else
  code=$?
  if [ "$code" -eq 1 ]; then echo "none"; else echo "failed: $code"; fi
fi
```

- Espere `found` enquanto existir um `TODO`, e `none` quando ele sumir
- Um write cujo pipe alimenta o `jaq` esconde o status dele a menos que você leia `${PIPESTATUS[0]}`

```bash
out="$(atomwrite --workspace "$WS" edit src/main.rs --old nothere --new x --fuzzy off)"; code=$?
printf '%s\n' "$out" | jaq -r .ok
echo "$code"
```

- Espere `false` e `65`
- Passe um `--workspace` absoluto a processos destacados, porque eles não herdam um diretório de trabalho útil


## Integração Com Agentes de IA
- Todo subcomando escreve NDJSON determinístico no stdout, e toda escrita devolve um `checksum` BLAKE3
- Rode `atomwrite agent-surface --format json` uma vez para aprender ferramentas, exit codes e nomes de schema
- Passe sempre `--workspace` como caminho absoluto, e use `--select`, `--filter` e `--limit` para poupar tokens
- Leia o exit code antes dos campos, e leia `best_candidate` depois de um exit 65
- Rode `--dry-run`, `--plan` ou `--preview` antes de qualquer mudança em massa
- Use `--expect-checksum` em edições concorrentes, e `--ack-overwrite` só em sobrescritas grandes intencionais
- Nunca passe `delete --confirm` nem `delete --yes`, e nunca rode `watch` sem `--max-events` ou um timeout
- Veja [AGENTS.pt-BR.md](AGENTS.pt-BR.md) para o contrato completo do agente e o tratamento dos exit codes


## Integração Com Crates Rust
- O atomwrite é uma ferramenta de linha de comando, e o jeito suportado de chamá-lo do Rust é um subprocesso
- Confira primeiro o status de saída e depois parseie as linhas NDJSON do stdout

```rust
use std::process::Command;

fn main() -> std::io::Result<()> {
    let ws = std::env::current_dir()?;
    let out = Command::new("atomwrite")
        .arg("--workspace")
        .arg(&ws)
        .args(["hash", "src/lib.rs"])
        .output()?;
    match out.status.code() {
        Some(0) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let first = stdout.lines().next().unwrap_or("");
            println!("ok: {first}");
        }
        Some(code) => eprintln!("atomwrite exited with {code}: {}", String::from_utf8_lossy(&out.stdout)),
        None => eprintln!("atomwrite was killed by a signal"),
    }
    Ok(())
}
```

- Espere `ok:` seguido do evento `hash` quando rodar no workspace de exemplo
- Parseie cada linha com `serde_json` num projeto real, e valide-a contra `atomwrite --print-schema NAME`


## Erros Comuns
- O primeiro bullet de cada entrada é o sintoma, e os seguintes são causa e correção

### Exit 65 MATCH_FAILED
- Um par de `edit` ou `edit-loop` não achou match
- Leia `best_candidate`, alargue o `--old`, ou use `--fuzzy auto` ou `--fuzzy aggressive`

### Exit 65 em um Write Grande ou Que Encolhe
- O alvo está acima de 100 KiB, ou o conteúdo novo é menos da metade do tamanho antigo
- Passe `--ack-overwrite` no primeiro caso e `--allow-shrink` no segundo, só quando for intencional

### Exit 82 STATE_DRIFT
- O arquivo mudou desde que você leu o checksum dele
- Recarregue o checksum e decida de novo, ou use `--allow-sequential-drift` num pipeline controlado

### Exit 126 WORKSPACE_JAIL
- Um caminho aponta para fora de `--workspace`
- Use um caminho dentro do workspace, e estagie arquivos de conteúdo sob `.atomwrite/scratch`

```json
{"ok":false,"ok":false,"code":"WORKSPACE_JAIL","exit":126,"suggestion":"use a path inside the workspace (/home/user/project); for --old-file/--new-file stage under /home/user/project/.atomwrite/scratch/ (never OS /tmp outside the jail)","path":"/etc/passwd","workspace":"/home/user/project"}
```

### Exit 4 FILE_NOT_FOUND
- O caminho não existe, e o envelope lista `similar_paths`
- Confira a grafia, ou use `list` e `search --target files` para achar o arquivo

### Exit 2 ARGUMENT_PARSE_ERROR
- Uma flag não existe ou falta um argumento obrigatório, e a `message` traz o uso do clap
- Rode `atomwrite COMMAND --help` e use só as flags listadas ali

### Exit 1 NO_MATCHES
- Um comando de busca não achou nada, o que é uma resposta e não um crash
- Amplie o padrão, ou revise `--include`, `--exclude` e `--max-filesize`

### Exit 78 CONFIG_INVALID
- Um arquivo de config está malformado, tem uma chave desconhecida, ou o verbo está ausente do build, como o `watch`
- Corrija o arquivo nomeado na `message`, ou reinstale com `--features watch`

### Exit 88 SYNTAX_ERROR_DETECTED
- `--syntax-check` ou `--verify-parse` achou um erro de parse no resultado
- Corrija o conteúdo, e espere que a primeira chamada por linguagem baixe o parser

### Zero Silencioso do case em um Diretório
- O `case` em um diretório sai 0 com `identifiers_total` 0 e não muda nada
- Passe os arquivos no lugar

### O Dry Run do prune-backups
- `--dry-run` não recebe valor, e `--dry-run false` lê `false` como um caminho
- Omita `--dry-run` para aplicar, e nomeie os arquivos-alvo cujos backups você quer podar

### Pipe Quebrado
- `read` e `search` saem 141 quando o leitor fecha o pipe, e a saída fica incompleta
- Isso vale para `completions`, `--print-schema`, `--help` e todo verbo, sem panic e sem relatório de falha


## Tempo de Modificação e Sistemas de Build
- `edit` e `replace` atualizam o mtime do arquivo para o fim da escrita, o que as ferramentas de build precisam
- Cargo, make, cmake, gradle, bazel e ninja comparam mtimes para decidir o que recompilar
- Um mtime antigo preservado pode fazer o cargo pular uma recompilação e relatar um binário velho como pronto
- Use `--preserve-timestamps` só para snapshots, artefatos reproduzíveis ou arquivos fora de qualquer build
- O campo `mtime_preserved` na resposta diz qual dos casos aconteceu


## Instalação no Windows
- Instale o Visual Studio Build Tools com a carga de trabalho de desktop C++ e o Rust 1.90 ou superior
- Use o Windows Terminal ou o PowerShell 7 para saída UTF-8 e sequências ANSI
- O lock de escrita advisory é só Unix, e o Windows ainda não tem lock de escrita
- Veja [INSTALL.pt-BR.md](INSTALL.pt-BR.md) para o guia completo do Windows e a solução de problemas


## Histórico de Versões
- Esta seção é histórica, e os detalhes vivem no [CHANGELOG](../CHANGELOG.pt-BR.md) e no [MIGRATION](MIGRATION.pt-BR.md)

### v0.1.36 (2026-07-31)
- Histórico: 46 subcomandos, e staging do `scratch init` sob `.atomwrite/scratch/`
- Falhas emitem `ok: false` e um `match_count` estável, então confira o exit antes dos campos de sucesso
- As globais `--select`, `--filter` e `--limit` moldam o stdout em todo subcomando
- Estratégias fuzzy normalizadas relatam `similarity: null`, e o `block_anchor` usa um portão duplo

### v0.1.35 (2026-07-19)
- Sobrescritas grandes viraram default-deny, e `--ack-overwrite` é o reconhecimento one-shot
- `delete --plan` virou o jeito de listar alvos, e `delete --confirm` e `--yes` passaram a ser rejeitados
- O `watch` sempre emite um `watch_summary` final, com saída por ociosidade padrão de 500 ms
- O `semantic-merge` grava marcadores de conflito por padrão

### v0.1.34 e v0.1.30
- O timeout global padrão virou 120 segundos, e o exit 124 marca um prazo esgotado
- O multi-apply fuzzy virou one-pass da esquerda para a direita, com padrão de um apply
- Linhas de sucesso ganharam `match_count` e `indent_adjusted`, e os misses ganharam `best_candidate`
- Histórico: 41 subcomandos naquele momento

### v0.1.12 a v0.1.29
- A v0.1.12 acrescentou `set`, `get`, `del`, `case`, `query` e `outline`
- A v0.1.18 acrescentou `wal-stats` e `wal-heal`, a v0.1.22 acrescentou `edit-loop` e `prune-backups`, a v0.1.25 acrescentou `verify`
- A v0.1.23 fez do backup o padrão, e a v0.1.28 unificou as flags de backup nos comandos que gravam
- A v0.1.29 acrescentou `recipe`, `sparse`, `semantic-merge`, `agent-surface`, `watch`, `codemod`, `semantic-search` e `stat`


## Próximos Passos
- Navegue por receitas mais longas no [COOKBOOK](COOKBOOK.pt-BR.md)
- Leia o contrato do agente em [AGENTS.pt-BR.md](AGENTS.pt-BR.md)
- Confira as notas de plataforma em [CROSS_PLATFORM.pt-BR.md](CROSS_PLATFORM.pt-BR.md)
- Atualize com [MIGRATION.pt-BR.md](MIGRATION.pt-BR.md), e instale com [INSTALL.pt-BR.md](INSTALL.pt-BR.md)
- Valide a saída com os schemas em [docs/schemas](schemas/README.md)
