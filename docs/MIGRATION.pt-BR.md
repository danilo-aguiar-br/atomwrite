# Guia de Migração do atomwrite


[Read in English](MIGRATION.md)


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


## Versão Atual
- atomwrite está na v0.1.38
- Este documento cobre migração de v0.1.0 a v0.1.38
- Veja as seções abaixo para mudanças aditivas e breaking changes em cada versão


## Trabalho dobrado na v0.1.38: o envelope de falha enxuto
### O Que Muda
- O envelope de falha padrão carrega `ok`, `code`, `exit`, `path` quando existe, `suggestion`, `target_resolved` e `target_source`, e não carrega mais `error`, `message`, `error_class`, `retryable` nem `suggestion_lang`
- Passe `-v` para receber as quatro chaves de volta, tanto no envelope de erro quanto no resumo que falha, e o resumo que falha mantém `code` e `exit` e nada mais do detalhe de falha
- Os schemas publicados de erro e de terminal exigem `ok`, `code` e `exit` numa falha e mantêm as quatro chaves de diagnóstico opcionais, então um validador regenerado dos arquivos novos aceita as duas formas
- Quem ramifica por `retryable` deve passar `-v` ou mapear o código pela tabela de retry, e o exit code continua o sinal primário
- Pin: `cargo install atomwrite --locked --version "^0.1.38"`


## Trabalho dobrado na v0.1.38: o registro enxuto, a imagem recusada e a seção Markdown
### O Que Muda
- O `output.profile = "lean"` tira os digests, as contagens de bytes e o tempo de um registro de mutação bem-sucedida, o que levou um `apply` de três linhas de 433 bytes para 196, e o `full` continua o padrão
- Um arquivo cuja assinatura de imagem está corrompida falha no `read` com exit 65 `INVALID_INPUT` em vez de devolver os bytes como texto, então quem tratava todo arquivo legível como texto precisa tratar a recusa
- O `outline` lê um título Markdown como uma `section` que vai até o fim da própria seção, e o `read --symbol NOME` devolve essa seção inteira com as subseções
- Pin: `cargo install atomwrite --locked --version "^0.1.38"`


## Trabalho dobrado na v0.1.38: o delete recursivo é dono da árvore
### O Que Muda
- O `delete --recursive <dir>` sem `--include`, `--exclude`, `--older-than` ou `--limit` passa a remover a árvore nomeada inteira, incluindo entrada oculta, ignorada, link, fifo, socket e dispositivo, então exit 0 significa que o diretório sumiu
- O chamador que recebia exit 65 de um delete filtrado na versão anterior agora recebe exit 0 com `dirs_left` e `entries_left`, porque a sobra é o que o filtro pediu para manter
- O chamador que recebia exit 65 de um delete sem filtro agora recebe o código de I/O medido com o caminho sobrevivente em `errors[]`, ou exit 0 quando nada sobrevive
- O chamador que lia o `hint` para saber que `--hidden` resolveria deve parar de mandar a flag, que é no-op no `delete` e emite uma linha de diagnóstico
- O chamador que contava com `--follow-symlinks` para apagar o alvo de um link deve nomear o alvo na linha de invocação, porque o link é sempre removido como entrada e nunca seguido
- O chamador que passa `--backup` com raiz de diretório agora recebe exit 65 antes de qualquer mudança, porque a cópia nasceria dentro da árvore e sobreviveria à remoção, então rode `copy -r` antes ou passe `--no-backup`
- O chamador que trata `checksum_before` como obrigatório deve aceitar a ausência no evento de fifo, socket ou dispositivo, porque não há conteúdo para somar
- O chamador que lê `dirs_left`, `entries_left` ou `hint` de `--plan` ou `--dry-run` deve parar de exigi-los, porque um plano não remove nada e não publica sobra
- O `copy -r` agora inclui o caminho listado em `.ignore`, no gitignore global ou no `info/exclude`, que o contrato da cópia de árvore já prometia
- Pin: `cargo install atomwrite --locked --version "^0.1.38"`


## Trabalho dobrado na v0.1.38: o desfazer do batch e o alvo em array
### O Que Muda
- `target_resolved` é sempre array, inclusive na execução que resolveu um alvo só, então o campo deixa de trocar de tipo conforme a contagem
- Leia o sujeito com `jaq -r '.target_resolved[0]'` ou `jq -r '.target_resolved[0]'`; um programa lê `event["target_resolved"][0]`
- O schema publicado declara `target_resolved` como `array` de strings, então um validador gerado pela união anterior de string e array precisa ser regerado
- O `batch` publica `warnings` no registro da operação, então o `--keep-backup` redundante num `delete` responde ali exatamente como responde no verbo isolado
- O `batch` publica `backup_path` no registro da operação quando ela fez cópia, e o `move` que recusa cópia de uma cópia nomeia a recusa em `backup_skipped`
- O resumo de um `batch --transaction` publica `backups_preserved`, nomeando o snapshot que o commit guardou de um arquivo excluído ou sobrescrito
- Um `move` ou um `copy` dentro de `--transaction` guarda o snapshot do destino que sobrescreveu, porque aquele conteúdo não vive em outro lugar; a origem do `move` fica fora, porque o conteúdo dela vive no destino
- `write`, `replace`, `edit` e `copy` dentro de `--transaction` reutilizam o snapshot que a execução já tirou, então `--keep-backup` deixa uma cópia em vez de duas, e `backup_path` nomeia o caminho que sobrevive em `backups_preserved`
- O `delete` cria cópia só quando a chamada pede, por `--backup` ou por `[defaults].backup = true`; `--keep-backup` só preserva uma que existe, o aviso nomeia `--backup` como a flag que cria a cópia, e o `delete` do `batch` segue a mesma regra
- `[defaults].backup` alcança o `delete` e as operações do `batch` quando está escrita, porque a chave agora distingue ausente de `false`; a chave ausente mantém a reescrita copiando e o delete não copiando, e `retention` continua valendo para os 15
- Um `apply` que falha publica `backups_preserved` com as cópias que o rollback deixou no disco
- Fixe: `cargo install atomwrite --locked --version "^0.1.38"`


## Trabalho dobrado na v0.1.38: `--hidden` e o veredito de escrita
### O Que Muda
- `hash`, `copy -r` e `semantic-search` agora escondem dotfile no padrão, do mesmo jeito que `count`, `list`, `search` e a família `sparse` já faziam; a flag que devolve o inventário anterior é `--hidden`
- Recupere a caminhada antiga com `atomwrite --workspace "$WS" hash -r . --hidden`, `atomwrite --workspace "$WS" copy -r . dst --hidden` ou `atomwrite --workspace "$WS" semantic-search "CONSULTA" . --hidden`
- Um caminho nomeado na invocação continua sendo lido sem a flag, então `atomwrite --workspace "$WS" hash .env` funciona como antes e a mudança só alcança a caminhada que nunca nomeou o dotfile
- `sparse list` e `sparse outline` aceitam `--all`, que soma as entradas ocultas à visita dentro do próprio verbo; `--all` e `--hidden` pedem a mesma visita, e o padrão continua escondendo
- `copy`, `move`, `delete`, `rollback`, `semantic-merge`, `config set`, `config unset` e cada operação do `batch` publicam `write_strategy` e `backup_skipped`, então o consumidor lê o mecanismo no evento em vez de deduzi-lo da chamada de sistema
- `edit`, `replace`, `transform`, `scope`, `case`, `set`, `del`, `edit-loop` e `locale` publicam os mesmos dois nomes, a prévia publica `"none"`, e o `del` de chave ausente omite `write_strategy`
- O `batch delete` recusa copiar uma cópia, então um alvo nomeado como `x.bak.<carimbo>` é removido sem segunda cópia e o evento carrega `backup_skipped`
- O `apply` publica `backup_path` quando a escrita fez cópia, e omite a chave quando não fez
- Um `apply` que falha depois de gravar um arquivo restaura o arquivo e publica um registro `rollback` com `ok: false`, e a restauração que falha sai `TRANSACTION_ROLLBACK_FAILED`
- O rollback do `apply` e do `batch --transaction` compara conteúdo em vez de inode, então o arquivo que a própria execução regravou volta; a versão anterior recusava toda restauração como deriva de estado e deixava a árvore parcialmente revertida
- Uma transação bem-sucedida do `batch --transaction` remove o snapshot tirado antes da mutação em vez de deixar `x.bak.<carimbo>`, e `--keep-backup` é a única flag que o mantém; `--no-backup` vence
- O registro `rollback` do `apply` e do `batch --transaction` nomeia todo caminho revertido em `target_resolved`, sempre como array mesmo quando um arquivo só voltou, e `target_source` diz se o manifesto veio de `argv` ou de `stdin`
- Uma transação do `batch` que exclui um arquivo mantém a cópia feita antes da mutação depois do commit, porque o arquivo excluído não vive em nenhum outro lugar, e `--no-backup` é a única forma de proibir essa cópia
- Uma reversão que não consegue restaurar um arquivo publica um registro `rollback` com `failed_paths` e `backups_preserved`, e os snapshots ficam no disco para quem precisa recuperar
- O `apply` relata `target_source: "stdin"` quando o patch traz a própria linha de caminho, e `argv` quando `--file` nomeia o alvo na linha de invocação
- `backup_path`, `backup_skipped` e `write_strategy` são strings puras em todo schema publicado, então um validador gerado pelos arquivos anteriores continua aceitando os eventos
- Um consumidor que falha com campo desconhecido precisa permitir os dois nomes; `backup_skipped` fica ausente quando nenhuma cópia foi recusada, então o evento não carrega `null`
- Fixe: `cargo install atomwrite --locked --version "^0.1.38"`
- Superfície: 47 subcomandos, sem mudança desde a v0.1.37, e a fonte única continua sendo `atomwrite agent-surface`


## Trabalho dobrado na v0.1.38: leituras mais baratas e saídas honestas (Atual)
### O Que Muda
- `read --format raw` agora para em `READ_DEFAULT_MAX_BYTES` (24.000 bytes) e imprime uma linha no stderr com o próximo `--lines`, então quem contava com o arquivo inteiro precisa passar `--all` ou seguir a dica
- `read --lines` é repetível e aceita vários alvos numa chamada, o que a recusa anterior transformava em exit 65; as faixas unidas imprimem na ordem do argv com cabeçalhos `==> caminho <==`
- O `read` ganhou `--number`, `--no-comments`, `--symbol NOME` e `--line N --enclosing`, e `--range` virou apelido de `--lines`
- Uma janela de `--lines` que começa depois da última linha é erro com o total, onde a versão anterior respondia sucesso vazio
- `search --format text` tem teto e imprime um `caminho:linha:coluna:texto` por achado, e o `search` sai `89 ALL_SKIPPED_BINARY` quando todo candidato era binário, onde a versão anterior respondia `NO_MATCHES`
- `list --format text` e `outline --format tsv` são novos, e o `outline` publica `parse_errors` e `first_error_line`
- O `apply` casa linha inteira, então SEARCH que é pedaço de linha não casa mais; SEARCH repetido sai 65 a menos que `--all` seja passado
- O `apply` roda a checagem de sintaxe antes e depois da troca e recusa quando a contagem de erros de parse sobe; `--no-syntax-check` restaura o aceite incondicional antigo
- `apply --show` devolve as linhas alteradas, o `diff` publica `old_line` e `new_line` a partir de 1, e o `delete` publica `backups_kept`, `backup_bytes` e `dirs_left` quando existe backup ou sobra
- Pin: `cargo install atomwrite --locked --version "^0.1.38"`
- Superfície: 47 subcomandos, sem mudança desde a v0.1.37, e a fonte única da verdade continua sendo `atomwrite agent-surface`


## Trabalho dobrado na v0.1.37: read ampliado e teto de busca
### O Que Muda
- O `read` expande padrão cujo prefixo de diretório tem vários níveis, e só curinga dentro do nome de um diretório sai 65, porque a checagem antiga contava todos os componentes do caminho em vez de perguntar qual deles é padrão
- Operando com `*`, `?` ou `[` que existe no disco é lido literalmente em vez de ser expandido, então `read` e `cat` concordam num arquivo como `relatorio[1].txt`
- O `search` deriva o teto do buffer de linha de `search.max_filesize` em vez de um limite fixo, então arquivo que o chamador já autorizou ler não é mais recusado por ter uma linha acima daquele limite, e o erro nomeia a chave a subir
- Pin: `cargo install atomwrite --locked --version "^0.1.37"`
- Superfície: 47 subcomandos, sem mudança desde a v0.1.36, e a fonte única continua sendo `atomwrite agent-surface`


## Trabalho dobrado na v0.1.37: lote do read e o jail do workspace
### O Que Muda
- O `read` aceita um ou mais caminhos. Caminho explícito sai na ordem do argv, e operando com `*`, `?` ou `[` é padrão que o próprio binário expande contra um nível do diretório dele, na ordem de bytes do `LC_ALL=C`, pulando nome que começa com ponto. O shell deixa de ser necessário para a expansão
- `--format raw` concatena todo alvo sem cabeçalho, sem separador e sem `\n` próprio, então `atomwrite read --format raw '*.rs' | cmp - <(LC_ALL=C cat *.rs)` sai 0 com diff vazio. Um alvo nesse formato sai byte a byte igual ao 0.1.37, e o formato NDJSON padrão não muda
- O lote copia em blocos de 64 KiB e confere o `--max-filesize` por alvo, então a memória fica plana em vez de somar os arquivos, e cada bloco avança a checagem de cancelamento, então o `--timeout-secs` corta o lote com exit 124
- Alvo que falha imprime uma linha em stderr e o run segue para o próximo. O código final é o semântico da primeira falha: 4 para caminho ausente, 13 para permissão, 126 fora do `--workspace`. O `cat` responderia 1 nos três casos, então consumidor que testa 1 precisa testar o código que o envelope nomeia
- Erro de `read --format raw` agora vai para stderr e nunca para stdout, porque aquele canal promete bytes e envelope JSON ali era o defeito
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` e `--verify-checksum` exigem alvo único e saem 65 com mais de um, e `read` com vários alvos sem `--format raw` também sai 65
- Padrão com `**` ou com mais de um separador sai 65 com mensagem própria, porque recursão está fora do escopo e casar outra coisa em silêncio é pior do que recusar
- Pin: `cargo install atomwrite --locked --version "^0.1.37"`
- Superfície: 47 subcomandos, sem mudança em relação à v0.1.37, e a fonte única continua sendo `atomwrite agent-surface`


## Notas da v0.1.37
### O Que Muda
- Avaliação de risco: `edit`, `edit-loop`, `replace`, `case`, `transform` e `codemod` passam a publicar `risk_assessment` quando o conteúdo que seriam gravados casa um padrão de risco, exatamente como o `write` já fazia. O campo fica ausente quando nenhum padrão casa, e as barreiras de tamanho continuam fora desses verbos de propósito, porque eles reescrevem uma região e um resultado menor é a operação funcionando
- Novo status de saída: `69` `GRAMMAR_UNAVAILABLE` com classe `precondition_failed`. Uma gramática tree-sitter ausente que não pode ser baixada chegava como 255 `INTERNAL_ERROR` com uma sugestão mandando abrir um bug; agora ela nomeia o idioma e diz para rodar o verbo uma vez com a rede ligada
- Superfície de configuração: `write.content_risk_patterns` passa a ser acessível por `config get` e `config set` como valor de lista, então os padrões de risco são configuração XDG e não literal compilado. A tabela tem 43 chaves
- `log.file` e `log.rotation` passam a ser aceitos também no escopo workspace: a seção `[log]` do `.atomwrite.toml` agora liga o destino de log, onde antes ela era validada e descartada
- O `--version` não acrescenta mais `-dirty`: o identificador é o id curto do commit da árvore rastreada, então um arquivo não rastreado não muda mais a string
- `batch --transaction` publica um veredito só: o evento `rollback` carrega `ok: false` com o `code` e o `exit` da falha que o causou, em vez de `ok: true` antes da falha
- Pin: `cargo install atomwrite --locked --version "^0.1.37"`
- Superfície: 47 subcomandos, sem mudança em relação à v0.1.37, e a fonte única continua sendo `atomwrite agent-surface`


## v0.1.36 para v0.1.37

### O Que Muda
- Shape de stream: `--sort-by` (`path-line` / `line-path` / `key`), `--dedupe KEY`, `--aggregate`, `--join`, `--diff-against FILE`, `--redact`, `--normalize-encoding` e `--max-output-bytes` entram no shape global. Ordenação, dedupe e comparação de stream bufferizam a execução inteira e são aplicadas no fim, então funcionam em todo subcomando sem flag por comando
- Chave de projeção: todo envelope de sucesso carrega `ok: true`; `verified` segue sendo veredito de checksum e continua exigindo `--verify`
- Sujeito do `transform`: os posicionais depois de `transform` são obrigatórios, porque verbo com efeito colateral recebe o alvo na própria linha de invocação
- Modo contínuo: o `watch` saiu do conjunto de features padrão. O binário padrão responde com o erro de feature fechada (exit 78, `code: CONFIG_INVALID`); o `agent-surface` reporta `available: false` e a dica de instalação com `--features watch`. Instale com `--features watch` para mantê-lo
- Descoberta de schema: `--print-schema` lista todo nome de schema, ou emite um pelo nome, sem subcomando. A flag precisa vir antes do nome do subcomando; depois dele, a chamada falha com erro de uso em vez de trocar o comando por uma listagem. As duas escritas funcionam, `--print-schema NAME` e `--print-schema=NAME`. Nome desconhecido sai pelo canal de dado como envelope de erro com `code: INVALID_INPUT` e exit 65, onde antes saía exit 1 sem payload nenhum
- Execução interrompida: todo verbo que varre encerra o canal de dado no envelope `cancelled`, que carrega `deadline_exceeded`, então prazo de `--timeout-secs` estourado (exit 124) se distingue de sinal (exit 130/143) sem interpretar o `reason`. O `search` e o `count` antes não publicavam nada ao cancelar, e um leitor podia tomar a última linha parcial por resultado final. O registro terminal de cancelamento é escrito sem shape, então `--filter`, `--select` e `--limit` não conseguem removê-lo
- Sinal de artefato: `extract-artifact` lê cabeçalho de PDF, OOXML e imagem e emite sinal delimitado, nunca os bytes brutos do artefato
- Caminho de reescrita: o teste que exigia `watch_summary` em ociosidade agora é gateado na feature `watch`, a matriz de backup passa sujeito ao `transform`, e a asserção da sugestão de jail fixa `--locale en`
- Contrato do `edit-loop`: tudo-ou-nada por padrão. Par sem match sai com 65 e nada é gravado. `edit-loop --partial` grava os pares que casam e sai com 87 (`EDIT_LOOP_PARTIAL`)
- Caminhos obrigatórios: `replace`, `scope`, `codemod`, `case`, `delete`, `backup` e `prune-backups` exigem seus caminhos. `wal-heal` e `scratch init` recebem o caminho como posicional, e `recipe run` exige `--path` ou `--target`. Sem alvo a saída é 2, e `replace foo src/` não lê mais `src/` como o texto novo
- Evidência de alvo: o evento terminal de todo verbo com efeito colateral traz `target_resolved` e `target_source` (sempre `argv`)
- Nota de correção acrescentada depois: essa linha descreve a v0.1.13, e o contrato atual está em `docs/AGENTS.md`, onde `target_source` é `argv` ou `stdin` e `target_resolved` é sempre array
- `doctor`: não lê nenhuma variável de ambiente. Os campos `ci`, `github_actions` e `sudo` saíram do bloco de ambiente, e o check de política sobre servidor de protocolo de ferramentas saiu do relatório
- `agent-surface`: a chave de protocolo de ferramentas saiu do manifesto
- Config estrita: chave desconhecida ou arquivo inválido em `config.toml` ou `.atomwrite.toml` sai com 78 (`CONFIG_INVALID`). A chave `version` (valor atual 1) é aceita, e uma maior é rejeitada
- Arquivo de log: desligado por padrão. Ligue com `[log] file = true` no `config.toml` XDG global (rotação diária ou horária, 7 arquivos, modo 0600)
- Lock de escrita: automático e multiplataforma. O `std::fs::File::try_lock` embrulha o `flock` no Unix e o `LockFileEx` no Windows, e o lock é tomado no próprio inode do alvo, então a escrita espera até 5 s pelo arquivo que vai mudar e sai com 83 (`LOCK_TIMEOUT`) no estouro. O sidecar `.<alvo>.atomwrite.lock` continua sendo tomado ao lado, sem garantia, o que mantém as versões antigas excluídas, e ele segura o lock sozinho quando o alvo ainda não existe. O Windows não foi validado
- Nome do sidecar de lock: ele mantém o literal `.<alvo>.atomwrite.lock` até 255 bytes, e acima disso o radical vira um digest de 16 caracteres do nome do alvo no mesmo diretório, então um alvo de nome longo deixa de falhar com `ENAMETOOLONG`
- `search`: respeita `--timeout-secs` em cerca de 1x, e todo `match` fica dentro do par `begin` / `end` do próprio arquivo. `files_skipped` no resumo conta os arquivos pulados por tamanho
- Stdin: write, edit, edit-loop, apply, hash `--stdin`, extract, diff, calc e regex obedecem SIGTERM (143), SIGHUP (129) e o prazo (124) com stdin aberto
- Verbo `config` (o 47º subcomando): `config path|list|get|set|unset|doctor` lê e grava 43 chaves persistentes. `--scope global|workspace` é obrigatório em `path`, `set` e `unset`; `list` e `get` mostram o valor efetivo e a origem. A gravação preserva os comentários do arquivo, valida o arquivo inteiro e entra de forma atômica (modo 0600 no arquivo global em Unix)
- Camadas de config: o `.atomwrite.toml` do workspace e o `config.toml` global agora são mesclados chave a chave (flag, depois workspace, depois global, depois embutido). Antes, o arquivo do workspace substituía o global inteiro
- Um evento terminal: o resumo que falha (`search`, `replace`, `transform`, `edit-loop`, `semantic-merge`, `recipe`, `scope`) carrega `ok: false`, `code`, `exit` e o detalhe do erro nele mesmo, e nenhum segundo envelope de erro vem depois. Quem esperava uma linha de erro depois do resumo deve ler o resumo
- `MATCH_FAILED` para par ausente: um par sem match no `edit-loop` ou num `edit` multi-par agora reporta o código `MATCH_FAILED` (exit 65, com `best_candidate`); antes, a mesma falha reportava `INVALID_INPUT`
- `files_failed` e `partial`: todo resumo que publica `files_failed` publica `partial: true` ao lado, e essa regra cobre `search`, `transform`, `replace`, `scope`, `count`, `list`, `sparse`, `semantic-search` e `case`. Os dois campos vêm da mesma contagem, então nunca divergem
- `errors[]`: cada entrada carrega `path`, `code` e `message`, e a lista para em `summary.errors_max_entries` entradas (50 por padrão) enquanto `files_failed` guarda a contagem total. `search`, `transform`, `replace` e `scope` mantêm exit 0 quando responderam e uma falha foi encontrada durante a varredura, enquanto `count`, `list`, `sparse`, `semantic-search`, `hash`, `copy`, `delete` e `case` reprovam a execução sobre uma árvore parcial, raiz nomeada na invocação que não existe sai 4 `FILE_NOT_FOUND` mesmo tendo casado, e execução sem match e com falha sai com a primeira falha
- Teto do fuzzy: arquivo acima de `[fuzzy].max_content_bytes` (padrão 256 MiB) falha no fuzzy com exit 65 e uma dica; `--fuzzy off` segue só exato; a estratégia `escape_normalized` é pulada acima de 32 MiB
- Pipe quebrado: exit 141 quando o SO entrega SIGPIPE e também quando a escrita devolve `BrokenPipe`
- Retry e limites: retry de lock e de rename no Windows usa backoff com jitter e respeita `--timeout-secs`; a contagem automática de workers para em 16 threads (um `-j N` explícito vale); o `search` mantém os dados de match em voo sob um orçamento de bytes (8 MiB por padrão)
- Heal de WAL na partida: roda só em verbos que gravam, e só nos diretórios citados no argv
- Build slim: `outline` e `sparse outline` saem 78 (`CONFIG_INVALID`) com dica de rebuild quando falta a feature `ast`
- Schemas: `docs/schemas/` tem 54 arquivos que acompanham o `--print-schema` (o procedimento para atualizá-los está em `docs/schemas/README.md`): eventos de sucesso listam `ok`, `code`, `exit`, `target_resolved` e `target_source` como propriedades opcionais, e todo verbo que escreve mais de um formato de registro passa a servir uma união (`recipe`, `codemod`, `batch`, `search`, `replace`, `list`, `backup`, `semantic-search`, `sparse`, `semantic-merge`, `case` e `watch`), enquanto `count-by-ext` e `count-by-size` são nomes novos
- Pin: `cargo install atomwrite --locked --version "^0.1.37"`

### Mudanças Que Quebram o Consumidor
- Guia de leitura: cada item diz o que o consumidor via antes da 0.1.37, o que vê agora e o exit code
- `apply` com diff unificado: cada hunk agora compara o contexto e as linhas removidas com o arquivo (espaços no fim ignorados) e é achado na linha do cabeçalho ou no vizinho exato mais próximo, até 64 linhas de distância. Divergência sai 65 `MATCH_FAILED` com `best_candidate` e nada é gravado. Antes, o patch `@@ -1 +1 @@ -zzz +yyy` trocava a linha 1 por `yyy` com exit 0
- `apply`, hunk só de inserção: insere DEPOIS da linha citada, onde antes inseria uma linha acima. Cabeçalho `@@` ilegível e patch sem hunk saem 65 `INVALID_INPUT`
- `apply` com SEARCH/REPLACE: qualquer bloco ausente falha o lote inteiro com `MATCH_FAILED` (exit 65) e nada é gravado, e um bloco que casa em dois lugares sai 65 `MATCH_AMBIGUOUS` com `match_count`. Antes, bloco ausente deixava o arquivo meio aplicado com exit 0 e `hunks_applied: 1`. O `apply` não tem `--partial`
- `semantic-merge` com `--fail-on-conflict` e conflito: exit 84 `MERGE_CONFLICT` (`error_class: permanent`, com `conflict_count`), onde saía 65 `INVALID_INPUT`. O resultado do merge com `status: "conflict"` é a única linha terminal
- `STATE_DRIFT` (exit 82, `retryable: true`) agora é possível em `case`, `transform`, `scope` e `codemod`: arquivo que mudou entre a leitura e a escrita falha para aquele arquivo e a execução deve ser repetida. Antes, o último a gravar sobrescrevia a mudança sem aviso
- Verbos de leitura-modificação-escrita serializam no lock advisory: `edit`, `edit-loop`, `set`, `del`, `config set`, `config unset`, `apply`, `write --append`, `write --prepend` e as ops `replace` e `edit` do `batch`. Trinta `edit` paralelos em linhas distintas saíam 0 com `ok: true` e guardavam só de 4 a 10 das 30 mudanças
- Onde o `flock` não existe (NFS) o lock degrada, e só `edit`, `replace`, `case`, `transform`, `scope` e `codemod` mantêm a rede de checksum, e isso não foi validado em NFS
- Entrada inválida do usuário é exit 65 `INVALID_INPUT` e não mais 255 `INTERNAL_ERROR`: a regex de `replace` e `search`, os globs de include e exclude em `search`, `replace`, `transform`, `scope`, `list`, `count`, `delete`, `sparse` e `hash -r`, padrões AST, regras YAML (arquivo e inline), `query -Q`, `read --grep` (era ignorado em silêncio), `locale --set` e as ops de `edit --multi`
- `sparse list --include '['` e `hash -r --exclude '['` saíam 0 em silêncio e agora saem 65. `sparse list --include` é aplicado (antes era ignorado) e `--exclude` é um glob de verdade
- Empate do fuzzy: bloco ausente entre linhas parecidas é `MATCH_FAILED` (exit 65, `match_count: 0`, com `best_candidate`), onde era `MATCH_AMBIGUOUS`. Empate real continua `MATCH_AMBIGUOUS` com `match_count: 2`. Afeta `edit`, `replace`, `apply` e `batch`
- `edit-loop` e `edit` multi-par: par sem match é `MATCH_FAILED` (era `INVALID_INPUT`), o `edit-loop` não grava nada sem `--partial`, e `--partial` sai 87 `EDIT_LOOP_PARTIAL`
- Todo resumo que publica `files_failed` reporta arquivo ilegível ou não gravável como `files_failed` e `errors[]` (no máximo `summary.errors_max_entries` entradas (50 por padrão)), mais `partial: true` sempre que um arquivo falhou. `search`, `transform`, `replace` e `scope` mantêm exit 0 quando responderam e uma falha foi encontrada durante a varredura, enquanto `count`, `list`, `sparse`, `semantic-search`, `hash`, `copy`, `delete` e `case` reprovam a execução sobre uma árvore parcial, então leia `partial` e `files_failed` antes de confiar num resultado. Raiz nomeada na invocação que não existe sai 4 `FILE_NOT_FOUND` mesmo tendo casado, e `--format text` escreve cada falha no stderr com o caminho e o motivo. Execução sem match e com falha sai com a primeira falha, onde saía 1 `NO_MATCHES`
- Valores `null` não são mais escritos no topo dos eventos do stdout: chave cujo valor seria `null` fica ausente (teste `r9_no_null_top_level_values`). Consumidor que comparava um campo com `null` deve testar a chave ausente, por exemplo `jaq '.campo // empty'`
- `--max-output-bytes` exige `--allow-truncate`, e cada flag é rejeitada sem a outra com exit 2 `ARGUMENT_PARSE_ERROR`. O corte vale para todo valor string, `checksum` incluído, então não use as duas quando o `checksum` ou o `content` inteiro for necessário
- `doctor --strict` com check falho: exit 1 `DOCTOR_CHECKS_FAILED` e uma linha terminal (o `doctor_report`), onde saía 255. O exit 1 é compartilhado com `NO_MATCHES`, então ramifique em `code`
- Pipe quebrado: exit 141 quando o leitor fecha o pipe, também no `read`, que saía 0 antes. O Unix instala um handler cooperativo de SIGPIPE (desvio consciente, ADR-0063): a escrita que acerta o pipe fechado falha com `BrokenPipe`, todo `Drop` roda e nenhum lock, journal ou tempfile fica. A saída fica incompleta, então reconfira o estado antes de assumir que o trabalho terminou
- `write` interrompido sai com o status real do sinal ou do prazo (143, 130 ou 124), onde saía 255
- `get`, `set` e `del` em arquivo TOML ou JSON inválido: exit 65 `INVALID_INPUT` só com linha, coluna e uma causa curta, onde saíam 1 sem NDJSON e citavam o arquivo inteiro na mensagem
- Falha sem tipo publica um envelope `INTERNAL_ERROR` (exit 255) no stdout, onde imprimia uma linha no stderr com exit 1
- `read --verify-checksum` com hash errado emite um envelope de erro e sai 81, onde stdout e stderr ficavam vazios
- `search.max_inflight_bytes`: o mínimo subiu de 1 MiB para 4 MiB, e um `.atomwrite.toml` que ainda diz 1 MiB é recusado ao carregar (exit 65 `INVALID_INPUT`, com a faixa aceita 4194304..=1073741824 na mensagem)
- A retenção de backup apaga só `<nome>.bak.<carimbo válido>`, onde `f.txt.bak.0-user-note` podia ser apagado
- `semantic-search --index-dir` fora do jail do workspace sai 126, onde lia e gravava fora do jail
- `case --subvert` rejeita OLD igual a NEW com exit 65, e `edit --multi` rejeita faixas sobrepostas com exit 65 e deixa o arquivo intacto

### API da Biblioteca
- A interface suportada do atomwrite é a CLI e o seu contrato NDJSON. O crate também publica uma biblioteca, e a superfície dela NÃO é coberta por SemVer nesta versão
- `cargo semver-checks --baseline-version 0.1.36` (cargo-semver-checks 0.50.0) nesta árvore sai 100 com 10 checks major falhos e 0 minor
- `auto_trait_impl_removed`: `LockGuard` deixou de ser `Send` e `Sync`
- `constructible_struct_adds_field`: campos públicos novos em `GlobalArgs`, `OutputShape`, `AtomwriteConfig`, `ErrorJson`, `CancelledEvent`, `EditLoopArgs`, `WalHealArgs`, `SearchSection`, `FuzzySection`, `LocaleState`, `MatchOpts` e `ScratchInitEvent`
- `enum_missing`: `locale::Idioma`, agora `locale::Language`
- `enum_no_repr_variant_discriminant_changed`: `LocaleSource::Persisted`, `LocaleSource::System` e `LocaleSource::Default` mudaram o discriminante
- `enum_variant_added`: `Commands::ExtractArtifact` e `Commands::Config` num enum exaustivo
- `enum_variant_missing`: `LocaleSource::EnvVar`
- `feature_not_enabled_by_default`: `watch`
- `function_missing`: `signal::reset_sigpipe`, `locale::negotiate_to_idioma` e `locale::resolved_idioma`
- `pub_module_level_const_missing`: `LOCK_POLL_FAST_ATTEMPTS`, `PERSIST_RETRY_DELAYS_MS`, `LOCK_POLL_SLOW_MS` e `LOCK_POLL_FAST_MS`
- `struct_pub_field_missing`: `RuntimeEnvironment.ci`, `RuntimeEnvironment.sudo` e `LocaleState.idioma`
- Projeto que depende do crate `atomwrite` como biblioteca deve fixar `=0.1.36` até portar o código. Projeto que só roda o binário não é afetado por esta lista

### Rollback
- Volte para a versão anterior com versão exata: `cargo install atomwrite --locked --version 0.1.36 --force`
- Um requisito como `^0.1.36` não é rollback, porque o Cargo o resolve para a 0.1.x mais nova, que é a 0.1.37
- O rollback traz de volta os comportamentos antigos listados acima, entre eles o `apply` sem conferência de diff unificado, que pode destruir conteúdo com exit 0

### Validação (v0.1.37)
- Reproduzido em `atomwrite 0.1.37 (no-git) x86_64-unknown-linux-gnu` num workspace isolado com `HOME` e `XDG_*` apontando para dentro dele
- `apply` de `@@ -1 +1 @@ -zzz +yyy` num arquivo sem `zzz`: `MATCH_FAILED`, exit 65
- `semantic-merge --fail-on-conflict` com os dois lados mudando a mesma linha: exit 84
- `replace --regex '('`, `list --include '['`, `hash -r --exclude '['` e `sparse list --include '['`: exit 65 `INVALID_INPUT`
- `get` num arquivo TOML que não faz parse: exit 65
- `--max-output-bytes 10 read ARQUIVO`: exit 2, e com `--allow-truncate` sai 0 e corta `checksum` em 10 caracteres
- `read ARQUIVO | head -c 5` e `search PADRAO ARQUIVO | head -1` num arquivo de 50000 linhas: exit 141
- `write` com stdin aberto e `--timeout-secs 1`: exit 124 com um evento `cancelled` que traz `deadline_exceeded: true`
- `.atomwrite.toml` com `[search] max_inflight_bytes = 1048576`: exit 65 `INVALID_INPUT`
- `semantic-search "unwrap value" src/ --index-dir /tmp/FORA`: exit 126 `WORKSPACE_JAIL`
- `case --subvert foo foo`: exit 65, e `read ARQUIVO --verify-checksum deadbeef`: exit 81 `CHECKSUM_VERIFY_FAILED`
- `--print-schema` lista 55 nomes e `--print-schema nope` sai 65
- O envelope de falha não carrega mais `error`: `read FALTANDO` publica `{"ok":false,"code":"FILE_NOT_FOUND","exit":4,...}`, e quem ramificava por `has("error")` passa a testar `.ok == false and .code`
- Não reproduzido aqui: `STATE_DRIFT` sob corrida real, `doctor --strict` com check falho (este host não tem nenhum), `write` interrompido por SIGTERM e SIGINT, NFS, macOS e Windows

### Os 47 Subcomandos na 0.1.37
- Um exemplo por subcomando, cada um executado contra o `atomwrite 0.1.37` num workspace isolado; a lista é a do `atomwrite agent-surface`
- Preparação: `WS` é o caminho absoluto de um workspace vazio que contém `src/a.rs` (um `unwrap()` e um comentário), `src/b.rs` (a palavra `beta` uma vez), `src/c.rs` (com `user_id`), `src/tmp.rs`, `config.toml`, `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` e `rules.yaml`; `HASH_A` é o `checksum` que `hash src/a.rs` imprime
- Os exemplos rodam na ordem abaixo (o `rollback` precisa do `backup` anterior e o `edit-loop` precisa do `edit` anterior). 46 dos 47 saem 0, e o `watch` sai 78 porque o build padrão não tem a feature `watch`
- O texto completo da preparação está em `docs/INSTALL.pt-BR.md`

- `read` — Lê um arquivo com checksum, tamanho e linhas, com opção de só início, fim ou faixa. Exemplo: `atomwrite --workspace "$WS" read --head 2 src/a.rs`
- `write` — Cria ou sobrescreve um arquivo de forma atômica a partir do stdin. Exemplo: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` — Edita um arquivo por texto exato ou aproximado, número de linha ou marcador. Exemplo: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `delete` — Apaga arquivos ou árvores com backup, ou só mostra o plano. Exemplo: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `copy` — Copia um arquivo ou árvore com verificação de checksum. Exemplo: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `move` — Move ou renomeia um arquivo de forma atômica. Exemplo: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `apply` — Aplica um patch (diff unificado, SEARCH/REPLACE, arquivo inteiro) vindo do stdin. Exemplo: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `search` — Busca conteúdo de arquivos em paralelo com o motor do ripgrep. Exemplo: `atomwrite --workspace "$WS" search alpha src/`
- `replace` — Troca texto em vários arquivos com escrita atômica e casamento aproximado. Exemplo: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `transform` — Reescreve código por padrão de AST (ast-grep). Exemplo: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `hash` — Mostra o checksum BLAKE3 de arquivos. Exemplo: `atomwrite --workspace "$WS" hash src/a.rs`
- `count` — Conta linhas e arquivos por extensão ou por tamanho. Exemplo: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` — Compara dois arquivos (unificado, estatística ou mudanças). Exemplo: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `list` — Lista uma árvore de diretórios com metadados. Exemplo: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` — Extrai campos de NDJSON lido do stdin. Exemplo: `printf '{"a":1}\n' | atomwrite --workspace "$WS" extract --field a`
- `extract-artifact` — Sinal limitado dos cabeçalhos de PDF, OOXML e imagem (nunca os bytes brutos). Exemplo: `atomwrite --workspace "$WS" extract-artifact img.png`
- `scope` — Escopo gramatical: age sobre comentários, strings, funções e outras categorias de código. Exemplo: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `regex` — Gera uma regex a partir de exemplos. Exemplo: `atomwrite --workspace "$WS" regex abc-12 xyz-99 id-7 --digits`
- `calc` — Cálculo e conversão de unidades. Exemplo: `atomwrite --workspace "$WS" calc "2 hours to seconds"`
- `completions` — Gera ou instala completions de shell (bash, zsh, fish, elvish, powershell). Exemplo: `atomwrite --workspace "$WS" completions bash`
- `backup` — Cria backups com data e hora e checksum BLAKE3. Exemplo: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` — Restaura um arquivo a partir de um backup anterior. Exemplo: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `batch` — Executa operações NDJSON do stdin, com opção de transação única. Exemplo: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
- `edit-loop` — Aplica N pares antigo/novo numa escrita só, tudo ou nada. Exemplo: `printf '[{"old":"gamma","new":"delta"}]\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` — Remove arquivos .bak antigos por idade ou quantidade. Exemplo: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run --recursive .`
- `wal-stats` — Mostra o estado dos journals WAL (só leitura). Exemplo: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` — Remove journals WAL terminais órfãos mais antigos que um limite. Exemplo: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `verify` — Confere um arquivo contra um hash BLAKE3 esperado (exit 0 se igual, 81 se diferente). Exemplo: `atomwrite --workspace "$WS" verify src/a.rs HASH_A`
- `set` — Grava um valor num caminho com pontos em arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` — Lê um valor num caminho com pontos em arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` — Remove uma chave de um arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` — Renomeia identificadores entre estilos de caixa em vários arquivos. Exemplo: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` — Percorre a AST do tree-sitter e emite os nós. Exemplo: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` — Emite a estrutura de alto nível de um arquivo de código. Exemplo: `atomwrite --workspace "$WS" outline src/a.rs`
- `semantic-merge` — Merge de três vias por linha com marcadores de conflito (não é AST). Exemplo: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `sparse` — Lista, leitura e outline com orçamento para árvores grandes. Exemplo: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576`
- `recipe` — Lista e roda receitas embutidas de vários passos. Exemplo: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `stat` — Só metadados, alias de read --stat. Exemplo: `atomwrite --workspace "$WS" stat src/a.rs`
- `agent-surface` — Manifesto de ferramentas para agentes (a fonte única dos subcomandos). Exemplo: `atomwrite --workspace "$WS" agent-surface --format json`
- `watch` — Transmite eventos do sistema de arquivos com debounce e termina com watch_summary (exige build com `--features watch`, o build padrão sai 78). Exemplo: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500`
- `codemod` — Aplica uma lista YAML de regras de reescrita por AST numa árvore. Exemplo: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` — Ranking offline por Jaccard de arquivos para uma consulta. Exemplo: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `doctor` — Diagnóstico de prontidão do host. Exemplo: `atomwrite --workspace "$WS" doctor`
- `scratch` — Cria o diretório de staging .atomwrite/scratch dentro do jail do workspace. Exemplo: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`
- `locale` — Mostra, define ou limpa o idioma da interface. Exemplo: `atomwrite --workspace "$WS" locale`
- `commands` — Imprime a árvore completa de comandos em JSON. Exemplo: `atomwrite --workspace "$WS" commands --include-globals`
- `config` — Lê e grava chaves de ajuste persistentes (43 chaves, escopos global e workspace). Exemplo: `atomwrite --workspace "$WS" config get fuzzy.threshold`

### Migração Passo a Passo

1. Pin `^0.1.37`
2. Se você depende do `watch`, instale com `--features watch` ou migre para um laço de polling com `list`
3. Troque passos inline de `sort` / `uniq` / `diff` por `--sort-by`, `--dedupe` e `--diff-against`, para a CLI responder no canal de dado
4. Passe um sujeito ao `transform`; chamada sem caminhos agora é erro de uso em vez de varredura implícita do diretório corrente
5. Parser que lê veredito de checksum continua lendo `verified`; parser que lê o resultado da operação agora encontra `ok` em todo envelope de sucesso
6. Mova o `--print-schema` para o começo da linha, antes do nome do subcomando, e leia nome desconhecido do envelope de erro em vez de um exit 1 sem payload
7. Leia `deadline_exceeded` no envelope `cancelled` para distinguir prazo de sinal. Consumidor que tratava a última linha de um `search` cancelado como resultado final agora precisa esperar o registro de cancelamento ali
8. Em quem chama o `edit-loop`, trate o exit 65 como "nada gravado" e passe `--partial` só quando gravação parcial for aceitável, tratando então o exit 87
9. Passe caminho explícito a `replace`, `scope`, `codemod`, `case`, `wal-heal`, `scratch init` e `recipe run`, e leia `target_source` para confirmar qual alvo rodou
10. Pare de ler `ci`, `github_actions` e `sudo` do `doctor`, e pare de ler a chave de protocolo de ferramentas do `agent-surface`
11. Valide todo `config.toml` e `.atomwrite.toml` contra o schema estrito antes de atualizar, porque chave desconhecida agora interrompe a execução com exit 78
12. Não passe flag de lock nem flag de espera de lock: as flags que docs antigas citavam nunca existiram, e o lock é automático
13. Mova o ajuste que vivia num TOML editado à mão para `config set CHAVE VALOR --scope global|workspace`, e confira que um arquivo de workspace não esconde mais as chaves globais que ele não cita
14. Em scripts que leem a última linha de `search`, `replace` ou `transform`, leia `files_failed` e `errors[]` no resumo, e trate um resumo com `ok: false` como a única linha de erro da execução
15. Ramifique em `MATCH_FAILED` para par ausente do `edit-loop` e leia `best_candidate` como no `edit`
16. Revise todo script que dependia de exit 255 ou exit 1 para entrada ruim: regex, glob, padrão AST ou regra YAML inválidos saem 65, `doctor --strict` com check falho sai 1 `DOCTOR_CHECKS_FAILED`, `write` interrompido sai 143, 130 ou 124, e `get`, `set` e `del` em arquivo inválido saem 65
17. Trate o exit 65 do `apply` como "nada gravado": leia `best_candidate`, corrija o contexto do patch e rode de novo, sem reenviar o mesmo patch
18. Trate o exit 84 (`MERGE_CONFLICT`) do `semantic-merge --fail-on-conflict` e o exit 82 (`STATE_DRIFT`, retryable) de `case`, `transform`, `scope` e `codemod`: repita a execução para os arquivos que falharam
19. Se você passa `--max-output-bytes`, acrescente `--allow-truncate`, e não use o par quando precisar do `checksum` ou do `content` inteiro
20. Pare de comparar campos de topo com `null`: teste a chave ausente
21. Procure `[search] max_inflight_bytes` abaixo de 4 MiB em todo `.atomwrite.toml`, porque o arquivo é recusado com exit 65
22. Se o seu código depende do crate `atomwrite` como biblioteca, fixe `=0.1.36` ou porte o código (veja API da Biblioteca)
23. Trate o exit 141 como saída incompleta e confira o estado dos arquivos antes de seguir


### Veja Também
- `CHANGELOG.md`, seção 0.1.37, para a lista completa de mudanças, medições e notas
- `docs/INSTALL.pt-BR.md` para os perfis de instalação, os tamanhos, os caminhos XDG e as 43 chaves de config
- `docs/schemas/README.md` para os 54 arquivos de schema e o mapa nome-arquivo
- `docs/decisions/README.md` para o ADR-0058 em diante (as decisões da v0.1.37, entre elas o handler de SIGPIPE no ADR-0063)
- `docs/CROSS_PLATFORM.pt-BR.md` para o que foi e o que não foi validado em cada plataforma


## v0.1.35 para v0.1.36 (substituída pela v0.1.37)

### O Que Muda
- Staging: `atomwrite scratch init .atomwrite/scratch` e `--old-file`/`--new-file` sob `.atomwrite/scratch/` (WORKSPACE_JAIL 126 em `/tmp` do SO permanece)
- Envelope: `ok: false` + `match_count` em falhas
- Stdout agent-native: `--select`/`--fields`, `--filter`, `--limit` (só long — sem `-n` global). Erros always-emit mesmo com `--filter` agressivo; `--limit` só conta sucesso
- extract: `--field` local (repetível) ou posicionais; `--select` global projeta a linha (não pode haver `--select` local — colide com o global). Planos/CAPA legados que citam `extract --select` → use `extract --field`
- Fuzzy: `similarity: null` em estratégias normalizadas; dual-gate block_anchor; `soft_floor_delta` no XDG
- Histórico (v0.1.36): 46 subcomandos SSOT (`Commands` / `agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`) — não “41” nem “42” incompleto
- Pin histórico: `cargo install atomwrite --locked --version "^0.1.36"`

### Passos

1. Pin `^0.1.37`
2. Substitua scratch em `/tmp` por `.atomwrite/scratch/`
3. Cheque exit e `ok` antes de campos de sucesso (stdout ainda pode ter linha de erro com exit ≠ 0 sob `--filter`)
4. Use `--select` e `--limit N` em vez de jq/`-n` quando bastar projeção top-level
5. Gate: `cargo test --tests`


## v0.1.34 para v0.1.35

### O Que Muda
- Sobrescrita grande default-deny (A-WRITE-001): sobrescrever arquivo existente maior que XDG `[write].confirm_large_bytes` (bootstrap 100 KiB) exige `--ack-overwrite`. Sem isso, exit 65. Sem prompt Y/N (one-shot).
- `--require-large-ack` independente (A-WRITE-002): não é mais alias clap de `--confirm`. Pode passar ambos; alvos grandes ainda precisam de `--ack-overwrite`.
- `delete --confirm` / `delete --yes` rejeitados (B-014/B-015 / A-TEST-001): use `delete --plan` ou `--dry-run` para plan-only; omita flags para apagar de fato. Atualize scripts/testes que ainda passam `--yes` ou `-y`.
- `watch` sempre emite `watch_summary` (A-WATCH-001): mesmo com zero eventos / idle, stdout inclui linha NDJSON final `type:watch_summary`.
- Idle do watch 500 ms (A-ONESHOT-001): era ~3000 ms. Override com `--idle-exit-ms` ou XDG / `.atomwrite.toml` `[watch].idle_exit_ms`.
- Markers do semantic-merge default ON (A-MERGE-001): saída com conflito grava `<<<<<<< ours`. Prefer-ours sem markers: `--no-conflict-markers` ou `--write-conflict-markers false`.
- Debounce watch XDG (A-XDG-002): `[watch].debounce_ms` + CLI `--debounce-ms`.
- Multi-OS honesto (A-XPLAT-001): Linux e2e completo; Windows-gnu `cargo check` no host Linux; macOS precisa de Mac real ou osxcross. Só gates locais.
- Superfície de config: knobs de produto só CLI + XDG / `.atomwrite.toml` (sem knobs `ATOMWRITE_*` em runtime). Desde a 0.1.37 o `doctor` não lê nenhuma variável de ambiente (só sondas de arquivo).

### Migração Passo a Passo

1. Histórico (pin 0.1.34 → 0.1.35 naquela release): `cargo install atomwrite --locked --version "^0.1.35"` (ou `cargo install --path . --force` a partir do fonte). Linha atual é `^0.1.38`.
2. Agentes que sobrescrevem arquivos grandes: adicionar `--ack-overwrite` (e opcionalmente `--require-large-ack`).
3. Agentes que usavam `delete --confirm` como “preview”: trocar para `delete --plan`.
4. Agentes que usavam `delete --yes` / `-y`: remover essas flags; agora falham com exit 65.
5. Parsers que assumiam stdout vazio em `watch` idle: aceitar `type:watch_summary`.
6. Pipelines que esperavam arquivo prefer-ours limpo em conflito: passar `--no-conflict-markers` se markers forem indesejados.
7. Reexecutar gates locais: `cargo test --lib --tests`, `cargo clippy --all-targets -- -D warnings`, `cargo test --test cli_e2e_v0135`.

### Notas de Compatibilidade
- Writes não grandes e o contrato fuzzy one-shot de 0.1.33/0.1.34 permanecem.
- Histórico (na v0.1.35): a superfície documentada era 41 subcomandos. Atual (v0.1.37): SSOT é 47 (`doctor`, `locale`, `commands`, `scratch`, `extract-artifact` e `config` completam a superfície) — não trate “41 permanecem” como atual.
- Veja o [ADR-0055](decisions/0055-v0-1-36-agent-contract-gaps.md) para a matriz de closeout A-*; veja [v0.1.35 para v0.1.36](#v0135-para-v0136-substituída-pela-v0137) para extract `--field`, shape global e staging scratch.

### Rollback
- Pin de volta para `^0.1.34` se precisar restaurar large-write opt-in e stdout vazio do watch (não recomendado para agentes).


## v0.1.35 — sem knobs de ambiente de produto

Breaking (config runtime): a CLI não lê mais variáveis `ATOMWRITE_*` nem `RUST_LOG`/`NO_COLOR` como configuração de produto.
Use apenas:
- flags CLI (`--workspace`, `--locale`, `--no-backup`, `-v`/`-q`, `--color`, `--threads`, …)
- `atomwrite set` / `get` + XDG `config.toml` (`[storage].home`, `[fuzzy].threshold_*`, …)
- opcional `.atomwrite.toml` no workspace

Exemplos antigos com `export` de knobs de produto workspace/backup (removidos) devem ser reescritos para `--workspace` e `--no-backup`.
Logging: `init_tracing` + CLI; tee de arquivo sob state XDG — sem knobs de ambiente de log de produto.
`wal-stats` é diagnóstico local (não telemetria de produto).


## v0.1.33 para v0.1.34

### Alinhamento de docs (mesmo runtime one-shot da 0.1.33)
- Sem novo comportamento de runtime em relação à v0.1.33. O fix de código foi registrado como 0.1.33; 0.1.34 é a publicação docs-complete.
- A documentação (AGENTS, HOW_TO_USE, COOKBOOK, TESTING, INSTALL, MIGRATION, ADRs) agora documenta:
  - Global `--timeout-secs` / `--timeout` padrão 120, `0` desabilita, prazo esgotado exit 124
  - Multi-apply fuzzy one-pass E→D (`apply_fuzzy_one_pass`); nunca reescaneia texto inserido
  - Máximo de applies fuzzy padrão 1; teto rígido 10_000; embeds forçam apply único; caps e cancel cooperativo no meio da cascata
- A suíte de regressão permanece `tests/cli_v0133_oneshot_fuzzy.rs`

### Ação de migração
- Pin `^0.1.34`: `cargo install atomwrite --locked --version "^0.1.34"` ou `cargo install --path . --locked --force`
- Se você já roda 0.1.33, nenhuma mudança de comportamento é necessária — só pin de versão e docs
- Rode: `cargo test --test cli_v0133_oneshot_fuzzy`


## v0.1.32 para v0.1.33

### Mudanças BREAKING
- Default de `--timeout-secs` é 120 (era 0). Jobs longos: `--timeout-secs 0` ou valor maior. Prazo esgotado → exit 124. Alias `--timeout`.
- Fuzzy multi default 1 apply se `--max-replacements` omitido (era ilimitado). `--max-replacements N` explícito ainda multi-hit one-pass (nunca reescaneia texto inserido).
- Se `replacement` contém `pattern` (string fixa), fuzzy força apply único mesmo com `--max-replacements` grande (previne crescimento infinito).
- Teto rígido de applies fuzzy: 10_000 (`FUZZY_HARD_MAX_REPLACEMENTS`), mesmo com `--max-replacements` maior.
- Caps: pattern 64 KiB; lev 8192 chars; max windows 4096; growth max(4×, +16 MiB).

### Corrigido (crítico)
- Hang infinito de `replace --fuzzy` quando NEW embute OLD (expand section de agentes).

### Ação de migração
- Agentes: continue usando `replace --fuzzy auto`; o hang foi corrigido. Prefira blocos OLD únicos; NEW pode embutir OLD com segurança (expansões one-pass).
- Jobs longos/CI que dependiam de runtime ilimitado: passe `--timeout-secs 0` somente se for intencional.
- Prefira pin `^0.1.34` (docs-complete); `^0.1.33` tem o mesmo runtime.
- Regressão: `cargo test --test cli_v0133_oneshot_fuzzy`


## v0.1.29 para v0.1.30

### Mudanças BREAKING
- `--fuzzy off` é rejeitado (exit 65) com nota de migração; só restam `auto` e `aggressive`. Substituído na v0.1.35: Off = exact-only (G-010 CLOSED).
- `.atomwrite.toml` com `[fuzzy] mode = "off"` falha no parse com INVALID_INPUT. Substituído na v0.1.35: `mode = "off"` é exact-only.
- Multi-match sem `--replace-all` falha com MATCH_AMBIGUOUS (exit 65)

### Adicionado (relevante para migração)
- Campos NDJSON do EditOutput: `match_count` e opcional `indent_adjusted`
- Guarda escape-drift; estratégia unicode_normalized; best_candidate e candidates[] ranqueados
- search --target content|files|both com --offset e --limit; similar_paths em path miss
- recipe hash exclui *.bak.*; sparse outline emite outline_item AST real sob orçamento
- config [fuzzy] aplicado em edit/replace/loop/batch; platform.backup_method é reflink_or_copy
- Suíte de contrato tests/cli_v0130_agent_contract.rs

### Ação de Migração
- Remova `--fuzzy off` de scripts e prompts de agente; use auto ou aggressive (conselho histórico 0.1.30; na v0.1.35 `off` voltou a ser exact-only)
- Remova mode = "off" do .atomwrite.toml [fuzzy] (histórico 0.1.30; na v0.1.35 exact-only é permitido)
- Em edição multi-ocorrência passe --replace-all e parseie match_count com jaq
- Prefira jaq em match_count e indent_adjusted a reler o arquivo inteiro após edit
- Atualize pin: cargo install atomwrite --locked --version "^0.1.30"
- Rode: cargo test --test cli_v0130_agent_contract


## v0.1.28 para v0.1.29

### Mudanças BREAKING
- O `replace` de string fixa agora usa `fuzzy=auto` por padrão após multi-match exato com zero hits — antes a falha exact-only retornava exit 1 sem fallback fuzzy
- Pipelines que exigiam falha exact-only (exit 1 quando a string fixa está ausente) não devem depender de exact-only (na v0.1.30 `--fuzzy off` é rejeitado). Substituído na v0.1.35: `--fuzzy off` = exact-only (G-010); miss exato → `MATCH_FAILED` exit 65.

### Adicionado (relevante para migração)
- Módulo compartilhado `fuzzy`: cascata de 9 estratégias para `edit`, `replace`, `batch`, `edit-loop`
- Diagnósticos estruturados `best_candidate` em falha de match (envelopes de erro, caminhos de exit 65)
- `replace --fuzzy auto|off|aggressive` e `--fuzzy-threshold` (só na 0.1.29; `off` rejeitado desde 0.1.30)
- `replace --progress-every` e heartbeats de progresso em batch
- Features Cargo: `core`, `ast`, `lang-*`, `watch`, `semantic`, `full` — install slim sem AST (histórico: a feature `semantic` foi removida depois e o `semantic-search` é sempre compilado)
- Novos subcomandos: `semantic-merge`, `sparse list|read`, `recipe list|run`, `stat` (alias de `read --stat`), `agent-surface`, `watch` (feature `watch`), `codemod`, `semantic-search`
- `write --durability full|fast|auto` com NDJSON `platform.durability`
- Linux `renameat2` em `atomic_rename` com fallback ENOSYS
- Backup usa reflink_or_copy (nunca hardlink do arquivo vivo)
- Cancelamento cooperativo durante leitura de stdin e escrita atômica em chunks
- Documentação versionada de recipes em `recipes/*.yaml`
- Local gates: size-gate (slim core ≤ 15 MiB), core-test, schema-diff

### Alterado
- Numeração de estratégias normalizada para nove estratégias nomeadas
- Histórico (na v0.1.29): superfície skill documenta 41 subcomandos
- Release slim core ~7.7 MiB (size-gate local ≤ 15 MiB); default/full com AST ~52 MB — PRD 5–8 MB aplica-se somente ao core

### Ação de Migração
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.30"`
- BREAKING (v0.1.30): exact-only via `--fuzzy off` foi rejeitado; restaram só auto/aggressive. Substituído na v0.1.35 (G-010): `--fuzzy off` = exact-only de novo; miss exato em edit → `MATCH_FAILED` exit 65 (não é o mesmo que `NO_MATCHES` exit 1 em árvore)
- Em falhas de match, inspecione o campo opcional `best_candidate` no envelope NDJSON de erro para diagnósticos de near-miss
- Install slim (sem AST): `cargo install --path . --locked --no-default-features --features core`
- Install full (features default): `cargo install --path . --locked` ou `cargo install atomwrite --locked --version "^0.1.30"`
- Use `agent-surface` para inventário de ferramentas derivado do clap (integração por subprocesso, CLI com saída NDJSON)
- Durabilidade opcional: `write --durability full|fast|auto` — leia `platform.durability` no NDJSON
- Operadores Linux: rename atômico pode usar `renameat2` (fallback em ENOSYS)
- Feature-gated: habilite `watch` para o subcomando `watch` (histórico: não existe feature `semantic` na 0.1.37, o `semantic-search` é sempre compilado)
- MSRV inalterado em Rust 1.88

### Cobertura de Testes
- Build core-only funciona sem crates AST (stubs com exit 78)
- Recipe run despacha search→replace→hash de verdade (não é mais stub somente de plano)
- Schemas regenerados: write durability/rename_method, replace fuzzy*, error best_candidate, recipe/progress/cancelled


## v0.1.27 para v0.1.28 (2026-07-06)

### Breaking Changes
- `delete` agora cria backup por padrão (era opt-in) — passe `--no-backup` para desabilitar; `--keep-backup` em `delete` é redundante (backups de deleção sempre são preservados e nunca auto-removidos) e agora expõe um campo `warnings` no envelope NDJSON em vez de um no-op silencioso
- `move`/`copy` sobrescrevendo um destino existente agora exigem `--force` OU um `--backup` explícito — antes os dois subcomandos aplicavam essa regra de forma inconsistente

### Correções de Bugs (Críticas) — GAP-CLI-SURFACE-DRIFT
- Extraída uma única struct `BackupOpts` (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) achatada via `#[command(flatten)]` em 15 subcomandos mutantes: `write`, `edit`, `edit-loop`, `replace`, `transform`, `scope`, `apply`, `set`, `del`, `case`, `batch`, `delete`, `move`, `copy`, `rollback`
- `replace --retention` e `delete --no-backup` agora parseiam com sucesso — antes ambos retornavam exit 2 `ARGUMENT_PARSE_ERROR` porque cada struct redeclarava as flags de backup manualmente com presença e defaults divergentes
- `--backup` e `--no-backup` agora são mutuamente exclusivos via `conflicts_with` (exit 2) em vez da última flag silenciosamente vencer
- `rollback` mantém seu snapshot de segurança pré-rollback como opt-in via `--backup` explícito, documentado como exceção intencional ao contrato unificado default-true
- Adicionado um guard compartilhado de stdin-tty no `edit`: `--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`, e `--multi` agora falham rápido com exit 65 `INVALID_INPUT` e uma `suggestion` acionável quando stdin é um terminal, em vez de bloquear indefinidamente (antes exigia um `timeout` externo e saía com 143)
- Os modos consumidores de stdin do `edit` agora rejeitam `--old`/`--new`/`--old-file`/`--new-file` em tempo de parse via `conflicts_with_all` (exit 2) em vez de silenciosamente despachar para `edit_by_marker`
- Removido `retention: 5` hardcoded de 9 pontos de chamada; retention agora flui de ponta a ponta de `BackupOpts` através de um único `resolve_backup()`
- `batch --retention <N>` era aceito mas silenciosamente ignorado — agora efetivo de ponta a ponta com precedência CLI `--retention` > `.atomwrite.toml` `[defaults].retention` > default embutido (`5`), incluindo o passo de pre-backup transacional do batch
- `batch --backup` passado explicitamente agora força backup para toda operação no batch, simétrico a `--no-backup`

### Correções de Bugs (Altas) — GAP-CONFIG-DEFAULTS-DEAD
- As chaves `backup`/`retention` de `[defaults]` do `.atomwrite.toml` agora são efetivas em todo subcomando mutante — `resolve_backup()` antes recebia apenas `(backup: bool, no_backup: bool)` e nunca consultava a config parseada
- A precedência agora é flags CLI (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > config XDG > default embutido (`true` / `5`) — sem knobs de env de produto (v0.1.35)

### Ação de Migração
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.28"`
- BREAKING: se você depende de `delete` nunca criar backup por padrão, passe `--no-backup` explicitamente — o default inverteu de opt-in para opt-out
- BREAKING: se você script `move`/`copy` para sobrescrever um destino existente, passe `--force` ou `--backup` — os dois subcomandos agora aplicam essa regra de forma consistente
- Se você contornava `replace --retention` ou `delete --no-backup` retornando exit 2: ambos agora parseiam com sucesso
- Se você passa `--backup` e `--no-backup` juntos: isso agora falha rápido com exit 2 em vez da última flag silenciosamente vencer
- Se seu `.atomwrite.toml` define `[defaults]` `backup`/`retention`: agora são respeitados em todo subcomando mutante — verifique se a config do seu projeto reflete o comportamento pretendido
- Se você invoca os modos consumidores de stdin do `edit` (`--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between`, `--multi`) interativamente sem pipe de stdin: isso agora falha rápido com exit 65 `INVALID_INPUT` em vez de bloquear indefinidamente
- MSRV inalterado em Rust 1.88

### Cobertura de Testes
- 661 testes passando, 0 falhas, 3 ignorados
- 33 subcomandos, 32 ADRs em `docs/decisions/`


## v0.1.26 para v0.1.27 (2026-06-24)

### Correções de Segurança (CRÍTICAS)
- BUG-SEC-001: escape do jail do workspace via symlink-directory — `ln -s /tmp $WS/link && atomwrite write link/file.txt` conseguia escrever FORA do workspace. Corrigido com `canonicalize_existing_prefix` em `path_safety.rs`. Afeta: write, read, edit, edit-loop, apply, set, del, copy, move. Exit 126 na detecção.

### Correções de Bugs (CRÍTICAS)
- BUG-SCOPE-004: `scope --query comments --delete` em linhas com comentários inline (ex: `fn foo() {} // comment`) destruía a linha INTEIRA incluindo código válido. Corrigido: `expand_to_full_line` agora preserva código antes de comentários inline.

### Correções de Bugs (Médias)
- BUG-GET: `get` para chave ausente agora retorna exit 65 (`INVALID_INPUT`) com mensagem "key 'X' not found in Y" — reverte o revert da v0.1.26 para exit 4
- BUG-001: `edit-loop` com stdin vazio agora retorna exit 65 (`INVALID_INPUT`) em vez de exit 0 silencioso
- BUG-002: `edit-loop` com JSON inválido agora emite envelope de erro NDJSON no stdout em vez de texto não estruturado no stderr
- BUG-005: sugestão do `write --syntax-check` mudou de "--syntax-check=false" para "remove --syntax-check"
- BUG-008: `pair_results` do `edit-loop` agora inclui campos `old` e `new` para cada par
- BUG-SCOPE-002: queries `fn`, `struct`, `enum`, `trait`, `async-fn`, `unsafe-fn` do `scope` agora incluem variantes `pub`
- BUG-SCOPE-003: queries `const`, `static`, `type-alias`, `mod`, `use` do `scope` agora incluem variantes `pub`
- BUG-SCOPE-005: `scope --query var` em Go agora casa declarações sem tipo explícito (`var x = 0`)

### Limitações Conhecidas
- GAP-01: `scope --query test-fn` indisponível — ast-grep não consegue casar `#[test] fn` através de 2 nós AST
- GAP-02: `scope --query doc-comment` indisponível — tree-sitter parseia `///` como `line_comment` (sem distinção estrutural)
- GAP-03: `scope --query export` em JS/TS indisponível — padrão cruza múltiplos nós AST

### Ação de Migração
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.27"`
- CRÍTICO: Se você usa symlinks dentro de diretórios workspace, teste que eles resolvem dentro do jail. Symlinks apontando para fora do workspace agora corretamente retornam exit 126.
- Se parseia exit codes de `get` para chave ausente: exit agora é 65 (`INVALID_INPUT`) novamente (era 4 na v0.1.26, 65 na v0.1.25)
- Se parseia `pair_results` do `edit-loop`: novos campos `old` e `new` presentes
- Se depende de queries do `scope` casando apenas items não-pub: `fn`, `struct`, `enum`, `trait`, `const`, `static`, `type-alias`, `mod`, `use` agora capturam pub e não-pub
- MSRV inalterado em Rust 1.88

### Cobertura de Testes
- 631+ testes passando, 0 falhas
- 33 subcomandos, 29 ADRs em `docs/decisions/`


## v0.1.25 para v0.1.26 (2026-06-23)

### Correções de Bugs (Podem Afetar Comportamento)
- `replace` com zero matches agora retorna exit 1 (`NO_MATCHES`) em vez de exit 0 (GAP-141)
- `transform` com zero matches agora retorna exit 1 (`NO_MATCHES`) em vez de exit 0 (GAP-145)
- `scope` com zero matches agora retorna exit 1 (`NO_MATCHES`) em vez de exit 0 (GAP-146)
- `read --verify-checksum` com hash errado agora retorna exit 81 (`CHECKSUM_VERIFY_FAILED`) em vez de exit 82 (GAP-136)
- `get` para chave ausente REVERTIDO para exit 4 (`FILE_NOT_FOUND`) — reverte a mudança da v0.1.25 GAP-111 para exit 65 (GAP-137)
- `search --sort path` agora garante ordenação global determinística via buffer BTreeMap (GAP-147)
- Queries Rust do `scope` agora usam padrões AST duais para capturar generics e return types (GAP-144)
- Checksum do `verify` agora é argumento posicional: `verify <PATH> <HASH>` (GAP-135)
- `edit-loop` agora aceita input JSON array além de NDJSON (GAP-142)

### Esclarecimentos de Documentação (BY-DESIGN)
- Output do `query --kinds` é NDJSON stream (um objeto por kind), não array JSON único (GAP-138)
- `case` requer `--subvert OLD NEW` — scanning global não implementado (GAP-139)
- `write` com stdin vazio requer flag `--allow-empty-stdin` (GAP-140)
- Flag do `prune-backups` é `--max-age-secs` (não `--max-age`) para explicitar unidade (GAP-143)

### Ação de Migração
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.26"`
- Se parseia exit codes de `get` para chave ausente: exit agora é 4 (era 65 na v0.1.25, era 4 na v0.1.24)
- Se verifica exit codes de `replace`/`transform`/`scope`: zero matches agora retorna exit 1 (era exit 0)
- Se você passa o hash esperado ao `verify` por uma flag: passe-o como argumento posicional, `verify <PATH> <HASH>`
- Se usa `prune-backups --max-age`: mude para `--max-age-secs`
- MSRV inalterado em Rust 1.88

### Cobertura de Testes
- 631+ testes passando
- 13 gaps resolvidos (9 CODE + 4 BY-DESIGN) em 5 rodadas de auditoria
- Veja `gaps.md` daquela versão para a auditoria completa


## v0.1.24 para v0.1.25 (2026-06-22)

### Aditivo (Não-Breaking)
- Arquivo de configuração `.atomwrite.toml` com hierarquia: CLI > env > local > XDG global > defaults
- Novo subcomando `verify` (delega para `hash --verify`)
- Estratégia fuzzy Jaro-Winkler para strings curtas no edit
- `edit --fuzzy-threshold <FLOAT>` para sensibilidade configurável
- Ações `symbols` e `normalize` (NFC) no `scope`
- `delete --older-than` com duração legível por humanos
- `delete --confirm` como modo preview (substituído na v0.1.35: use `--plan`; `--confirm` rejeitado)
- `replace --preserve-case` com adaptação de case
- Flag `search --pcre2` (retorna exit 65 quando feature não habilitada)
- `transform --verify-parse` re-valida output via tree-sitter
- `scope --query comments` agora captura block comments em Rust
- `copy --preserve` agora copia permissões e timestamps da origem
- `outline --positions` agora emite byte offsets e posições de coluna
- Testes property-based de fuzzy via proptest

### Correções (Podem Afetar Comportamento)
- `write --backup` não reporta mais `backup_path` fantasma (correção crítica de integridade)
- `set` rejeita descida em valores escalares TOML (antes redirecionava silenciosamente)
- `case` sem `--subvert` agora sai com exit 65 (antes era no-op silencioso)
- Erros de I/O via anyhow agora emitem envelopes NDJSON (7 subcomandos afetados)
- Campo de output do `hash` renomeado de `value` para `checksum`
- `batch` move/copy agora exige `"force":true` para sobrescrever alvos existentes
- `get`/`del` em chave ausente retorna INVALID_INPUT (exit 65) em vez de FILE_NOT_FOUND (exit 4)
- Campo `modified` de `list --long` agora emite formato ISO 8601
- `size_delta_pct` em risk_assessment alterado de u8 para u32
- Default de avaliação de risco alterado para desabilitado (255)

### Ação de Migração
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.25"`
- Se você parseia output do `hash`: campo `value` agora é `checksum`
- Se usa `case` sem `--subvert`: agora sai com exit 65 (adicione pares `--subvert` explicitamente)
- Se parseia exit codes de `get`/`del` para chave ausente: exit mudou de 4 para 65
- Se parseia `batch` move/copy: alvos existentes agora exigem `"force":true`
- MSRV inalterado em Rust 1.88

### Cobertura de Testes
- 631 testes passando (621 na v0.1.24 + 10 novos na v0.1.25)
- ~505 cenários e2e em 6 rodadas de auditoria
- Veja `gaps.md` daquela versão para a auditoria completa de 64 gaps


## v0.1.23 para v0.1.24 (2026-06-21)

### Mudanças de Comportamento (Breaking)
- Formato de timestamp de backup: `YYYYMMDD_HHMMSS` alterado para `YYYYMMDD_HHMMSS_mmm` (sufixo de milissegundos)
- `rollback --timestamp` agora aceita match por PREFIXO (ex: `--timestamp 20260621_120000` casa com `file.bak.20260621_120000_042`)
- `prune-backups --max-count` agora ordena por NOME de arquivo (lexicográfico) em vez de mtime
- `replace` rejeita padrão vazio com exit 65 (antes destruía arquivos silenciosamente)
- `get` de chave inexistente retorna exit 4 (era exit 0 com `found: false`)
- `hash` de arquivo inexistente retorna exit 4 (era exit 0 silenciosamente)
- `regex` removeu `allow_hyphen_values` no campo examples; usar POSIX `--` para exemplos com hífen

### Tratamento de Erros (Breaking para parsers)
- 20 chamadas `anyhow::bail!()` convertidas para variantes tipadas `AtomwriteError`
- TODOS os erros voltados ao usuário agora emitem JSON estruturado no stdout com exit codes corretos
- Exit codes alterados: muitos caminhos que retornavam exit 1 agora retornam exit 4 (NotFound) ou exit 65 (InvalidInput)
- Comandos afetados: `set`, `del`, `get`, `query`, `outline`, `edit`, `batch`, `write`, `prune-backups`, `extract`

### Correções (Podem Afetar Comportamento Existente)
- `delete --recursive` AGORA FUNCIONA (era no-op para diretórios)
- `hash --recursive` AGORA FUNCIONA (era aceito mas nunca caminhava)
- `search --multiline` AGORA FUNCIONA (não era propagado ao SearcherBuilder)
- `scope --query comments --delete` agora remove nós line_comment inteiros (antes só removia `//`)
- `scope`/`count`/`transform` agora resolvem walk roots contra `--workspace` (usavam CWD)
- `diff` agora resolve paths contra `--workspace`
- `batch --transaction` agora reverte operações `move`/`copy` no rollback
- `case --subvert` alterado de greedy `num_args=2..` para exato `num_args=2`
- Valores JSON/TOML de `get` não são mais duplamente quotados
- `old_value`/`removed_value` de `set`/`del` não são mais duplamente quotados

### Aditivo (Não-Breaking)
- `write` pula risk_assessment para `--append`/`--prepend`
- `scope` summary emite `files_modified: null` em modo read-only
- Saída NDJSON de `wal-stats`/`wal-heal` agora inclui campo `type`
- `hash --stdin` não exige mais argumento PATHS
- Campo `op` em `edit --multi` agora é opcional (inferido como `"exact"`)
- `read --format raw` pula heurística binária
- `read --line/--lines` não entra mais em panic com índices fora do range
- `read --lines/--head/--tail` range vazio retorna conteúdo vazio em vez de `"\n"`
- `set` TOML chaves aninhadas agora retornam `old_value` corretamente
- `regex` emite warning quando exemplos parecem flags
- `--require-backup` agora detecta override via `--no-backup`
- ARGUMENT_PARSE_ERROR (exit 2) ganha campo `suggestion` acionável

### ADRs Adicionados
- ADR-0045: Suggestion acionável para erros de parsing do clap
- ADR-0046: Retrofit diff resolve-first
- ADR-0047: Correção do modo read-only do scope

### Ação de Migração
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.24"`
- Se você parseia exit codes: atualize handlers para caminhos que mudaram de exit 1 para exit 4 ou exit 65
- Se usa `rollback --timestamp`: formato antigo ainda funciona (prefix match é retrocompatível)
- Se usa `prune-backups --max-count`: resultados podem diferir (lexicográfico vs mtime)
- Se faz match de nomes de backup com regex: atualize para aceitar sufixo opcional `_\d{3}`
- Se passa padrão vazio para `replace`: agora é rejeitado (era bug de destruição silenciosa)
- MSRV inalterado em Rust 1.88

### Cobertura de Testes
- 621 testes passando (609 na v0.1.23 + 12 novos na v0.1.24)
- 3 ADRs adicionados (0045-0047)
- Veja `gaps.md` daquela versão para a auditoria completa de 52 issues resolvidos


## v0.1.22 para v0.1.23

### Mudanças Comportamentais (Ação Necessária)
- backup-by-default: todos os 9 comandos que mutam conteúdo (`write`, `edit`, `edit-loop`, `replace`, `transform`, `apply`, `set`, `del`, `case`) agora criam backup ANTES de escrever por padrão. O backup é auto-deletado após sucesso (default existente `keep_backup: false` inalterado). Se seu pipeline depende de NENHUM arquivo de backup ser criado (ex.: verifica ausência de arquivos `.bak.*`), adicione `--no-backup` ao comando
- guarda de shrink: `write --expect-checksum` agora BLOQUEIA writes que reduzem o arquivo em mais de 50%. Se seu pipeline legitimamente trunca arquivos usando `--expect-checksum`, adicione `--allow-shrink` ao comando. Sem `--expect-checksum`, o comportamento é inalterado

### Mudanças Aditivas (Sem Ação Necessária)
- `allow_hyphen_values`: 12 campos CLI em 8 structs agora aceitam valores iniciando com `-`, e os três posicionais (padrão do `search`, padrão e substituto do `replace`) exigem `--` desde a 0.1.37. Anteriormente causavam exit 2 (ARGUMENT_PARSE_ERROR). Nenhuma migração necessária — isso corrige um bug
- `edit --old-file <PATH> --new-file <PATH>`: novas flags que leem conteúdo de match/substituição de arquivos em vez de argumentos CLI. Contorna o ARG_MAX do kernel (~131 KB). Cross-mixing de `--old` com `--new-file` (ou vice-versa) retorna exit 65. Nenhuma migração necessária — são flags novas opt-in

### Checklist de Migração
- Se usa `write` sem `--backup`: nenhuma ação necessária (backup auto-deleta após sucesso)
- Se verifica ausência de arquivos `.bak.*` em scripts locais: adicione `--no-backup`
- Se usa `write --expect-checksum` para truncar arquivos legitimamente: adicione `--allow-shrink`
- Se passa valores iniciando com `-` para `edit --old`, `search`, `replace`, `calc`, `regex`, `transform`, `read --grep`, `query --query`: a correção é automática, nenhuma migração necessária


## v0.1.22 — Novidades

Esta release adiciona 2 novos sub-comandos para fechar o último GAP-2026-012 e dar aos operadores ferramentas para gerenciar backups legados e o padrão de N edits em 1 invocação. Os dois são aditivos: nenhuma flag, schema ou comportamento padrão mudou nos comandos existentes.

### Sub-comandos Adicionados
- `prune-backups [PATHS]...` — limpeza manual de backups `.bak.YYYYMMDD_HHMMSS` legados
  - Flags: `--max-age-secs <SECONDS>` (remove backups mais antigos que N segundos), `--max-count <N>` (mantém os N mais recentes), `--dry-run` (prévia, sem remover nada)
  - Saída NDJSON com uma linha por backup e um resumo
  - Veja `docs/decisions/0040-prune-backups-subcommand.md`
- `edit-loop <PATH>` — aplica N pares `{old, new}` vindos de NDJSON no stdin em 1 invocação
  - Flags: `--workspace`, `--partial`, `--allow-sequential-drift`, `--syntax-check`, `--line-ending`, `--backup`, `--keep-backup`, `--retention`; ele não aceita `--expect-checksum`, `--fuzzy` nem `--preserve-timestamps`
  - Saída NDJSON: um evento `result` com `pairs_total`, `pairs_applied`, `pairs_unmatched` e `pair_results[]`
  - Desde a v0.1.37 um par sem match sai 65 `MATCH_FAILED` e nada é gravado; `--partial` grava os pares que casam e sai 87
  - Veja `docs/decisions/0039-edit-loop-helper.md`

### Validação
- Histórico (na v0.1.22): 575+ testes passando, 2 novos ADRs (0039, 0040)
- 2 novos schemas NDJSON: `edit-loop-output.schema.json`, `prune-backups-output.schema.json`
- O cross-compile dos 3 targets Windows foi reportado então e não foi executado de novo na v0.1.37


## v0.1.21 — Novidades

Esta release fecha 3 GAP-2026 items (012, 013 Problema C, 014 v2) e adiciona 1 ADR (0038 backup cumprido deleta). A mudança mais visível é que operações `--backup` agora DELETAM o backup após sucesso por padrão; adicione `--keep-backup` para preservá-lo. A segunda mudança visível é que `edit` e `rollback` agora aceitam `--backup`, fechando o buraco de paridade de API da v0.1.20. A terceira mudança é `--allow-sequential-drift` no `edit` para pipelines sequenciais.

### Operações de Backup
- `write --backup` e `replace --backup` DELETAM o backup após sucesso por padrão (na v0.1.21)
- `edit --backup` e `rollback --backup` são NOVIDADES em v0.1.21; a flag é honrada em todos os 4 subcomandos mutantes
- `--keep-backup` é a flag OPT-IN para preservar o backup após sucesso em `write`, `edit`, `replace`, `rollback`, `apply` e `batch`
- `apply --keep-backup` e `batch --keep-backup` são NOVIDADES em v0.1.21 para paridade
- Backups são SEMPRE preservados no caminho de FALHA, independentemente de `--keep-backup`

### Padrão de Edits Sequenciais
- Encadear múltiplas chamadas `edit` no mesmo arquivo sem re-capturar `checksum_after` produz `STATE_DRIFT` (exit 82) em toda chamada após a primeira
- Dois padrões válidos: re-capturar checksum (Padrão A) ou passar `--allow-sequential-drift` (Padrão B)
- Comportamento padrão inalterado: `STATE_DRIFT` ainda dispara em mismatch de checksum quando a flag está ausente

### Exemplo — Padrão A

```bash
# Checksum inicial
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')

# Edit 1 — passa o checksum capturado
printf 'linha 2\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --after-line 1

# Re-captura o checksum pós-edição
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')

# Edit 2 — usa o novo checksum
printf 'linha 3\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --after-line 2
```

### Exemplo — Padrão B

```bash
# Edit 1 — checksum inicial
CS=$(atomwrite --workspace . read src/main.rs | jaq -r '.checksum')
printf 'linha 2\n' | atomwrite --workspace . edit --expect-checksum "$CS" src/main.rs --after-line 1

# Edit 2 — drift permitido, o arquivo não casa mais com CS (exit 0 com um aviso no stderr)
printf 'linha 3\n' | atomwrite --workspace . edit --expect-checksum "$CS" --allow-sequential-drift src/main.rs --after-line 2
```

- Os dois padrões foram executados na 0.1.37 com um arquivo que contém `linha 1`; o segundo edit do Padrão B sem `--allow-sequential-drift` sai 82 `STATE_DRIFT`

### Notas de Migração
- Breaking change — `write --backup` e `replace --backup` não deixam mais um sibling `.bak` em disco após uma escrita bem-sucedida. Adicione `--keep-backup` a qualquer script que dependa do backup persistindo através da operação, ou leia o backup antes da escrita completar
- Breaking change — `edit` e `rollback` agora aceitam `--backup` e o ignoram sem reclamação se as pré-condições da camada atômica o rejeitarem. A flag explícita `--backup` é o novo opt-in; scripts antigos que chamam `edit` supondo que não há backup continuam sem backup por padrão
- Não-breaking — `apply --keep-backup` e `batch --keep-backup` são aditivos. O comportamento padrão (sem backup) permanece inalterado

### Validação
- Histórico (na v0.1.21): 555+ testes passando (542 baseline v0.1.20 + 13 novos), 1 novo ADR (0038), 3 GAP-2026 fechados (012, 013 Problema C, 014 v2)
- O cross-compile dos 3 targets Windows foi reportado então e não foi executado de novo na v0.1.37


## v0.1.20 — Novidades

Esta release introduz uma nova camada de segurança chamada intention guards e renomeia a flag global `--lang` para `--locale` para desambiguar do seletor tree-sitter `--lang` usado por `scope` e `transform`.

### Intention Guards (5 flags OPT-IN)
- `--require-backup` — exige `--backup`: aborta quando falta e o arquivo alvo existe (não recebe valor)
- `--confirm` — flag legada de consciência no `write`, sem efeito sozinha: sobrescrita grande exige `--ack-overwrite`, e `delete --confirm` é rejeitado (use `--plan`)
- `--auto-rotate` — força um backup de rotação automática para arquivos modificados há pouco (idade de `[write].auto_rotate_max_age_secs`, padrão 24 h; não recebe valor)
- `--risk-threshold <PERCENT>` — limite de variação de tamanho, em percentual, para o aviso de risco (padrão 255 = desligado; por exemplo `--risk-threshold 50`)
- `--locale <en|pt-BR>` — renomeado de `--lang` para desambiguar do `--lang` tree-sitter

### Outras Adições
- `count --by-size` — lista os maiores arquivos da árvore com tamanhos e contagem de linhas
- `read --format raw|ndjson` — seleciona entre saída byte-stream e envelope NDJSON estruturado (uma grafia antiga com `--mode` nunca existiu)
- `search --no-begin-end` — desabilita a decoração implícita de âncoras `^` e `$` na saída regex
- `write --preserve-timestamps` — preserva o mtime do arquivo fonte ao sobrescrever
- `scope --lang rust` — alias explícito de `scope --language`; o `transform` aceita só `-l`/`--language`

### Estatísticas
- 542 testes passando em 47 suites de integração, 0 falhas
- 11 GAP-2026 fechados
- 3 targets de cross-compile Windows verdes
- 19 ADRs em `docs/decisions/` (0019-0037)

### Migração `--lang` para `--locale`

```bash
# Descobrir todos os arquivos com --lang
rg -l -- '--lang\b' .

# Substituir em massa preservando outros matches
fd -e sh -e md -e toml -e yml -e yaml -e json -x sd -- '--lang\b' '--locale' {}

# Ou via ruplacer (só simula até você acrescentar --go)
ruplacer -- '--lang' '--locale' .
```


## v0.1.12 para v0.1.15

### Aditivo (G117)
- O `edit` multi-par `--old`/`--new` roda a cascata fuzzy completa de 9 estratégias por par (antes era somente exato).
- O envelope de sucesso ganha `pairs_total` e `pair_results`; o envelope de erro ganha `failed_pair_index`, `pairs_total`, `pair_results` (continua `INVALID_INPUT`, exit 65, arquivo intacto).
- Novo opt-in `edit --partial` aplica os pares que casam e relata os demais; zero matches mapeia para `NO_MATCHES` (exit 1) sem escrita.

### Correção Comportamental (G118)
- O `write` resolve o alvo via jail do workspace ANTES de append/prepend, detecção automática de line ending e `--expect-checksum`.
- Com CWD fora do workspace: `--append`/`--prepend` não truncam mais, `--line-ending auto` volta a detectar o arquivo existente, e um `--expect-checksum` divergente retorna exit 82 (`STATE_DRIFT`) em vez de sobrescrever silenciosamente. Alvos fora do jail retornam exit 126 cedo.
- Breaking apenas para chamadores que dependiam da sobrescrita silenciosa do bug.

### Ação de Migração (v0.1.15)
- Atualize o pin de versão: `cargo install atomwrite --locked --version "^0.1.15"`
- MSRV inalterada em Rust 1.88. Nenhuma mudança de código necessária para chamadores em conformidade.
- Veja o ADR-0026 e o ADR-0027 em `docs/decisions/` para a justificativa completa.


## O Que Há de Novo na v0.1.12

Esta seção resume as mudanças relevantes para migração em v0.1.12. Veja a seção [v0.1.11 para v0.1.12](#v0111-para-v0112) abaixo, a seção [v0.1.32 para v0.1.33](#v0132-para-v0133) para o break de runtime one-shot, e a seção [v0.1.36 para v0.1.37](#v0136-para-v0137) para a transição mais recente.

### Novos Subcomandos (6)
- `set` — escreve um valor em um caminho dotted em TOML/JSON. Use em vez de reescrever o arquivo inteiro.
- `get` — lê um valor em um caminho dotted. Use em vez de ler o arquivo inteiro.
- `del` — remove uma chave em um caminho dotted. `--force-missing` para scripts idempotentes.
- `case` — renomeia identificadores em 5 estilos de case via `heck`.
- `query` — caminha um AST tree-sitter. 305 linguagens na v0.1.12, e a árvore hoje fixa o pack 1.22.1 com 372.
- `outline` — extrai definições top-level. 305 linguagens na v0.1.12, e a árvore hoje fixa o pack 1.22.1 com 372.

Todos os 6 subcomandos são aditivos. Nenhum código existente é afetado.

### Novas Flags (15 Total)
- `write --syntax-check` (G72)
- `search --include-fifo` (G56): FIFOs são pulados por padrão e a flag os abre de novo; `write` num FIFO sai 85 `FIFO_DETECTED`
- Fallback EXDEV (G90): `--strict-atomic` num verbo que escreve liga `WriteOptions.strict_atomic`, então a linha de comando alcança o exit 91 `EXDEV_FALLBACK_DISABLED` quando o alvo mora em outro sistema de arquivos
- Lock advisory de escrita (G54), automático desde a 0.1.37: não existe flag de lock nem flag de espera de lock
- `read --format raw` (G81); não existe alias `--raw`
- `read --head N`, `--tail N`, `--line N`, `--grep <REGEX>`
- `search --max-filesize` e `--max-columns` (G68)
- `replace --literal` e `-F` (G66)
- `transform --rules` e `--inline-rules` (G44)
- `batch --batch-size` (G77)
- `backup/copy --no-reflink` (G64)
- `diff --algorithm patience|myers|lcs` (G76)

Todas as flags são aditivas com valores padrão que preservam o comportamento de v0.1.11.

### Novos Códigos de Erro (5)
- 83 `LockTimeout` (G54)
- 88 `SyntaxError` (G72)
- 91 `ExdevFallbackDisabled` (G90)
- 92 `CopyBackBlake3Failed` (G114)
- 93 `OrphanJournal` (G114)

Total de códigos de erro: 25 (eram 20 em v0.1.4). Todos os novos códigos têm mensagens bilíngues e sugestões acionáveis.

### Dependências Adicionadas
- `tree-sitter-language-pack = "1.8"` — 305 linguagens, dynamic loading, naquela versão; a árvore hoje fixa a 1.22.1 com 372
- `toml_edit` — preserva formatação TOML
- `heck = "0.5"` — conversão de case
- `reflink-copy = "0.1"` — backup CoW
- `content_inspector = "0.2"` — detecção UTF-16
- `xattr = "1"` — extended attributes

Todas aditivas. Nenhuma dependência existente removida.

### Mudanças de Comportamento
- Nenhuma. v0.1.12 é totalmente retrocompatível com v0.1.11.
- Novos subcomandos são opt-in: scripts existentes continuam funcionando.
- Valores padrão das novas flags preservam o comportamento de v0.1.11.
- Adições de código de erro não mudam exit codes existentes.

### Ação de Migração
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.12"`
- Novos subcomandos e flags são opt-in. Nenhuma mudança de código necessária para chamadores existentes.
- Veja a seção [trabalho dobrado na v0.1.38](#trabalho-dobrado-na-v0138-leituras-mais-baratas-e-saídas-honestas-atual) para a transição mais recente; a seção [v0.1.32 para v0.1.33](#v0132-para-v0133) cobre o break de runtime one-shot.

### Cobertura de Testes
- 542 testes passando (445 na v0.1.12 + 2 na v0.1.14 + 8 G117 + 6 G118 na v0.1.15 + 40 v0.1.16-v0.1.18 cross-platform + 21 v0.1.19 + 20 v0.1.20)
- 9 ADRs em `docs/decisions/` (0019-0027)
- 7 novos JSON schemas em `docs/schemas/`
- Veja [docs/decisions/README.md](decisions/README.md) para decisões arquiteturais


## v0.1.11 para v0.1.12
### v0.1.12

A release v0.1.12 fecha 13 dos Top 20 gaps da auditoria PRD v5-v16 (`gaps.md` daquela versão). É aditiva: todo comportamento de v0.1.11 é preservado.

### Adicionado (Novos Subcomandos -- v14 Tier 3)
- `set <PATH> <KEY_PATH> <VALUE>` -- escreve um valor em um caminho dotted em arquivo TOML ou JSON, preservando comentários e ordem das chaves via `toml_edit`.
- `get <PATH> <KEY_PATH>` -- lê um valor em um caminho dotted. NDJSON: `{"type":"get","key_path","value","found","format"}`.
- `del <PATH> <KEY_PATH>` -- remove uma chave. Flag `--force-missing` trata chaves ausentes como no-op success.
- `case <PATHS...> --subvert OLD NEW --to <style>` -- renomeia identificadores via `heck`.
- `query <PATH> [--kinds|--query <KIND>|-Q <KIND>|--tree] [--positions]` -- caminha um AST tree-sitter.
- `outline <PATH> [--kind <KIND>] [--positions]` -- extrai estrutura de alto nível.

### Adicionado (G72 verificação de sintaxe REAL)
- `atomwrite write --syntax-check` invoca tree-sitter (24 linguagens). Exit 88.

### Adicionado (G114 sidecar WAL)
- `.atomwrite.journal.<target>.atomwrite.journal.json` com `Started`/`Committed`.
- `recover_orphan_journals(dir)` é consultivo.

### Adicionado (Outros Gaps)
- G54 flock advisory -- exit 83. Automático desde a 0.1.37 (espera de 5 s pelo sidecar oculto `.<alvo>.atomwrite.lock`), não existe flag de lock nem flag de espera de lock.
- G39 xattr -- macOS quarantine, Linux SELinux, capabilities POSIX preservados.
- G41 content_inspector -- UTF-8, UTF-16LE, UTF-16BE, Binário corretamente detectados.
- G64 reflink CoW -- `reflink_or_copy` em APFS/btrfs/XFS.
- G90 fallback EXDEV -- copy fallback para Docker/NFS. `--strict-atomic` num verbo que escreve liga `WriteOptions.strict_atomic` e recusa o fallback com exit 91.
- G44 transform multi-rule -- `--rules <file.yaml>` e `--inline-rules <YAML>`.
- G68 `--max-filesize` e `--max-columns` -- search skip/truncate.
- G80 SIGPIPE -- broken pipe → exit 0 (histórico na v0.1.12; desde a v0.1.37 pipe fechado sai 141).
- G81 `--format raw` -- read emite bytes crus para composabilidade Unix (não existe alias `--raw`).

### Adicionado (5 Novos Códigos de Erro)
- 83 `LockTimeout`
- 88 `SyntaxError`
- 91 `ExdevFallbackDisabled`
- 92 `CopyBackBlake3Failed`
- 93 `OrphanJournal`

### Ação de Migração
- Nenhuma mudança de código necessária
- Novos subcomandos são opt-in
- Atualizar pin de versão: `cargo install atomwrite --locked --version "^0.1.12"`


## v0.1.3 para v0.1.4 (Histórico)

### v0.1.4 (Histórico)

### Corrigido (Compilação Windows - GAP 14)

Três erros de compilação em blocos `#[cfg(windows)]` impediam `cargo install atomwrite` de funcionar no Windows 10/11 desde v0.1.3:

- `E0433` em `src/atomic.rs:404` — `persist_with_retry` usava `AtomwriteError::PermissionDenied` sem importá-lo. O `use crate::error::AtomwriteError;` agora está gated sob `#[cfg(windows)]` para evitar `unused_imports` em Linux/macOS.
- `E0507` em `src/atomic.rs:387` — `persist_with_retry` recebia `&NamedTempFile` mas chamava `temp.persist()` que requer ownership. Assinatura mudou para `fn persist_with_retry(mut temp: NamedTempFile, target: &Path) -> Result<()>`. O branch de retry agora recupera o arquivo de `e.file` (PersistError expõe o NamedTempFile original em falha).
- `E0308` em `src/platform.rs:116` — `GetStdHandle` retorna `HANDLE` que é `*mut c_void` no windows-sys 0.61. O literal `0` é `usize`; comparar raw pointer com inteiro é erro de tipo. Substituído `handle != 0` por `!handle.is_null()`. A comparação `handle != INVALID_HANDLE_VALUE` permanece inalterada porque `INVALID_HANDLE_VALUE` já é tipada como `HANDLE` (`-1i32 as _`).

Impacto da migração:
- Nenhuma mudança de API ou comportamento para usuários Linux ou macOS
- Usuários Windows: `cargo install atomwrite` agora funciona; sem necessidade de patches manuais ou compilar do código fonte
- Toda a semântica de escrita atômica, exit codes, output NDJSON, e flags CLI permanecem inalteradas

### Corrigido (Sugestões de Erro - GAP 13)

Sugestões de erro agora são context-aware e acionáveis:

- Sugestão de `WorkspaceJail` se adapta: quando o usuário forneceu `--workspace`, a sugestão agora diz "use a path inside the workspace (<root>)" em vez de re-pedir a flag.
- Todas as 20 variants de erro agora carregam texto `suggestion`. Anteriormente 6 variants (InvalidInput, Io, ConfigInvalid, FileImmutable, NoMatches, InternalError) retornavam `None`. Apenas `BrokenPipe` (SIGPIPE, não-acionável) permanece sem sugestão.
- Referência phantom à flag `--force-text` removida da sugestão de BinaryFile.
- Novo struct `ErrorContext` (`workspace_provided`, `workspace`) e API `ErrorJson::from_error_with_context()`. A versão legacy `from_error()` é preservada.

Novas sugestões:
- `FileImmutable` — menciona `chattr -i` (Unix) e `fsutil` (Windows) para limpar o atributo imutável
- `NoMatches` — orienta o usuário a ampliar o padrão e revisar filtros `--include`/`--exclude`
- `InternalError` — solicita report de bug com o contexto da razão
- `InvalidInput` — pede ao usuário para revisar o input e checar argumentos
- `Io` — aponta para a mensagem de erro de I/O subjacente
- `ConfigInvalid` — aponta para a razão da configuração

Impacto da migração:
- Sem quebra de API: `ErrorJson::from_error()` ainda funciona com o mesmo output
- Se você parseia o campo `suggestion` de envelopes de erro, o texto pode diferir para as variants afetadas. A semântica (dica acionável) é preservada ou melhorada.

### Adicionado (Validação Cross-Platform - GAP 14)
- `tests/cross_compile_check.rs` — 3 testes de cross-compile gated para `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, e `x86_64-pc-windows-msvc`. Falha em qualquer regressão de `E0433`, `E0308`, ou `E0507` em blocos `cfg(windows)`. Execute com `cargo test --test cross_compile_check -- --ignored` antes de releases que tocam código Windows-only.
- `output::write_error_json_with_context()` — propaga `ErrorContext` do parser CLI até o output NDJSON.
- `docs/INSTALL.md` e `docs/INSTALL.pt-BR.md` — pré-requisitos de instalação Windows 10/11, comandos `cargo install`, e troubleshooting.

### Referência

Veja as seções "GAP 13" e "GAP 14" em `gaps.md` daquela versão para a análise completa de causa raiz e rationale de design.


## v0.1.2 para v0.1.3

### v0.1.3 (Anterior)

### Mudado (BREAKING)

### Escrita atômica padrão não preserva mais o mtime

Os subcomandos `edit` e `replace` agora atualizam o tempo de modificação do arquivo (mtime) para o momento em que a escrita é concluída, em vez de preservar o mtime original. Este é o padrão correto para sistemas de build que usam mtime para detectar mudanças em código fonte.

Antes da v0.1.3:
- `edit` e `replace` hardcodavam `AtomicWriteOptions::preserve_timestamps = true`
- O mtime do arquivo era restaurado para o valor que tinha ANTES do rename atômico
- Sistemas de build que comparam mtime do fonte com mtime do dep-info (cargo, make, cmake) pulavam o rebuild silenciosamente quando o fonte parecia mais antigo que o binário
- Workaround: agentes tinham que rodar `touch <file>` após `atomwrite edit` para forçar o cargo a detectar a mudança

Depois da v0.1.3:
- `edit` e `replace` usam `AtomicWriteOptions::preserve_timestamps = false` por padrão
- O mtime é definido para "agora" automaticamente, então o cargo detecta a mudança sem intervenção manual
- Agentes não precisam mais de `touch` após editar um arquivo fonte Rust antes de `cargo build`
- Para workflows de backup, snapshot ou builds reproduzíveis que precisam do timestamp original, passe a nova flag `--preserve-timestamps`

Consumidores afetados (agentes LLM):
- Se você constrói código após editar com atomwrite, o novo padrão corrige um no-op silencioso "Finished in 0.29s" onde o cargo pula o rebuild
- Se você depende do comportamento antigo de preservação de mtime, adicione `--preserve-timestamps` às suas invocações de `edit` e `replace`

Campo de diagnóstico:
- A saída NDJSON de `edit` e `replace` agora inclui `mtime_preserved: bool` para que você verifique qual caminho foi tomado (true = timestamp mantido, false = timestamp atualizado)

### Adicionado (Consciência de Sistema de Build)
- Flag `--preserve-timestamps` em `edit` e `replace` para voltar ao comportamento de preservação de mtime da v0.1.2
- Campo `mtime_preserved` nas respostas NDJSON de `EditOutput` e `ReplaceResult`

### Referência

Veja `gaps.md` daquela versão, seção "Atomic Edit Preserva mtime E Quebra Detecção De Mudança Pelo Cargo" (GAP 12) para a análise completa de causa raiz e justificativa de design.


## v0.1.1 para v0.1.2

### v0.1.2

### Correções (Bug Fixes)

### `batch --transaction` rollback agora é real
Anteriormente, arquivos criados por operações `write` durante uma transação nunca eram removidos no rollback. Agora:
- `RollbackEvent` inclui `files_restored`, `files_removed` e `total_reverted`
- Novos arquivos criados no meio da transação são deletados no rollback
- Arquivos pré-existentes modificados são restaurados do backup

Consumidores afetados (agentes LLM): confiem no evento NDJSON `rollback` — o estado do disco corresponde a ele.

### `replace` não infla mais contadores em violações de jail
Anteriormente, `total_replacements` era incrementado para arquivos fora do jail do workspace. Agora:
- Validação do jail roda ANTES do incremento do contador
- Violações emitem `ReplaceErrorEvent` com `kind: jail_violation`, `error_class: permanent`, `retryable: false`
- `total_replacements` reflete apenas matches dentro do jail

### Eventos paralelos do `search` agora são agrupados por path
O walker paralelo não intercala mais eventos `begin`/`match`/`end` de arquivos diferentes. Sequências de eventos para um dado path agora são contíguas na saída NDJSON.

### `scope --delete` em comentários Rust não deixa mais espaço em branco órfão
A query preparada `comments` para Rust agora casa whitespace trailing, então a deleção produz código limpo.

### `search` com regex inválido emite envelope JSON estruturado
Padrões inválidos agora falham com `AtomwriteError::InvalidInput` que é serializado como `error.json` no stdout, não stderr cru.

### `batch --file <PATH>` agora é funcional
A flag agora realmente lê o manifesto NDJSON de um arquivo (validado contra jail do workspace) em vez de ser ignorada.

### `backup --output-dir <DIR>` agora é respeitado
A flag agora coloca backups no diretório customizado (criado se faltar) e poda backups antigos naquele diretório.

### Mensagem de erro de WORKSPACE_JAIL corrigida
A sugestão enganosa "use an absolute path" agora é "set --workspace <root> (XDG/config only; no environment variables)".

### Adicionado (Funcionalidades Agent-First)
- Flag global `--timeout <SECONDS>` para execução com limite de tempo (na introdução: 0 = sem timeout, padrão 0; desde v0.1.33 o padrão é 120, exit 124 no prazo)
- `read --grep <REGEX>` filtra para retornar apenas linhas que casam com regex
- `completions --install` para instalar scripts de completions no diretório de dados XDG

### Mudado (Dependências)
- `nix` 0.29 → 0.31
- `signal-hook` 0.3 → 0.4
- `windows-sys` 0.59 → 0.61
- `rust-i18n` 3 → 4

### Cross-Platform

atomwrite v0.1.2 agora compila no macOS arm64 (Apple Silicon) e macOS x86_64. A syscall `posix_fadvise` agora está corretamente restrita a `target_os = "linux"` apenas.


## v0.1.0 para v0.1.1
### Resumo
- ZERO mudanças que quebram compatibilidade
- Todos os comandos, flags e saída JSON da v0.1.0 permanecem inalterados
- Nenhuma ação de migração necessária para consumidores existentes

### Comportamentos Corrigidos (falhas silenciosas corrigidas)
- `search --include` e `search --exclude` agora realmente filtram arquivos (eram silenciosamente ignorados)
- `replace --include` e `replace --exclude` agora realmente filtram arquivos
- `transform --include` e `transform --exclude` agora realmente filtram arquivos
- `search --context N` agora emite linhas de contexto ao redor dos matches
- `search --max-count N` agora limita matches por arquivo
- `search --invert` agora mostra linhas sem correspondência (estava invertido)
- `search --sort path` agora ordena resultados por caminho de arquivo
- `transform` agora processa arquivos em paralelo (era sequencial)
- Timestamp `modified` de `read` agora retorna ISO 8601 em vez de epoch seconds
- Backup de `batch delete` agora usa create_backup() atômico com fsync (estava competindo com a escrita)
- `create_backup` agora usa `fs::copy` em vez de `fs::hard_link` (hard links divergiriam silenciosamente)
- 12 links intra-doc quebrados em `error.rs` corrigidos
- Números mágicos de exit code substituídos por constantes nomeadas em `constants.rs`
- Seis chamadas `unwrap()` em `edit.rs` modo multi-edit substituídas por `ok_or_else`
- `scope.rs` thread join não causa mais panic em falha

### Mudanças Aditivas
### Novos Subcomandos
- Subcomando `scope` para escopo gramatical com ações baseadas em AST (delete, upper, lower, titlecase, squeeze, replace)
- `scope` suporta Rust (30 queries preparadas), Python (13), JavaScript/TypeScript (11), Go (8)
- Subcomando `backup` para backups com timestamp e checksums BLAKE3 e retenção configurável
- Subcomando `rollback` para restauração a partir de backups com verificação BLAKE3 opcional
- Subcomando `apply` para aplicação de patches com detecção automática de formato (unified diff, SEARCH/REPLACE, markdown-fenced, full file)

### Novas Flags
- Flag `batch --transaction` para execução tudo-ou-nada com rollback
- Flag `edit --fuzzy` com cascata de 7 estratégias para matching aproximado
- Flag `edit --multi` para múltiplas edições NDJSON em uma escrita atômica
- Flag `--line-ending lf|crlf|cr|auto` em `write` e `edit`
- Flag global `--lang <LOCALE>` para override de locale (en, pt-BR)
- `batch` move e copy aceitam `source`, `from`, `src` como aliases de campo
- `batch` write, delete, edit, hash aceitam `path` como alias de `target`

### Internacionalização
- Suporte a i18n via `rust-i18n` com detecção automática de locale do SO
- Todas as strings voltadas ao usuário agora cientes de locale (erros, avisos, mensagens informativas)
- Documentação bilíngue (en + pt-BR) para todos os documentos principais

### Segurança
- Detecção de FIFO e arquivos de dispositivo na validação de caminho (códigos de saída 85 e 86)
- Detecção de hardlink antes do rename atômico com `tracing::warn` quando nlink > 1
- Detecção de mesmo arquivo em `copy` e `move` para prevenir perda de dados quando origem=destino
- Headers de licença SPDX em todos os 64 arquivos `.rs` fonte
- `deny.toml` para auditoria de licenças e advisories via cargo-deny

### Infraestrutura de Testes
- 282 testes (eram 5 na v0.1.0)
- Testes de integração para `backup`, `rollback`, `apply` e `scope`
- 2 alvos de fuzzing (`batch_parse`, `extract_json`) com `libfuzzer-sys`
- Testes de integração de locking otimista
- Testes de validação NDJSON expandidos de 5 para 20 de 21 comandos
- Testes de interoperabilidade `jaq` validando NDJSON via filtro
- Teste de integração i18n

### Mudanças na Saída JSON
- Saída de `edit` inclui novos campos opcionais: `fuzzy`, `strategy`, `strategies_tried`, `similarity` (apenas quando correspondência fuzzy é usada)
- Timestamp de `read` mudou de epoch seconds para formato ISO 8601 (quebra para consumidores lendo `modified` como número)
- Novos tipos de saída adicionados para `scope`, `backup`, `rollback`, `apply`
- Todos os campos existentes permanecem inalterados

### Exemplo de Mudança de JSON Schema

```json
// Antes (v0.1.0)
{"type":"read","path":"/abs/file","content":"...","modified":1704067200}

// Depois (v0.1.1)
{"type":"read","path":"/abs/file","content":"...","modified":"2024-01-01T00:00:00Z"}
```

### Limitações Conhecidas Corrigidas em v0.1.2
- Flag `batch --file <PATH>` era declarada mas não era conectada (agora lê manifesto de arquivo)
- `batch --transaction` não deletava arquivos criados no meio da transação
- `replace` inflava contadores em violações de jail
- Walker paralelo do `search` intercalava eventos de arquivos diferentes
- `search` com regex inválido produzia erro cru no stderr em vez de envelope JSON
- `scope --delete` em comentários Rust deixava whitespace órfão
- Compilação no macOS falhava (nix 0.29 restringia `posix_fadvise` a Unix não-macOS)
- `backup --output-dir` era declarado mas não era conectado
- Sem flags `--timeout`, `--grep`, `completions --install`

### Ação de Migração
- Nenhuma ação necessária para v0.1.0 a v0.1.1
- Filtros `jaq` e código de parsing JSON existentes continuam funcionando para todos os campos exceto `read.modified` (epoch → ISO 8601)
- Atualize consumidores que leem `read.modified` como valor numérico
- Novos campos são aditivos e seguros para ignorar
- Recomendado: atualize para v0.1.2 em seguida, que corrige 14 issues introduzidas na v0.1.1


## Notas de Compatibilidade
### v0.1.15
- G117: o `edit` multi-par ganha paridade fuzzy, `pair_results`, `failed_pair_index` e o opt-in `--partial` -- os campos de envelope são aditivos
- G118: o `write` resolve o alvo contra o workspace antes de append/prepend, detecção automática de line ending e `--expect-checksum` -- exits 82/126 agora disparam onde havia sobrescrita silenciosa
- Nenhum código de erro novo; MSRV permanece em Rust 1.88
### v0.1.12
- 6 novos subcomandos: `set`, `get`, `del`, `case`, `query`, `outline` (v14 Tier 3)
- G72 verificação de sintaxe REAL via tree-sitter (`atomwrite write --syntax-check`, exit 88)
- G114 sidecar WAL para recuperação de crash (`.atomwrite.journal.<target>.atomwrite.journal.json`)
- G54 lock advisory via `flock` (exit 83 em timeout), automático desde a 0.1.37
- G39 preservação de xattr (quarantine macOS, SELinux Linux, capabilities POSIX)
- G41 content_inspector para detecção UTF-16/BOM/binário
- G64 reflink CoW para backup/copy em APFS/btrfs/XFS
- G90 fallback EXDEV para Docker/NFS (o exit 91 vem de `--strict-atomic` no verbo que escreve, que liga `strict_atomic`)
- G44 transform multi-rule YAML (`--rules` / `--inline-rules`)
- G68 `--max-filesize` e `--max-columns` para search
- G80 tratamento de SIGPIPE (broken pipe → exit 0)
- G81 `--format raw` para read (composabilidade Unix)
- 5 novos códigos de erro: 83, 88, 91, 92, 93
- Todo comportamento de v0.1.11 preservado
- 445 testes (era 320 baseline, +125 novos)

### v0.1.3 (Histórico)
- BREAKING: `edit` e `replace` não preservam mais o mtime original do arquivo por padrão
- Nova flag `--preserve-timestamps` em `edit` e `replace` restaura o comportamento da v0.1.2
- Novo campo `mtime_preserved` nas respostas NDJSON de `EditOutput` e `ReplaceResult`
- Todo comportamento da v0.1.2 preservado caso contrário (correção de build no macOS, correção de transação do batch, agrupamento de eventos do search, etc)

### v0.1.2 (Anterior)
- Todo comportamento da v0.1.1 preservado
- 6 correções críticas de bugs incluindo build no macOS, transação do batch, contador do replace
- 2 correções de alta prioridade (batch --file, backup --output-dir)
- 3 flags agent-first (--timeout, --grep, completions --install)
- 4 atualizações de dependências (nix 0.31, signal-hook 0.4, windows-sys 0.61, rust-i18n 4)

### v0.1.1
- Todo comportamento da v0.1.0 preservado
- Novos subcomandos e flags são apenas aditivos
- Exit codes inalterados da v0.1.0

### v0.1.0
- Primeira versão pública
- Todos os JSON schemas estão definidos em `docs/schemas/`
- Use `--json-schema` em qualquer subcomando para introspecção em runtime
- Exit codes seguem convenções sysexits
- Releases pré-1.0 não garantem estabilidade de saída
- Releases pós-1.0 manterão compatibilidade retroativa dentro de versões maiores


## Plano de Rollback
- Mantenha o binário da versão anterior disponível antes de atualizar
- Use `cargo install atomwrite@0.x.y` para fixar uma versão específica
- Verifique o rollback executando `atomwrite --version`
- Teste a nova versão em ambiente de staging antes de produção
- Monitore exit codes e saída NDJSON para mudanças inesperadas
- Reverta para a versão anterior se testes do agente falharem
- Reverta a configuração do agente para corresponder a versão anterior da CLI


## O Que Muda
### Compromisso SemVer
- atomwrite segue Semantic Versioning 2.0.0
- Versão MAJOR: mudanças que quebram flags CLI, exit codes ou schema de saída JSON
- Versão MINOR: novos subcomandos, novas flags, novos campos JSON (apenas aditivos)
- Versão PATCH: correções de bug sem mudanças de API

### O Que Conta Como Quebra
- Remover ou renomear uma flag CLI
- Mudar o significado de um exit code
- Remover um campo da saída JSON
- Mudar o tipo de um campo JSON existente
- Renomear um campo JSON
- Mudar o comportamento padrão de uma flag existente

### O Que NÃO Quebra
- Adicionar um novo subcomando
- Adicionar uma nova flag opcional
- Adicionar um novo campo na saída JSON
- Adicionar um novo exit code
- Melhorar mensagens de erro
- Melhorias de performance

### Estabilizações Planejadas para 1.0
- Histórico (naquela release): schemas de saída NDJSON para todos os 41 subcomandos
- Atribuições de exit codes
- Strings de código de erro (`FILE_NOT_FOUND`, `STATE_DRIFT`, etc)
- Nomes e comportamento de flags globais
- Formato do manifesto de batch

### Potenciais Mudanças Quebrando Antes do 1.0
- Nomes de campos na saída NDJSON podem mudar antes do 1.0
- Novos campos obrigatórios podem ser adicionados aos tipos de saída
- Valores de exit codes podem mudar para alinhar com sysexits
- O formato de saída do `--json-schema` pode evoluir


## Template de Migração Passo a Passo
- Use este template ao migrar entre versões

### Passo 1 -- Leia o Changelog
- Revise o `CHANGELOG.md` para a versão alvo
- Identifique todas as entradas marcadas como BREAKING

### Passo 2 -- Verifique Seus Comandos
- Liste toda invocação de atomwrite no seu agente ou scripts
- Compare cada flag contra as notas de migração

### Passo 3 -- Compare JSON Schemas
- Execute `atomwrite <subcommand> --json-schema` com ambas as versões
- Identifique adições, remoções e mudanças de tipo nos campos

### Passo 4 -- Atualize o Parsing de JSON
- Atualize seus filtros `jaq` ou código de parsing JSON
- Trate novos campos com graciosidade (mudanças aditivas)
- Remova referências a campos deletados

### Passo 5 -- Atualize o Tratamento de Exit Codes
- Revise blocos `case` ou `if` que tratam exit codes
- Adicione tratamento para novos exit codes
- Remova tratamento para exit codes depreciados

### Passo 6 -- Teste em Modo Dry-Run
- Execute toda invocação modificada com `--dry-run` primeiro
- Verifique se a estrutura de saída corresponde ao esperado

### Passo 7 -- Deploy
- Atualize o binário via `cargo install atomwrite`
- Execute sua suite de testes
- Verifique o comportamento do agente em ambiente de staging


## Template de Mudanças de JSON Schema
- Use este formato para documentar mudanças de campo entre versões

### Antes (vX.Y.Z)

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc..."}
```

### Depois (vX.Y.Z)

```json
{"type":"write","status":"ok","path":"/abs/path","bytes_written":42,"checksum":"abc...","new_field":"value"}
```

### Ação de Migração
- Novo campo `new_field` é aditivo e OPCIONAL
- Nenhuma ação necessária para consumidores existentes
- Atualize o parsing para consumir o novo campo se útil
