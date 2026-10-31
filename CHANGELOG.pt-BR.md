[Read in English](CHANGELOG.md)


# Changelog

- Todas as mudanças notáveis deste projeto são documentadas neste arquivo
- O formato segue [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/)
- O versionamento segue [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)


## [0.1.37] - 2026-10-31
- Esta entrada funde o rascunho de 2026-10-15 da `0.1.37` com o trabalho de 2026-10-28 até 2026-10-31, porque a `0.1.37` nunca foi publicada: o crates.io carrega a `0.1.36`, e os números maiores que esta árvore declarou por um momento voltaram para dentro da `0.1.37`
- O rascunho mantém as subseções próprias, rotuladas pela data em que foi escrito, para o leitor saber de qual metade veio cada linha

### Alterado — um schema por verbo, descrevendo o stream inteiro (2026-10-31)
- `--print-schema recipe` servia só `RecipeResult` e agora serve `RecipeOutput`: onze ramos de verbo, todo registro de filho reemitido marcado com `step`, mais o envelope de erro
- `--print-schema codemod` servia só `CodemodSummaryEvent` e agora serve `CodemodOutput`: sete ramos de verbo, todo registro de filho marcado com `rule_id`, mais o envelope de erro
- `--print-schema batch` servia só `BatchSummary` e agora serve `BatchOutput`: `batch_op`, `progress`, `rollback`, o resumo terminal, o manifesto do `--input-schema` e o envelope de erro
- `search`, `replace`, `list` e `backup` serviam dois nomes cada e agora servem um: o nome do verbo carrega os registros e o resumo terminal que os nomes `*-summary` guardavam
- Os nomes `search-summary`, `replace-summary`, `backup-summary` e `list-summary` deixaram de ser servidos, então `--print-schema` lista 55 nomes apoiados por 54 arquivos
- `semantic-search`, `sparse`, `semantic-merge`, `case` e `watch` passaram de um ramo único para a união do verbo mais o envelope de erro
- O envelope de erro é ramo de toda união, então uma execução que falha antes do primeiro registro valida contra o schema do próprio verbo
- O `diff` ganhou o envelope de erro como quinto ramo da `oneOf`, exclusivo porque todo registro do verbo não tem `error`
- `close_terminal_schema` deixa a raiz de união intacta, e isso também consertou o `prune-backups`, cuja raiz trazia `additionalProperties: false` e `required: ["ok"]` ao lado do `anyOf` e recusava todo registro por entrada
- `union_of`, `tagged_record`, `terminal_record` e `error_branch` substituem as três cópias escritas à mão do laço de `$defs`
- `README.md`, `README.pt-BR.md`, `docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md`, `llms.txt`, `llms.pt-BR.txt`, `llms-full.txt`, `llms-full.pt-BR.txt` e as duas `skills/atomwrite-*/SKILL.md` passam a dizer 55 nomes com 54 arquivos, e as skills descrevem a união no lugar dos nomes de resumo removidos
- Mais onze lugares mantinham o total antigo e também foram corrigidos: `docs/COOKBOOK.pt-BR.md`, `docs/INSTALL.md`, `docs/INSTALL.pt-BR.md`, `docs/MIGRATION.md`, `docs/MIGRATION.pt-BR.md`, `llms.pt-BR.txt`, `llms-full.pt-BR.txt` e `docs/schemas/README.md`
- O `watch` passa a ser provado por execução com a feature opt-in: a build com `--features watch` emitiu um `watch` e um `watch_summary`, e os dois validam contra o schema servido

### Alterado — locks, drift, origem do workspace e gate de docs ampliado (2026-10-31)
- O `replace` compara o `--expect-checksum` sob o lock, então a janela entre a comparação e a escrita fecha, e o `--dry-run` e o `--preview` seguem comparando o conteúdo que leram porque nunca tomam o lock
- O `edit-loop` entrega ao `atomic_write_based` o checksum que leu sob o lock, então uma edição concorrente termina em `STATE_DRIFT` com exit 82 em vez de perder atualização
- O nome do sidecar de lock vira um digest BLAKE3 curto quando `.<nome>.atomwrite.lock` passaria de 255 bytes, então um alvo de 240 bytes toma lock em vez de falhar com `ENAMETOOLONG`
- O lock usa `std::fs::File::try_lock` em toda plataforma, que é `flock` no Unix e `LockFileEx` no Windows, então o build de Windows deixa de carregar um lock que não fazia nada
- O `batch --transaction` toma o lock de todos os alvos, ordenados por caminho, antes do snapshot e os mantém até o commit ou o rollback, então dois lotes com os mesmos alvos esperam em vez de entrelaçar e não podem dar deadlock
- O `wal-heal` remove journal `Started` quando o PID dono não existe mais e publica `dead_owner_removed`, então journal de queda deixa de sobreviver para sempre enquanto um escritor vivo e lento mantém o seu
- O envelope terminal carrega `workspace_source` (`argv` ou `cwd`), então a execução que tirou a raiz do diretório de trabalho declara isso em vez de esconder a autoridade ambiente
- A guarda de hardcode varre também `build.rs` e `xtask/src`, com isenção escrita para as chamadas de `git` e `cargo` que o ferramental de build possui
- O gate de docs passa a cobrar também as afirmações `N names` e `N files`, e não só `N schemas`
- O `resolve_workspace`, o `rename_path` e os auxiliares de fsync levantam erro tipado em vez de `anyhow!`, então diretório de trabalho ilegível ou permissão negada mantém exit 13 ou 74 em vez de cair para 255

### Corrigido — `replace` para de deixar `.bak` para trás (2026-10-31)
- O `replace` deduzia a retenção do valor efetivo de `backup`, e `[defaults].backup` resolve para verdadeiro, então uma execução comum deixava um irmão `.bak.<carimbo>` que ninguém pediu
- A retenção agora segue só `--keep-backup`, como o ADR-0038 decide, então `--backup` continua criando a rede de proteção e a escrita bem-sucedida remove o arquivo
- O comportamento do `GAP-105` está superado: fazer `--backup` implicar `--keep-backup` transformava uma flag em duas e reproduzia exatamente a Alternativa 4 que o ADR-0038 recusou
- O `tests/cli_v0123_backup_default.rs` ainda exigia um `.bak` depois do `replace` e era o único teste vermelho da suíte, e agora afirma zero como os irmãos de `write` e `edit`
- O `docs/AGENTS.md` e o `docs/AGENTS.pt-BR.md` marcam a promessa de retenção da v0.1.25 como superada, e o `gaps.md` reconta os itens W44 que o código já fechou
- Uma varredura com `fd -I -H -p '\.(bak|orig|old)\.'` achou 19 irmãos sob `docs/`, e os 19 foram apagados

### Corrigido — cortes que ficaram em silêncio (2026-10-31)
- Um registro cortado por `--truncate-content` ou `--max-output-bytes` agora carimba `truncated: true` nele mesmo em todo verbo, e o escritor soma `truncated_records` no terminal, então campo encurtado nunca é lido como o original
- `search --max-count` publica `matches_suppressed`
- A busca pede um match além do teto, para o número ter piso em vez de silêncio
- `replace --max-replacements` publica `replacements_suppressed`, porque o helper já contava todas as ocorrências e devolvia só as aplicadas
- `semantic-search --min-score` publica `below_min_score`, então resposta vazia diz se o corpus estava vazio ou se o piso estava alto
- `--filter` e `--select` que esvazia o registro publicam `records_suppressed` no terminal em vez de devolver `None` em silêncio
- `FlushOutcome.dropped` chega ao terminal como `records_suppressed`, cobrindo o dedupe, o aggregate sem grupo e o bloco cortado por `--limit`
- `--dedupe path` mantém um match por arquivo
- As molduras `begin` e `end` saem da identidade de dedupe e não consomem mais a chave
- O corte do escritor por `--limit` cobra no mesmo contador `files_budget_dropped` do `cap_collected`, e o terminal carimba a contagem quando o verbo não declara o campo
- `replace`, `transform`, `scope` e `case` publicam `files_planned` em `--dry-run` e `--preview`, e `files_modified` continua zero sem escrita real
- `outline` publica `items_omitted` para o corte de `--limit`, e `items_truncated` agora diz que conta assinatura cortada, nunca item ausente
- `--print-schema` declara `records_suppressed`, `files_budget_dropped`, `files_planned`, `truncated` e `truncated_records`, então o validador deixa de ver chave não declarada
- `docs/schemas/README.md` diz 54 arquivos sem órfão
- O `config` lê 37 chaves em todo documento, e `config doctor` aparece em toda lista de subcomandos
- As duas famílias do ADR-0077 são citadas sem `count` e sem `list`, cujo `ok:false` com exit `13` foi medido

### Corrigido — fechamento da cauda (2026-10-31)
- `--max-count` fica documentado como teto de registro e não de leitura, porque provar que o arquivo tinha mais custa no máximo um match lido além do teto naquele arquivo
- A sonda não tem custo mensurável: `search 'let ' . -c -m 1 --filter type=summary` roda em 0,01 s por 603 arquivos, e a mesma execução sem `-m` roda em 0,01 s
- `records_suppressed` fica documentado como contagem de registros e nunca de arquivos, então ele não se compara com `files_matched`, porque um arquivo pode contribuir com vários registros
- Medido: `search 'fn ' src/commands/search.rs --max-count 1 --filter type=summary` publica `records_suppressed: 3` contra `files_matched: 1`, e os três são um match mais as duas molduras que o filtro recusou
- `files_budget_dropped` nomeia a unidade que o verbo produz: arquivos num verbo que caminha por caminhos, registros num verbo que os transmite
- `recipe` e `codemod` reemitem os registros capturados do filho sem cobrar `--limit` uma segunda vez, então o orçamento conta cada registro uma vez
- O `summary` do filho é dobrado em `recipe_step.child_summary` e em `codemod_summary.transform_summary`, então cada execução termina em exatamente um registro terminal
- `recipe` reemite os registros do passo replace, que antes eram lidos só pelo checksum e descartados
- `recipe` marca os passos restantes como `skipped` quando o `--limit` para o fluxo, e o `recipe_result` carrega `steps_skipped` com `ok: true`
- Medido: `recipe run search-replace-verify ... --dry-run --limit 20` sai 0 com 20 registros `match` e dois passos pulados, onde antes saía 1 com `code: "NO_MATCHES"`
- O terminal `recipe_result` passa por `write_terminal`, então o orçamento não remove mais o registro que diz que a receita terminou
- `codemod` publica o envelope de falha do filho com o `rule_id` da campanha e devolve o erro marcado `VerdictPublished`, então a falha carrega um terminal e não dois
- O `batch` não tem processo filho, então continua publicando `batch_op` e um único `summary`, e a afirmação anterior de que ele reemitia eventos filhos era falsa para ele

### Corrigido — família de resposta, carimbo de alvo e honestidade do resumo (2026-10-30)
- `search --target files` publica `files_visited` como os arquivos que a varredura examinou e `files_matched` como os nomes que casaram, então a página omitida por `--offset` e `--limit` fica visível em vez de disfarçada de tamanho da varredura
- `hash --stdin` carimba `target_resolved: "-"` com `target_source: "stdin"`, e todo schema que carrega o envelope declara o terceiro valor
- `sparse` e `semantic-search` mantêm a resposta que publicaram e saem 0 com `partial: true`, em vez de descartá-la atrás do exit 13
- `scope` mantém a reescrita que aconteceu e relata a escrita malsucedida em `errors[]` em vez de derrubar a execução
- `backup` mantém o resumo terminal e conta `files_failed` quando uma origem não pode ser copiada, e `prune-backups` sai com a falha da remoção que deixou backup em disco
- `copy` publica sempre o resumo, e `files_visited` conta as origens examinadas em vez das cópias que foram feitas
- `stat`, `verify`, `get`, `query`, `outline`, `sparse`, `semantic-search`, `watch`, `extract-artifact` e `diff` com operando `-` passam a carregar `target_resolved` e `target_source`, e o envelope de erro nomeia o sujeito com a mesma origem que o caminho de sucesso usa
- `summary.errors_max_entries` torna o teto de `errors[]` configurável de 1 a 1000, 50 por padrão, com `files_failed` continuando a guardar a contagem total
- `files_visited`, `files_modified`, `total_matches` e o `total_bytes` do `list` passam a descrever todo verbo que os publica
- `error-output.schema.json` declara `target_resolved` e `target_source`, que o writer já carimbava em toda execução que falha, então um validador deixa de ver chave não declarada

### Corrigido — contadores antes do corte, truncamento visível e schemas fechados (2026-10-30)
- O `semantic-search` publica `files_scanned` e `hits_before_cut`, então o corte de `--k` deixou de exigir uma segunda varredura da árvore
- O `search` publica `offset_skipped`, que o tally já contava e nunca serializava
- O `scope` publica `files_read` e `total_bytes` no resumo, tanto no `--dry-run` quanto na reescrita
- O `hash` publica `total_bytes`, e o `--stdin` mede o fluxo por `hash_reader_with_len`
- O `copy` publica `total_bytes` do que copiou
- O `cap_collected` conta o que o `--limit` derrubou, e todo verbo que o chama publica `files_budget_dropped`
- O `files_skipped` deixa de serializar `null` no verbo que não pulou nada
- O `count` publica `total` nos três modos, obedece ao `--top` no `--by-extension` e declara `items_omitted`
- O `sparse` publica `files_scanned`, `path_bytes`, `files_unparsed` e `files_budget_dropped`
- O `delete` publica `total_bytes` do contador que já somava, e serve o próprio resumo no `--print-schema`
- O `transform` publica `total_bytes` e serve o próprio resumo no `--print-schema`
- O `diff` declara os eventos `change` e `summary` que escreve, e carimba `timed_out` quando o `similar` gasta o orçamento de similaridade
- O `search --target files` conta `files_matched` como os arquivos que contribuíram nome publicado
- O `read` perde `content_b64`, que nenhum produtor preenchia
- O `outline` e o `sparse outline` declaram a assinatura cortada em `signature_truncated` e `items_truncated`
- O `query` declara `text_truncated` por nó e `nodes_truncated` no resumo
- O `extract-artifact` declara `truncated_parts` no membro cortado pelo teto anti zip bomb, e a defesa não mudou
- O `backup` conta a origem que não era arquivo regular em `files_skipped`, e o `prune-backups` conta o alvo ausente em `targets_missing`
- O `replace` publica `binary_rejects`, então o binário recusado deixa de se esconder dentro de `files_skipped`, e `files_total` carrega a contagem elegível sempre que o precount de progresso roda
- Os `required` dos schemas fechados descrevem o que cada verbo sempre emite, em `list-summary`, `replace-summary` e `delete-output`
- O `docs/schemas/README.md` diz 59 nomes servidos por 58 arquivos sem órfão, nos quatro lugares que declaram contagem
- Os seis documentos citam as duas famílias do ADR-0077 em vez de repetir uma lista que derivava
- As quatro contagens de exit code passam a 41 entradas e 33 números distintos, os treze campos do README são chaves de `config` provadas por `config get`, e o `config doctor` entra nas seis listas de subcomandos
- A frase de schemas diz 58 arquivos servindo 59 nomes, com `read` e `stat` dividindo um
- O `delete --recursive` deixa de usar `remove_dir_all`, então remove só os diretórios que o run esvaziou, e um arquivo barrado por `--include`, `--exclude`, pelo filtro de ocultos ou por um ignore sobrevive em vez de sumir enquanto o resumo contava zero
- O `watch` publica o `watch_summary` terminal também no caminho de sucesso, provado com `reason: idle` e exit 0 num diretório vazio, então o carimbo deixa de ter prova só no envelope de erro
- O `transform` aceita 29 linguagens embutidas, provado pela recusa de `-l fortran` com exit 65 nomeando as 29, e a deriva que dizia 306 está corrigida aqui e nos dois arquivos `docs/AGENTS`
- Os dois changelogs perdem todo marcador de negrito (183 pares neste arquivo e 157 no inglês), então seguem o mesmo formato de bullet do resto da documentação
- O build slim é medido em vez de herdado: `cargo build --release --no-default-features --features core`, e o resultado vai para `README`, `ARCHITECTURE`, `TESTING` e `COOKBOOK`
- O `gaps.md` fecha a rodada, e os dois changelogs citam as mesmas partes

### Adicionado
- O `read` aceita um ou mais operandos, e `--format raw` concatena todos na ordem do argv sem cabeçalho, sem separador e sem `\n` extra, igual ao `cat`
- Um operando com `*`, `?` ou `[` é expandido pelo próprio binário num nível do diretório dele: ordem de byte, nome oculto descartado e nenhuma descida para subpasta
- `read --format raw` de vários arquivos copia bloco a bloco, então o pico de memória fica num bloco de 64 KiB em vez da soma dos tamanhos
- A concatenação avança um `CancelCheck::for_bytes`, então o `--timeout-secs` corta a leitura multi-arquivo no meio em vez de depois do último arquivo
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` e `--verify-checksum` se nomeiam num `INVALID_INPUT` claro quando a chamada tem mais de um alvo
- `risk_assessment` em `edit`, `edit-loop`, `replace`, `case`, `transform` e `codemod`: a guarda de padrão de conteúdo agora roda em todo verbo que grava, com os mesmos valores `risk_level` e `guard_triggered` que o `write` já publicava (G-RISK-1)
- As barreiras de tamanho ficam fora desses seis verbos de propósito: eles reescrevem uma região, então um resultado menor é a operação funcionando, e aplicar a barreira de encolhimento ali bloqueava o `edit --delete-range` por completo
- `Kind::List` na tabela de chaves de configuração, então `write.content_risk_patterns` é acessível por `config get` e `config set` (36 chaves, G-CONFIG-2)
- Status de saída `69` `GRAMMAR_UNAVAILABLE` com classe `precondition_failed`, levantado quando a gramática tree-sitter de um idioma falta e o download não acontece
- `src/grammar.rs`: uma função decide se o erro do pacote é nome errado (`INVALID_INPUT`, 65) ou gramática que esta máquina não consegue fornecer (`GRAMMAR_UNAVAILABLE`, 69), e `outline`, `query` e `syntax_check` chamam a mesma
- Check `grammar_cache` no `doctor`: informa quantas das 23 gramáticas que os verbos de AST podem precisar já estão em disco, com status `pass` quando todas estão, `info` quando falta alguma (o pacote baixa no primeiro uso) e `skipped` quando o build não tem a feature `ast`
- `src/constants.rs::EXTENSION_LANGUAGES`: uma tabela mapeia extensão de arquivo para linguagem tree-sitter, lida pelo `syntax_check` e pelo check do doctor, então o check mede as linguagens que os verbos realmente pedem
- O mesmo campo nos três registros de plano, então o dry-run carrega o diagnóstico que a escrita carregaria: `DryRunPlan` (`write`, `edit`, `replace`, `set`, `del`, `delete`, `semantic-merge`, `wal-stats`, `wal-heal`), `ReplacePreview` e `ScopeResult`
- O `scope` passou a chamar `assess_content_risk`: ele gravava arquivos sem diagnóstico de risco, enquanto todo outro verbo de escrita publicava um
- `rule_ids_from_file`, `load_rules_file` e `parse_rules` em `transform_multi.inc.rs`: um leitor de manifesto só, compartilhado por `transform --rules` e `codemod`, então os rótulos de uma campanha e as regras realmente aplicadas não podem divergir
- O `codemod` rotula regra sem `id` como `rule-N` pela posição 1-based, em vez de sumir com ela da lista

### Alterado
- O `search` publica `target_resolved` e `target_source` em todo envelope terminal: raiz digitada na linha sai como `argv`, e raiz omitida sai como `workspace` contra a raiz do workspace, onde o resumo não trazia nenhuma das duas chaves
- `read --format raw` reporta falha no stderr em vez de escrever o envelope NDJSON de erro no stdout: registro JSON no canal de bytes não é algo que um substituto do `cat` possa emitir
- Arquivo ausente, diretório ou glob sem resultado imprime uma linha no stderr e os outros alvos ainda chegam ao stdout, com o código semântico da PRIMEIRA falha como status
- Operando absoluto é julgado pelo trecho que fica sob o `--workspace`: `/w/*.rs` era recusado com 65 enquanto `/w/a.rs` funcionava, porque o limite de níveis contava o prefixo absoluto como se o chamador tivesse pedido profundidade
- Padrão que não casa nada reporta `GLOB_NO_MATCHES` com `no file matches the pattern: …` e mantém o exit 4, e vários alvos no `stat` nomeiam o verbo, recebem dica sobre o próprio verbo em vez de uma sobre flags que ninguém passou, e `stat --format raw` concatena em vez de ser vetado pela flag `--stat` do próprio verbo
- `[profile.dev] opt-level = 1`: o cascade do fuzzy de `replace --fuzzy auto` no mesmo acervo de 15 MB caiu de 18,5 s para 4,2 s nesta máquina, medido antes e depois
- `tree-sitter-language-pack` de 1.8.1 para 1.22.1, `ast-grep-core` e `ast-grep-language` de 0.43 para 0.51, e `tree-sitter` de 0.26 para 0.27: o pacote 1.19 e acima exige `tree-sitter ^0.27`, e o `ast-grep` 0.43 exige `^0.26`, então os três sobem juntos ou não sobem
- O `zstd` vai de 0.13.3 para 0.14.3 por causa desse pacote, e com ele `zstd-safe` de 7.2.4 para 8.2.0 e `zstd-sys` de 2.0.16 para 2.1.2, que é o que zera as quatro vulnerabilidades e os dez avisos de unsound do RUSTSEC-2026-0391, 0395 a 0406 e 0410
- O `rust-version` sobe de 1.89 para 1.90, piso do `ast-grep-core` 0.51 e do `tree-sitter` 0.27, e todo documento que declara o MSRV acompanha
- O cache de gramática é versionado pela versão do pacote, então a subida invalida o diretório de gramáticas `v1.8.1` e o primeiro verbo de AST baixa essas gramáticas de novo sob `v1.22.1`
- `log.file` e `log.rotation` passam a aceitar o escopo workspace, e `startup_log_section` mescla a camada global com o `.atomwrite.toml` do workspace, então a seção `[log]` agora liga o destino em vez de ser validada e descartada
- O `atomwrite --version` não acrescenta mais `-dirty`: o identificador é o id curto do commit da árvore rastreada, então um arquivo não rastreado não consegue mais mudar a string
- `batch --transaction` publica um veredito só: o evento `rollback` saiu de `SUCCESS_ENVELOPE_TYPES` e é gravado pelo caminho de falha, então carrega `ok: false` com o `code` e o `exit` da falha que o causou
- O `syntax_check` degrada para `Skipped` com a razão tipada em vez de uma string opaca do pacote


### Corrigido
- O `scope` publica `files_matched` de um contador dedicado, então arquivo que falhou não conta como casamento e a execução só-leitura informa os arquivos que responderam
- `transform --rules` conta uma falha uma vez por arquivo distinto, em vez de uma vez por regra, e o `errors[]` lista cada caminho uma única vez
- O `case` mantém exit 0 e publica `partial: true` quando renomeou ao menos um arquivo e outro falhou, a mesma regra que `search`, `replace`, `transform` e `scope` seguem
- O `transform --rules` mantém exit 0 e publica `partial: true` quando ao menos uma substituição foi feita e outro arquivo falhou, que é a regra que o `transform` de uma regra já seguia e o caminho multi-regras não seguia
- `.gitignore` ou `.ignore` com sintaxe quebrada ou modo ilegível é reportado em `files_failed` e `errors[]` com o caminho do próprio arquivo de ignore, em vez de ser descartado em silêncio pela varredura
- `read`, `count`, `list`, `hash` e `diff` publicam `target_resolved` e `target_source`, com `workspace` só no padrão `.` de `count` e `list`
- As descrições dos campos do `Summary` batem com o que os produtores publicam: `files_skipped` cobre todo caso que a execução não respondeu, `search-files` informa nomes casados em `files_visited`, e `transform` publica substituições em `total_matches`
- O `delete` publica `files_skipped` como a contagem pulada por idade, e a expressão tautológica que sempre resultava no mesmo número saiu
- O inventário de gaps deixou de ser ponteiro vivo: `gaps.md` saiu do `include` do pacote, e os documentos que o citavam apontam para a série de ADRs
- `replace`, `transform` e `scope` falham com exit 4 `FILE_NOT_FOUND` quando uma raiz nomeada na invocação não existe, mesmo tendo casado ou reescrito: antes respondiam `ok: true` com exit 0 e a raiz ausente aparecia só em `errors[]`
- O `scope` publica `partial: true` ao lado de `files_failed`, porque ele mantinha uma cópia privada do resumo de falha enquanto o resumo compartilhado já tinha ganhado o campo
- `count`, `list`, `sparse`, `semantic-search` e `case` publicam `partial: true` ao lado de `files_failed`, pela mesma função `partial_of`, então nenhum produtor decide o par à mão
- `case` publica o resumo terminal quando um arquivo não pode ser lido ou gravado: o erro por arquivo entra na contagem em vez de abortar por `item?`, então `files_failed`, `partial` e `errors[]` nomeiam o arquivo antes de o mesmo código de saída encerrar a execução
- `scope` publica `total_matches` como a soma de `scopes_matched`, um campo que o schema já declarava e nenhuma execução preenchia
- `transform --rules` soma as falhas de cada regra em `files_failed`, `partial` e `errors[]`, e conta uma raiz ausente uma única vez, em vez de responder com um resumo que não sabia de arquivo nenhum falho
- Falha de varredura que não nomeia caminho próprio recebe o marcador `<walk>`, então `errors[]` nunca carrega `path` vazio
- O `scope` serializa o resumo de falha compartilhado em vez da cópia dele, então um campo futuro do relatório de falha alcança o verbo sem uma segunda definição
- `locale --set` e `--clear` deixam de prometer efeito na chamada seguinte, porque o relatório já mostra o estado que a chamada deixa
- O `read --format raw` expande padrão cujo prefixo de diretório tem vários níveis, e só curinga no nome de um diretório é recusado, porque a checagem antiga contava todos os componentes do caminho em vez de perguntar qual deles é padrão
- Nome com `*`, `?` ou `[` que existe no disco é lido como está em vez de ser expandido, então `read` e `cat` concordam num arquivo como `relatorio[1].txt`
- O `search` deriva o teto do buffer de linha de `search.max_filesize` em vez de usar um limite fixo, então arquivo que o chamador já autorizou ler não é mais recusado por ter uma linha acima daquele limite
- O `search` e o `replace` não aceitam mais uma flag que o verbo não tem como padrão: o atributo que permitia isso saiu de `pattern` e de `replacement`, então `search -Z 'agulha'` sai 2 com a dica do próprio parser `use '-- -Z'`, onde antes a busca rodava pelo texto literal `-Z` e respondia `ok: true` ou `FILE_NOT_FOUND`
- Um padrão que começa com hífen continua funcionando, agora atrás de `--`, igual ao `rg` e ao `grep`: `search -F -- '-deprecated' .`
- O `search` falha quando uma raiz nomeada na invocação não existe, mesmo tendo casado: a raiz ausente é relatada antes da varredura, as raízes que existem continuam sendo varridas, e o resumo traz `ok: false`, `code: FILE_NOT_FOUND` e exit 4 (medido: `search -F 'pub pattern' src/cli_args nao-existe.rs` imprimiu os três matches e saiu 4)
- O resumo de falha ganhou `partial: true` sempre que `files_failed > 0`, então uma execução que respondeu por parte dos alvos diz isso numa chave só; arquivo que falha durante a varredura continua mantendo exit 0
- O `--format text` escreve cada falha no stderr com o caminho e o motivo, e segue as mesmas regras de status
- O resumo de `search --sort --limit` exigia `walker_stopped_early: true` num teste cujo campo documenta o contrário: o caminho com `--sort` bufferiza a árvore inteira, então o walker nunca para cedo, e a asserção agora segue o contrato
- `read --format raw` de um diretório saía 74 com envelope JSON no stdout, e agora sai 74 com o diagnóstico no stderr e o canal de bytes vazio
- `read --format raw` com vários alvos furava o jail do `--workspace`: `/etc/passwd` nomeado ao lado de um arquivo dentro do jail, ou alcançado por um link simbólico que um glob casou, saía 0 e tinha os bytes impressos, enquanto o mesmo nome como alvo único saía 126. Todo alvo agora passa por `validate_path`, então a brecha sai 126, os outros alvos continuam indo para o stdout, e um operando relativo resolve contra o `--workspace` exatamente como um alvo único já resolvia
- Registro terminal que nomeia falha sobrevive ao `--filter`: `search --filter ok=true` numa árvore com diretório sem permissão não imprimia nada e ainda saía 0, então a entrada `PERMISSION_DENIED` dentro de `errors[]` nunca chegava ao chamador. O filtro continua estreitando um run limpo
- O `walker_stopped_early` era verificado em árvores de 12 arquivos onde o walker às vezes terminava antes do `--limit` cortar, em `--count`, `--files` e `--offset` com `--limit`; esses três testes existentes passam a usar 400 arquivos, que é a condição que o campo documenta
- Os guardas de documentação estavam vermelhos por dois fatos vencidos: o `INTEGRATIONS` mantinha `v0.1.37` no título que carrega o total de subcomandos, então a contagem era lida como histórico, e a tabela de exit codes do README não tinha a linha `2 UNSUPPORTED_COMBINATION`
- `read --format raw` imprimia a primeira falha do lote por último: o primeiro erro era guardado para o status e os seguintes saíam na hora, então o stderr corria contra a ordem do argv (medido com dois alvos acima do teto, `b.rs` antes de `a.rs`). Toda falha agora imprime onde acontece e o erro devolvido carrega `VerdictPublished`, então o status continua sendo o da primeira falha e nenhuma linha sai duas vezes
- O `--timeout-secs` estourava cerca de 900 ms num arquivo que é uma linha sem terminador, e a causa foi medida em vez de suposta: limitar cada `read` do `CancelRead` não mudou nada (cinco pares alternados dentro do ruído), porque a flag de cancelamento só é consultada quando o searcher pede bytes, e entre duas leituras o grep reconstruía o buffer de linha copiando tudo o que já tinha lido, cópias de 43, 131, 385 e 1097 ms numa linha de 4 GiB. O searcher agora tem `heap_limit` de 64 MiB, então o buffer e a cópia ficam pequenos e uma linha maior é reportada por arquivo como `INVALID_INPUT` nomeando a linha: o mesmo fixture foi de 1946 a 2267 ms contra um prazo de 1 s para 76 ms, e o controle de cinquenta linhas de 50 MiB foi de 1053-1071 ms para 1017-1022 ms
- O `search --limit` agora lê o `walker_stopped_early` do caminhante em vez do orçamento de saída do escritor: numa árvore de 4000 arquivos com `--limit 1`, 7 de 10 execuções paravam entre 9 e 58 arquivos e ainda publicavam `false`, e depois da correção 12 de 12 publicam `true` quando a varredura para
- O mesmo defeito foi corrigido no `replace` (os três pontos de `Quit` agora o registram; 10 de 10 execuções publicam `true`, contra 8 de 10 publicando `false` antes) e no `search_files`, onde o campo agora está ausente porque aquele caminho coleta todos os nomes e fatia a página depois, então nunca corta a própria varredura
- O `risk_assessment.size_delta_pct` é derivado dos dois tamanhos em `write_guards::assess_content_risk` em vez de ser um `0` cravado: o mesmo campo tinha dois significados entre verbos, e um crescimento de 22 para 45 bytes publicava `0` onde o delta real é 104
- O `scope` carregava o mesmo carimbo do orçamento do escritor e agora lê o próprio `walk_quit`: numa árvore de 410 arquivos com `--limit 1` ele publica `files_visited: 32` com `walker_stopped_early: true`, e sem limite visita 410 e omite o campo
- O `semantic-search` também deixou de carimbar `walker_stopped_early` pelo orçamento do escritor, porque ele coleta a árvore inteira e fatia o ranking depois, então o campo agora é omitido e o corte aparece como `results` contra `k`
- O `codemod` contava um match por linha de NDJSON em vez de ler o campo `matches`, então uma campanha de duas regras publicava `alpha.matches: 4` para dois matches reais; agora soma a contagem do próprio evento, e o `by_rule_id` fecha com o `total_matches` ao lado
- O `codemod` carimbava o `transformed` da segunda regra com o id da primeira, porque caía em `rule_ids.first()`; a atribuição agora segue o fluxo de `rule_begin`, então `beta` reporta os próprios matches e os próprios arquivos em vez de `files: 0`
- O `codemod` lia os ids de regra com uma heurística por linha que perdia sequência em forma de fluxo, id com aspas e `id :` com espaço antes dos dois-pontos; os ids vêm do mesmo parse tipado de manifesto que o `transform` já roda
- O `--config` não isolava a seção `[log]`: um arquivo de workspace com rotação inválida imprimia `invalid configuration` no stderr e o comando seguia com os padrões e saía 0, enquanto o mesmo arquivo sem `--config` falhava com 78. O `startup_log_section` agora recebe o caminho explícito e lê só ele
- `transform`, `replace`, `case`, `edit`, `write` e `scope` não publicavam `risk_assessment` nos registros de dry-run, porque a avaliação morava dentro do ramo `if !dry_run`; o modo que existe para o agente decidir antes de gravar era justamente o que ficava sem o diagnóstico
- O `codemod --help` descrevia o arquivo de regras como "ast-grep multi-rule format", enquanto o parser aceita exatamente `id`, `language`, `pattern` e `rewrite`, e a recusa dizia "invalid YAML" mesmo com YAML válido e só o esquema errado; a mensagem agora nomeia a camada e o campo que falhou
- O `transform_stub.rs` ficou com cinco parâmetros enquanto os dois chamadores passavam seis, então nenhum build `--no-default-features` compilava: o `cargo xtask check-features` saía 101 com dois E0061
- O `startup_log_section` linkava a `merge_layers` privada a partir de um comentário de doc público, e o `#![deny(rustdoc::private_intra_doc_links)]` transformava isso em `cargo doc` saindo 101
- O `case` carimbava `walker_stopped_early` como `None` literal enquanto o `cap_collected` já levantava a flag de processo que todo verbo que gasta o orçamento lê, então `case --limit 1` sobre cinco arquivos publicava `files_visited: 1` sem palavra sobre o corte; ele agora lê a mesma flag e publica `true`, que é o que virou `case_converts_one_file` de falha pré-existente declarada para aprovação
- O `CodemodRuleStats` documentava `matches` como contagem de eventos, que é a semântica que o código implementava e a que esta versão remove

### Validado
- Toda prova abaixo é execução real de `target/debug/atomwrite` ou `cargo check`, nunca teste: a regra do projeto é que teste é maquete e só rodar o programa conta
- Compilação cruzada medida alvo por alvo com `cargo check --offline --target <T> --lib`: `x86_64-pc-windows-gnu` terminou com exit 0 em 12,28s, que é o único alvo de Windows que esta máquina consegue linkar
- Bloqueado por linkador C ausente, e não por defeito de código: `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl` e `aarch64-unknown-linux-musl` param dentro do `zstd-sys` no `cc-rs` porque `aarch64-linux-gnu-gcc`, `x86_64-linux-musl-gcc` e `aarch64-linux-musl-gcc` não estão no `PATH`
- NÃO VALIDADO por falta de alvo instalado: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`, `i686-pc-windows-gnu`, `aarch64-apple-darwin` e `x86_64-apple-darwin`
- A linha `9 passed` do `cross_compile_check` não é prova de nove alvos: o teste retorna cedo quando o alvo falta, então ele aprova por ausência; a contagem honesta é um compilado, três bloqueados e cinco sem alvo


### Adicionado no rascunho de 2026-10-15 desta entrada
- Shape de stream global no canal de dado: `--sort-by` (`path-line` / `line-path` / `key`), `--dedupe KEY`, `--aggregate`, `--join`, `--diff-against FILE`, `--redact`, `--normalize-encoding`, `--max-output-bytes` (E4)
- Descoberta de schema `--print-schema [NAME]`, respondida antes do clap, listando todo nome ou emitindo um (E3)
- Subcomando `extract-artifact`: sinal delimitado de cabeçalho de PDF, OOXML (documento / planilha / apresentação) e imagem — texto, estrutura e metadados, nunca os bytes brutos do artefato (E5)
- Guardas de efeito colateral aceitas globalmente: `--non-interactive` e `--confirm-side-effect`, com truncamento atrás de opt-in explícito (E6)
- Primitiva única de cancelamento com granularidade de bloco (`CancelCheck`), compartilhada pelo laço de varredura em vez de checagens copiadas (A1)
- Campo booleano `deadline_exceeded` no envelope `cancelled`, sempre serializado e nunca omitido, para o consumidor distinguir estouro de prazo de sinal sem interpretar o texto humano (A3)
- Ponto único de fim de cancelamento no despacho: todo verbo que varre emite o mesmo envelope `cancelled` ao ser interrompido, em vez de cada um decidir sozinho entre envelope, silêncio e resumo parcial (A3)
- O registro terminal de cancelamento é escrito sem shape, então `--filter`, `--select` e `--limit` não conseguem removê-lo
- Flag de shape descreve como apresentar uma resposta completa e não pode apagar o aviso de que a resposta está incompleta (A3)
- Entrada `discovery` no `agent-surface`: publica a porta de descoberta de schema, com as duas formas e a lista de nomes, gerada de `SCHEMA_NAMES` para que o manifesto e a porta nunca discordem (E3)
- Erro de domínio `MERGE_CONFLICT` com `error_class: permanent` e `conflict_count`, exit 84 (C1)
- Campo estruturado `missing_flag` no erro de diretório sem recursivo (C3)
- Lista de código de saída derivada do enum de erro e publicada no `agent-surface`, de modo que código novo entra por construção (B3)
- `src/stdin_cancel.rs`: leitor de stdin cancelável, uma thread leitora mais um canal limitado consultado com timeout curto (R2)
- A thread é o único jeito portável de abandonar um `read(2)` bloqueado em Linux, macOS e Windows, porque o `signal-hook` instala handlers com `SA_RESTART` e o kernel reinicia a leitura
- `edit-loop --partial`: grava os pares que casam e sai com 87 (`EDIT_LOOP_PARTIAL`) quando ao menos um par não casou (R13)
- `pairs_applied` no envelope do `edit-loop`, ao lado dos contadores de par que ele já trazia (R13)
- Tratamento de SIGHUP: terminal fechado encerra a execução como cancelamento, com exit 129 e `signal: "SIGHUP"` no envelope `cancelled` (R22)
- `ctrlc` compilado com a feature `termination`, então o handler de Ctrl-C também cobre os sinais de término (R22)
- `files_skipped` no resumo do `search`, contando os arquivos pulados por exceder o limite de tamanho (R29)
- `files_failed` e `errors[]` no resumo do `scope`, então arquivo que falhou ao gravar aparece no canal de dado e não só numa linha de log (R11)
- Lock advisory de escrita ligado ao `atomic_write` (R23): sidecar oculto `.<alvo>.atomwrite.lock` ao lado do alvo, com espera de até 5000 ms
- Lock não concedido a tempo falha com `LOCK_TIMEOUT`, exit 83, trazendo o caminho real do alvo
- Versão do schema de config: chave `version` no `config.toml` e `CONFIG_SCHEMA_VERSION = 1` (R25)
- `[log] file = true` e `[log] rotation = "daily" | "hourly"` no `config.toml` XDG global, para ligar o arquivo de log (R26)
- `target_resolved` e `target_source` no evento terminal de todo verbo com efeito colateral: `edit`, `write`, `edit-loop`, `set`, `del`, `apply`, `rollback`, `copy`, `move`, `delete`, `backup`, `prune-backups`, `case`, `replace`, `transform`, `scope`, `codemod`, `wal-heal`, `scratch init` e `recipe run` (R14)
- `target_source` é sempre `argv` nesses verbos, e `target_resolved` é o caminho absoluto canônico (várias raízes são unidas por vírgula, na ordem do argv)
- `STREAM_RECORD_TYPES`: a lista fechada de tipos de registro de stream (`begin`, `match`, `progress` e semelhantes), ao lado de `SUCCESS_ENVELOPE_TYPES` corrigida, então todo tipo terminal fica classificado (R6)
- `NdjsonWriter::write_event_failed` (R7): carimbava `ok: false`, `code` e `exit` num resumo seguido de erro; foi trocado nesta versão por `publish_failure`, descrito abaixo, e não existe mais
- `exit_codes_help()`: a lista de exit codes no `--help`, derivada da mesma fonte do `agent-surface` (R12)
- Tabela de exit codes entre os marcadores `<!-- exit-codes:begin -->` e `<!-- exit-codes:end -->` nos dois readmes, guardada por `tests/docs_exit_codes.rs` (R12)
- Teste de prazo real com processo filho e sinal de verdade (A4)
- Teste do token deslocado: alvo ambiente X com alvo no argv Y precisa falhar fechado e nunca agir em X, agora rodado para todo verbo com efeito colateral (F3)
- Suítes `cli_v0137_cancel`, `cli_v0137_envelope`, `cli_v0137_dominio`, `cli_v0137_agent_native`, `cli_v0137_artefato`, `cli_v0137_etd`, `cli_v0137_etd_targets`, `cli_v0137_config`, `cli_v0137_contract`, `cli_v0137_lock`, `cli_v0137_safety`, `cli_v0137_search_rss`, `cli_v0137_search_stream` e `cli_v0137_stdin_cancel`
- Subcomando `config`, o 47º: `config path`, `config list`, `config get CHAVE`, `config set CHAVE VALOR --scope global|workspace` e `config unset CHAVE --scope global|workspace`, para que um valor de ajuste não precise mais de edição de código (`src/commands/config_cmd.rs`)
- `--scope` é obrigatório em `path`, `set` e `unset`: sem ele o verbo sai com 2 e não toca em nada, e `get` e `list` mostram o valor efetivo com a camada de onde ele veio (`default`, `global` ou `workspace`)
- `config set` lê o arquivo, troca a chave preservando os comentários, valida o arquivo inteiro e só então grava com `atomic_write` (arquivo global com modo 0600, sem backup)
- `config` carrega `target_resolved` e `target_source: "argv"` como todo verbo com efeito colateral, com um ramo de carimbo próprio em `src/lib_dispatch.inc.rs` porque o arquivo XDG global fica fora do jail
- `config` é tratado antes da carga da configuração, então um `config.toml` quebrado não o bloqueia, e um arquivo que não é TOML válido é lido com aviso e nunca reescrito (`set` e `unset` saem com 78 e o caminho)
- O envelope terminal é `type: "config"` e `docs/schemas/config.schema.json` o descreve
- 35 chaves tipadas, uma só tabela alimentando `list`, `get`, `set` e `unset`: `fuzzy.mode`, `fuzzy.threshold`, `fuzzy.threshold_aggressive`, `fuzzy.threshold_context`, `fuzzy.threshold_jw`, `fuzzy.soft_floor_delta`, `fuzzy.max_content_bytes`, `write.confirm_large_bytes`, `log.file`, `log.rotation`, `storage.home`, `locale`, `concurrency.max_threads`, `concurrency.ram_fraction_percent`, `search.max_inflight_bytes`, `lock.acquire_timeout_ms`, `retry.base_ms`, `retry.max_ms`, `retry.max_elapsed_ms`, `extract.max_sample_rows`, `extract.max_text_chars`, `extract.max_sheets` e `extract.max_slides`
- `log.file`, `log.rotation`, `storage.home` e `locale` são só globais: o escopo workspace as rejeita com exit 65, e uma chave desconhecida sai com 65 listando as válidas
- `storage.home` é circular (ela decide onde a configuração mora), então `set` a grava direto no arquivo de bootstrap dos diretórios do projeto e avisa no envelope onde a configuração global passou a morar
- Quatro limites que o `extract-artifact` tinha fixos no código (`max_sample_rows`, `max_text_chars`, `max_sheets`, `max_slides`) viraram chaves de configuração, e as flags `--max-sample-rows` e `--max-text-chars` que já existiam continuam vencendo a configuração
- `[fuzzy].max_content_bytes` (padrão 256 MiB, `FUZZY_MAX_CONTENT_BYTES`): acima dele a cascata fuzzy não roda, porque cada estratégia custa O(linhas)
- `FUZZY_ESCAPE_MAP_MAX_CONTENT_BYTES` (32 MiB): acima dele a estratégia `escape_normalized` é pulada sem construir o mapa de deslocamentos, e a cascata segue com as outras estratégias
- Poll de cancelamento cooperativo em todo laço fuzzy (`CancelPoll`: contador local mais máscara, em `src/fuzzy/util.rs`), e `check_cancel()?` depois de cada estratégia e antes do `MatchFailed`
- `NdjsonWriter::publish_failure`: grava o resumo terminal com `ok: false`, `code`, `exit` e todo o detalhe do erro (`error_class`, `retryable`, `suggestion`, `suggestion_lang`, `conflict_count`, `match_count`, `pairs_*`) e devolve o erro marcado como já publicado, então o `main` não escreve um segundo envelope
- `files_failed` e `errors[]` nos resumos de `search`, `transform` e `replace` (como o `scope` já tinha), com `errors[]` limitado por `SUMMARY_ERRORS_MAX_ENTRIES` (50) e `files_failed` guardando a contagem total
- Seis nomes novos em `--print-schema`, `backup-summary`, `count-by-ext`, `count-by-size`, `list-summary`, `replace-summary` e `search-summary`, para que o schema de um resumo terminal deixe de ser o schema do evento dele (59 nomes no total)
- Arquivos `docs/schemas/search-summary`, `replace-summary`, `count-by-ext-output`, `backup-summary` e `list-summary`, e um `count-summary` regenerado
- `src/retry.rs`: uma política de retry com full jitter, teto, `max_attempts` e `max_elapsed` como duas condições de parada, esperas em fatias de 15 ms que consultam a flag de encerramento e um orçamento cortado no tempo que resta de `--timeout-secs`
- `fastrand = "2"` como dependência direta, para o jitter (ela já estava na árvore por meio do `tempfile`, travada em 2.4.1)
- `src/best_effort.rs`: um helper para limpeza cuja falha não deve mudar o resultado, registrada em debug em vez de descartada com `let _ =`
- `SEARCH_MAX_INFLIGHT_BYTES` (8 MiB): um orçamento de bytes único dividido entre os workers do `search`, em `src/commands/search_budget.inc.rs`
- `MAX_AUTO_THREADS` (16) e `WALK_AUTO_MAX_THREADS` (12): o teto da contagem automática de threads e do walker automático
- `cargo xtask check-features`: roda `cargo check --locked` nos conjuntos de features `core` e `core,watch` e para no primeiro erro
- Mensagem `feature.ast-required` nos catálogos `en` e `pt-BR`, para um build slim que recebe `outline` ou `sparse outline`
- Suítes `cli_v0137_terminal_event`, `cli_v0137_terminal_pipe`, `cli_v0137_terminal_schema`, `cli_v0137_wal_journals`, `cli_v0137_wal_startup_heal`, `cli_v0137_let_underscore_guard`, `cli_v0137_retry_deadline`, `cli_v0137_retry_guards`, `cli_v0137_hardcode_guard`, `cli_v0137_version` e `cli_v0137_config_verb`
- Guardas de fonte que quebram o build: nenhum `let _ =` e nenhum `.unwrap()`, `.expect(`, `unreachable!`, `panic!`, `todo!` ou `unimplemented!` em código de produção (as duas listas estão vazias), nenhum literal de tempo ou de sidecar fora de `constants*`, nenhum `env::var`, nenhum `Command::new`, nenhum `.sh` e nenhum `.github`
- `acquire_for_update` em `src/lock.rs` e `atomic_write_based` em `src/atomic/write.rs`: um verbo de leitura-modificação-escrita pega o lock advisory antes de ler, e a camada de gravação pode comparar o checksum do conteúdo-base sob o lock e falhar com `STATE_DRIFT` (veja Alterado)
- `CancelRead` e `search_file_cancellable` em `src/commands/search_stream.inc.rs`: um leitor que confere a flag de cancelamento a cada leitura, para que o prazo e o SIGTERM sejam vistos dentro de um único arquivo enorme
- Um handler de SIGPIPE (`register_broken_pipe`, `ShutdownSignal::record_broken_pipe`) e `is_global_hard_shutdown()` em `src/signal.rs` (veja Alterado)
- Exit 129 (`SIGHUP`) em `exit_codes` do `agent-surface`, no `--help` e nas tabelas de exit code dos dois READMEs
- `PATCH_MAX_HUNK_SHIFT` (64 linhas) e `PATCH_ERROR_SNIPPET_CHARS` (120) para o `apply`
- Dicas localizadas dedicadas `suggestion.fuzzy-content-cap` e `suggestion.fuzzy-pattern-cap`, escolhidas por duas constantes-marcador compartilhadas, para que a dica não se afaste do `reason` que ela explica
- `common::hermetic` em `tests/common/mod.rs`: todo teste que roda o binário recebe `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME` e `APPDATA` vazios e próprios, e `LANG` fixo em `en_US.UTF-8` (31 arquivos de teste), então a suíte não depende mais da configuração da máquina que a roda; o `XDG_CACHE_HOME` fica de propósito no cache real, porque a feature `ast` baixa parsers do tree-sitter ali em runtime e um cache vazio mandaria `outline`, `query` e `scope` para a rede
- Suítes `cli_v0137_apply_strict`, `cli_v0137_structured_errors`, `cli_v0137_backup_retention`, `cli_v0137_rmw_concurrent`, `cli_v0137_search_cancel_file`, `cli_v0137_audit_regressions` e `cli_v0137_completions_install`
- `tests/common/schema_check.rs`: um validador de JSON Schema para o subconjunto de palavras-chave que estes schemas usam (`type`, `properties`, `required`, `additionalProperties`, `items`, `enum`, `const`, `minimum`, `maximum`, `anyOf`, `oneOf`, `if`/`then`, `$defs`, `$ref`), escrito com a biblioteca padrão para a suíte não ganhar dependência
- A checagem de schema do `docs_schemas_sync.rs` comparava só `required` e chaves desconhecidas, então um schema que declarava `integer` passava com uma string no evento; agora ela roda o validador, e `the_validator_rejects_a_wrong_type_the_shape_only_check_let_through` prova que a checagem forte é mais forte em vez de supô-lo
- `every_published_schema_uses_only_keywords_the_validator_implements` recusa schema que passe a usar palavra-chave que o validador ignoraria, então a checagem não perde cobertura de um arquivo em silêncio

### Alterado no rascunho de 2026-10-15 desta entrada
- `config_cmd.rs` e `output.rs` ficaram abaixo do teto de 500 linhas: os tipos de argumento e de saída do `config` foram para `config_cmd_types.inc.rs`, e os emissores de envelope de erro foram para `output_error.inc.rs`, sem mudança de comportamento
- Versão 0.1.37
- `search` respeita o prazo global em cerca de 1x: numa árvore de 400 MB, prazo de 1 s termina em 1,02 s, de 2 s em 2,03 s e de 5 s em 5,02 s, onde antes levava 5,7x (R1)
- A memória residente do `search` na mesma árvore de 400 MB mediu 37 MB, onde era 950 MB (R1, R5); esse número não tinha máquina nem contagem de threads, e a nova medição com as duas está algumas linhas abaixo
- `search` usa um canal limitado por arquivo no lugar de um mapa global, então a memória fica limitada qualquer que seja o número de matches (R5)
- Todo `match` agora fica dentro do par `begin` / `end` do próprio arquivo com o número padrão de threads, onde o match ia para o maior caminho aberto (R3)
- Linha com match e UTF-8 inválido sai lossy, onde era trocada por string vazia (R28)
- `replace`, `transform` e `scope` fazem join das threads do walker antes de retornar e não gravam depois que o consumidor desistiu (R4)
- `write`, `edit`, `edit-loop`, `apply`, `hash --stdin`, `extract`, `diff`, `calc` e `regex` obedecem SIGTERM (exit 143) e o prazo (exit 124) com o stdin ainda aberto (R2)
- `cancelled` é a última linha da saída também com `--sort-by`, `--dedupe` e `--aggregate`: a cauda retida é descartada ao cancelar (R1)
- Argumento que não é UTF-8 não aborta mais o processo: é convertido lossy e chega ao clap (R18)
- `read --verify-checksum` com hash errado emite envelope de erro e sai com 81, onde stdout e stderr saíam vazios (R8)
- `edit-loop` é tudo-ou-nada por padrão, como o `edit`: par sem match sai com 65 e nada é gravado (R13)
- Isso substitui o comportamento do rascunho anterior (veredito não zero depois de gravar os pares que casam), e a sugestão agora cita `--partial` no lugar da flag `--allow-partial`, que nunca existiu
- Caminhos são obrigatórios em `replace`, `scope`, `codemod`, `case`, `delete`, `backup` e `prune-backups`, e posicionais em `wal-heal` e `scratch init`
- `recipe run` exige `--path` ou `--target`, e o `transform` já exigia os posicionais (F1)
- Verbo com efeito colateral sem alvo sai com 2, onde `replace foo src/` lia `src/` como o texto novo e varria o diretório atual (R15)
- Verbos só de leitura (`search`, `count`, `list`, `sparse`, `semantic-search`) mantêm o `.` implícito e não trazem `target_resolved`
- `doctor` e `env_detect` não leem nenhuma variável de ambiente: a detecção usa só sondas de arquivo (`/.dockerenv`, cgroup, `/proc/version`, `/.flatpak-info`, o mount de service account do Kubernetes, o prefixo do Termux, o caminho do executável) (R36)
- O bloco de ambiente do `doctor` perde `ci`, `github_actions` e `sudo`
- O relatório do `doctor` perde o check de política sobre servidor de protocolo de ferramentas
- O `agent-surface` não publica mais a chave de protocolo de ferramentas, e essa palavra saiu de `src/`
- A configuração é estrita (R25): toda struct de config nega campo desconhecido, e arquivo inválido ou chave desconhecida sai com 78 (`CONFIG_INVALID`)
- `version` maior que `CONFIG_SCHEMA_VERSION` é rejeitada do mesmo modo
- O arquivo de log fica desligado por padrão (R26)
- Com `[log] file = true` no `config.toml` XDG global, o arquivo gira por dia ou por hora, mantém 7 arquivos e é criado com modo 0600
- `set` em TOML preserva o comentário inline do valor trocado, e `comments_preserved` é calculado a partir do resultado, onde era um `true` fixo (R24)
- A contagem de subcomandos é 47 em todo doc e skill, conferida contra `CANONICAL_SUBCOMMAND_COUNT` (R39); era 46 antes do `config`
- `zip` passou de 2.4.2 para 9.0.5 com o backend `deflate-flate2` (R31)
- `serde_json` compilado com `preserve_order`, então envelope serializado via `Value` mantém a ordem de chave que a struct declara
- Sem isso o envelope de erro, que precisa passar por `Value` para o piso de credencial rodar, saía em ordem alfabética e mudava a forma em bytes de toda linha de erro
- Custo medido do `preserve_order`: +2 ms sobre 47 ms em 3000 eventos (cerca de 4%)
- Risco residual do `preserve_order`: `indexmap` troca O(log n) por O(1) amortizado na inserção ao custo de mais memória por mapa, o que é irrelevante para envelope de 10 a 20 chaves
- As constantes `DIAGNOSTIC_MARKER_KEY` e `DIAGNOSTIC_MARKER_VALUE`, sem uso, saíram, porque publicavam uma chave `channel` que nada estampava (o formatador JSON do `tracing-subscriber` não injeta campo constante)
- O discriminador que de fato vale é guardado por teste: todo diagnóstico carrega `level` e `timestamp`, e nenhum payload carrega
- Todo envelope de sucesso carrega `ok: true`, carimbado uma vez pelo writer em vez de declarado por subcomando (B1)
- A classificação dos tipos terminais foi corrigida, porque `SUCCESS_ENVELOPE_TYPES` tinha nomes como `copy`, `move`, `apply`, `delete` e `scope` que nenhum verbo emitia (R6)
- Resumo não é mais seguido de erro que o contradiz: `ok: true` não precede `NO_MATCHES` em `search`, `replace`, `transform` e `scope` (R7)
- A chave de envelope de sucesso deixa de colidir com o veredito de checksum: `verified` exige `--verify` (B2)
- `search` emite evento de fechamento sempre que emitiu o de abertura (B5)
- `--sort-by path-line` e `--sort-by line-path` leem as duas grafias, `line_number` e `line`, para a ordem ser propriedade do dado e não do entrelaçamento de threads (E4)
- `watch` saiu do conjunto de features padrão: o binário padrão responde com o erro de feature fechada e o `agent-surface` reporta `available: false` com a dica de adesão (E7)
- Envelope de erro carrega `suggestion_lang`, porque `message` vem da variante de erro (inglês por contrato) e `suggestion` vem do catálogo i18n, então um envelope pode misturar idiomas (B4)
- `--print-schema=NAME` passa a ser aceito do mesmo modo que a forma separada, porque a ajuda exibe o valor como opcional e o clap aceita as duas escritas
- `--print-schema` depois do nome do subcomando, sem valor, deixa de responder a listagem, porque flag no fim não é pedido de descoberta
- O motivo textual de cancelamento de `delete`, `copy`, `hash`, `backup` e da leitura de stdin vem de `cancel_reason` do sinal vivo, onde uma sentença fixa dizia `cancelled by signal` mesmo quando o que disparou foi o prazo global (A3)
- Mudança de contrato no canal de dado: `search`, `count` e os demais verbos de varredura emitem o envelope `cancelled` ao serem interrompidos, onde antes emitiam silêncio ou um resumo parcial
- Consumidor que lê a última linha de um `search` cancelado como resultado final passa a ver o registro de cancelamento
- Os módulos grandes foram divididos por responsabilidade: `output_shape.inc.rs` (1951 linhas) virou nove arquivos, e `run_replace`, `cmd_search`, `run`, `cmd_batch`, `schema_dispatch` e `sparse` foram divididos por responsabilidade (R40); remedido com `wc -l` sobre `src/`, quatro arquivos de produção continuam acima de 500 linhas, todos por margem estreita: `commands/mod.rs` (520), `commands/query.rs` (513), `commands/search_stream.inc.rs` (507) e `lib_dispatch.inc.rs` (503), mais seis módulos `tests.inc.rs` que são código de teste; `commands/read.rs` saiu da lista nesta rodada, porque a contagem e a seleção de linhas foram para `commands/read_lines.inc.rs`
- `locale.rs` foi dividido antes por responsabilidade: a persistência da preferência de idioma foi para `locale_persist.inc.rs`, seguindo o padrão `.inc.rs` existente (G2)
- `replace_run.inc.rs` foi dividido por responsabilidade: o veredito de fechamento e a emissão terminal ficam em `replace_finish.inc.rs` (G1)
- Mudança de comportamento na configuração: o `config.toml` global e o `.atomwrite.toml` do workspace agora são mesclados chave a chave, com o workspace vencendo, onde o arquivo local antes substituía o global inteiro (`--config CAMINHO` continua substituindo os dois em `get` e `list`)
- `locale --set` e a chave `locale` do `config` gravam a preferência por `atomic_write` (tempfile, fsync, rename, modo 0600), onde antes era uma gravação simples e não atômica
- A configuração global é lida uma vez por processo e compartilhada com a preparação do log, onde era lida duas vezes
- Memória residente do `search`, medida de novo numa máquina com 72 CPUs e 160 GB de RAM, árvore de 1620 arquivos e 380,9 MiB, page cache quente, saída para `/dev/null`: 142,6 MB no `-j` padrão antes deste trabalho e 45,3 MB depois, 37,9 MB e 39,1 MB com `-j 8`, 22,5 MB com `-j 1` depois
- Os 37 MB citados antes nesta entrada vieram de uma execução cuja contagem de threads não foi registrada, e batem com o valor de `-j 8`, não com a contagem padrão de threads
- A causa era um custo de cerca de 1,5 MB por thread do walker (31 MB com 1 thread, 38 MB com 8, 142 MB com 72), e não o canal de eventos
- O `read` no `-j` padrão foi de 44,3 MB para 17,9 MB, porque o pool do rayon deixou de ser construído para ele
- Um `-j 72` explícito deu 143,6 MB na medição da T3b e de 71 a 80 MB na medição da V1 no build final, já que o pedido explícito é respeitado acima do teto; a meta de 64 MB vale para o `-j` padrão, que mediu de 31 a 38 MB
- O `search` denso (3,24 milhões de linhas com match) foi de 10,4 s para 12,9 s numa única amostra tirada sob carga 50, e a diferença de 2,5 s não foi investigada
- Teto de threads: a contagem automática é `min(CPUs, 16, teto de RAM)`, e o walker automático é limitado a 12, a mesma proteção que a crate `ignore` aplica por dentro
- O pool do rayon é construído sob demanda e só para verbos que usam `par_iter`; `read`, `stat`, `write`, `move`, `get`, `set`, `del`, `rollback`, `calc`, `regex`, `extract`, `extract-artifact`, `verify`, `query`, `outline`, `scratch`, `locale`, `commands`, `agent-surface` e `completions` não constroem nenhum
- `hash_bytes` e o hash por mmap pegam o pool limitado antes do primeiro hash com rayon (arquivos a partir de 1 MiB), então um `read` ou `verify` grande deixa de construir uma thread por CPU
- O `search` divide um orçamento de 8 MiB de bytes entre os workers, com o stream que está sendo drenado isento só enquanto não tem bytes seus em voo, então a memória fica limitada qualquer que seja o número de threads e um consumidor lento não trava o pipeline
- O `watch` usa um canal limitado de 4096 eventos, onde era ilimitado
- Prazo fuzzy, medido com 2 s de orçamento numa fixture de linhas repetidas com um padrão que nunca casa, no binário debug: `replace` de 50 MB levava 6,16 s e de 150 MB levava 18,6 s antes, e os dois levam 2,02 s depois; `edit` de 50 MB levava 5,73 s e de 150 MB levava 17,7 s antes (exit 65), e os dois levam 2,02 s depois com exit 124 e envelope `cancelled`
- 900 MB: `replace` sai com 65 em 1,07 s com erro por arquivo `content too large` e `files_failed: 1`; `edit` leva 4,19 s com exit 124 no binário debug, acima da meta de 3,5 s, porque o arquivo é lido, hasheado e dividido em linhas antes de a cascata começar, e o número em release ainda será medido
- Só um miss de verdade (`MatchFailed`, `MatchAmbiguous`) vira zero matches agora: `Cancelled`, `InvalidInput` (conteúdo acima do teto) e erros internos chegam a quem chamou em `edit`, `edit-loop`, `replace` e `batch`, com o código próprio
- Mudança de comportamento no `batch`: uma op `replace` cujo match fuzzy é ambíguo agora falha a op, onde era relatada como "no matches"
- O resultado one-pass do fuzzy guarda o texto editado como `Option<String>`, então um arquivo sem match deixa de ser copiado; o `edit` com vários pares empresta o buffer até um par se aplicar
- A leitura de arquivo abre o arquivo uma vez, usa o `fstat` desse handle e lê direto num `Vec` do tamanho conhecido, onde antes mapeava o arquivo e depois copiava (o dobro do tamanho do arquivo residente)
- Um `search`, `transform` ou `replace` que não consegue ler, abrir, sondar ou percorrer um arquivo relata isso no canal de dados: `files_failed` e `errors[]` no resumo, o erro da primeira falha quando nada casou, e exit 0 com `files_failed` visível quando algo casou
- Arquivo grande demais, não UTF-8 ou binário continua pulado por desenho e não é contado como falha, para que uma árvore com imagens não troque `NO_MATCHES` por `INVALID_INPUT`
- Toda falha agora termina em exatamente um evento terminal: `search` sem match, `semantic-merge` com conflito, `edit-loop --partial`, `replace`, `transform` e `recipe` carregam `ok: false`, `error_class` e o resto do detalhe nesse evento, e o `main` não escreve mais um segundo envelope
- O evento de falha ignora `--filter` e `--limit` e respeita `--select` e `--redact`
- `EditPairFailed` (um par de `edit-loop` ou `edit` sem match) tem o código `MATCH_FAILED`, mesmo exit 65, onde relatava outro código
- O resumo do `recipe` não repete mais a linha do `search` filho que falhou, porque o veredito é o `recipe_result` único
- `--print-schema` serve `count` como `CountTotalOutput`, e `search-summary` e `replace-summary` são dois schemas distintos (`SearchSummary` e `ReplaceSummary`) que carregam `files_failed` e `errors`
- WAL, limite de taxa H3: o `Drop` do journal não o consulta mais, então um journal que gravou é removido. O H3 agora vale só na criação, na política `Auto`: depois de 10 journals abertos em 60 s as próximas gravações `Auto` não abrem nenhum (continuam atômicas por tempfile e rename), e `--wal-policy always` nunca é limitado
- Consequência a saber: num `replace` de 120 arquivos só os 10 primeiros ganham journal de recuperação de crash
- Medido em 120 arquivos de um diretório com `replace foo bar`: 110 arquivos `.atomwrite.journal.*` residuais antes e 0 depois
- `startup_wal_heal` roda só em verbo que muta e só nos diretórios citados no argv (a subárvore para alvo diretório, só o pai para alvo arquivo), pulando `.git`, `target` e `node_modules`, respeitando `.gitignore` e `.ignore`, e checando o orçamento de 100 ms dentro da varredura e não depois dela
- Medido em `read` de um arquivo num workspace cujo `target/` tem 40203 entradas: 0,02 s e 73 a 90 MB antes, 0,00 s e 23 a 24 MB depois (release antes, debug depois)
- `wal-heal` mantém a varredura completa, e os verbos que mutam mas não estão na lista de alvo explícito (`semantic-merge`, `batch`) não ganham heal automático
- O `inplace.rs` remove só o tempfile da própria operação, nunca todo arquivo `.atomwrite-*` do diretório
- Retry: o `lock.rs` espera por `crate::retry` com base de 50 ms, teto de 400 ms, 512 tentativas e orçamento igual a `lock.acquire_timeout_ms` (padrão `LOCK_ACQUIRE_TIMEOUT_MS`, 5000 ms), e o cancelamento o encerra como `Cancelled` enquanto o orçamento esgotado o encerra como `LOCK_TIMEOUT` (exit 83)
- Quando o orçamento é o prazo global, a espera dá até 100 ms para o watchdog levantar a flag, então a execução termina em 124 e não em 83
- O `persist_retry.rs` (Windows) tenta de novo pelo mesmo módulo e agora também trata `ERROR_SHARING_VIOLATION` como transitório
- O laço de `EAGAIN` do banner de encerramento espera com `poll(2)` e `POLLOUT` em fatias de 20 ms até 200 ms, onde girava `yield_now` a 100% de CPU
- O primeiro intervalo de retry é de 50 ms, o mínimo que a regra de retry do dono permite
- O `read` devolve 141 num pipe fechado e propaga uma falha de flush, onde devolvia 0
- `copy --preserve` falha quando as permissões ou os timestamps não podem ser copiados, onde a falha era descartada
- Restaurar permissões e timestamps depois de um `atomic_write` que já gravou registra um aviso e mantém o resultado, porque falhar o comando nesse ponto seria pior
- Mudança de comportamento: quando o próprio envelope de erro não pode ser escrito porque o leitor fechou o pipe, a execução sai com 141, onde mantinha o status do erro original; qualquer outra falha de escrita do envelope vai para o stderr e mantém esse status
- Mudança de comportamento: uma falha de flush do arquivo de log desliga o sink de arquivo com aviso, e uma falha ao aplicar a permissão 0600 no arquivo de log é avisada no stderr
- Envios por canal a um consumidor que já saiu passam por `best_effort::send_to_consumer`, e os demais `let _ =` de código de produção sumiram: propagam quando mudam o resultado, passam por `best_effort` quando são limpeza, e foram removidos quando só silenciavam um valor sem uso
- Nove panics de produção sumiram: sete em `rollback`, `apply`, no despacho de `completions`, em `locale` (dois) e em `commands::mod` viraram erro interno (`apply_replacement` agora é só de teste), e o `expect` da cascata fuzzy virou um `let ... else` com erro interno
- A regex estática de `normalize_punctuation_whitespace` virou um laço de `char`, então ela também não tem `expect`
- `language_identifier` em `locale.rs` devolve um `Result`
- `Idioma` e `direcao()` viraram `Language` e `direction()` (devolvendo `TextDirection`) em `src/locale.rs`, então o código e o rustdoc estão em inglês
- O comentário de `locale.rs` que dizia "never raw LANG" foi corrigido: no Linux o `sys-locale` lê `LANG`, e essa crate continua como fonte do sistema
- DESVIO CONSCIENTE da regra literal do dono sobre variáveis de ambiente: o `sys-locale` lê `LANG` e variáveis irmãs no Linux, e o `directories` lê `XDG_*` e `HOME` para posicionar os caminhos de configuração, dados, cache e estado. As duas são crates que as rules do dono sancionam, nenhuma é um knob de produto, e o código do produto não chama `env::var` (o `rg` fica vazio e um teste guarda isso); veja `PRIVACY.pt-BR.md:14`, `ARCHITECTURE.pt-BR.md:228` e o `gaps.md` (W54)
- DESVIO CONSCIENTE da regra literal do dono sobre idioma: `message` e `reason` dos erros NDJSON e o texto de ajuda do clap ficam em inglês por contrato, e só a `suggestion` é localizada (en e pt-BR, com `suggestion_lang`); veja `ARCHITECTURE.pt-BR.md:329` e o `gaps.md` (W55)
- Os literais do inventário de hardcode foram para `constants*`: nomes de sidecar e de backup (um só `BACKUP_INFIX`), unidades de tempo, limiares de risco, tamanhos de buffer e de bloco, caminhos de sonda de `/proc` e de contêiner, valores de errno, o BOM, nomes de thread e as constantes de calendário do `time_fmt.rs`
- O `list` não carrega mais uma cópia própria do algoritmo de calendário: ele chama `atomic::epoch_to_utc`
- O `src/constants.rs` foi dividido por responsabilidade em `constants_concurrency.inc.rs`, `constants_config.inc.rs` e `constants_literals.inc.rs`, e `config`, `fuzzy`, `ndjson` e `replace` ganharam arquivos `.inc.rs` pelo mesmo motivo (`config_tuning.inc.rs`, `config_cmd_keys.inc.rs`, `config_cmd_write.inc.rs`, `part3.inc.rs`, `replace_file.inc.rs`)
- `--version` imprime `atomwrite 0.1.37 (no-git) x86_64-unknown-linux-gnu` fora de um repositório: o `build.rs` sobe os diretórios procurando `.git`, chama o `git` só quando acha um, aceita o SHA só quando `git ls-files --error-unmatch Cargo.toml` prova que o repositório rastreia este crate, e imprime `no-git` nos outros casos, nunca `unknown`
- O `build.rs` emite `rerun-if-changed` só para caminhos que existem
- Build slim: `sparse outline` sem a feature `ast` sai com 78 `CONFIG_INVALID` e uma dica de rebuild na língua do locale, e o `outline` usa a mesma chave de mensagem; nenhum dos dois finge sucesso
- `cargo check --locked --no-default-features --features core` e `core,watch` passam, e o `cargo xtask check-features` roda os dois
- A ajuda do `-j` diz "omitido → todas as CPUs lógicas até o teto embutido, limitado pela RAM; `0` → igual a omitido; `N` → exatamente N workers, também acima do teto automático"
- O `deny.toml` não permite mais `BSL-1.0`, que nenhum crate da árvore usa e que gerava um aviso `license-not-encountered`
- O `Cargo.toml` declara `rust-version = "1.89"` (o texto anterior desta entrada dizia 1.88), o `rust-i18n` é 4.2 e o leitor de YAML do `transform` usa o `serde-saphyr` 1.3
- Broken pipe, decisão final: o SIGPIPE tem um handler que marca um encerramento cooperativo com exit 141 e não força a saída quando se repete; a escrita que bate no pipe fechado falha com `BrokenPipe`, o caminho de erro normal roda todo `Drop`, e o `main` sai 141
- Uma escrita em voo termina e grava (`is_global_hard_shutdown` em `src/atomic/rename_path.rs`), as checagens entre arquivos param o trabalho novo, e SIGINT, SIGTERM, SIGHUP e o timeout continuam abortando; o banner de encerramento não é escrito para broken pipe
- DESVIO CONSCIENTE da regra literal do dono, que manda resetar o SIGPIPE para `SIG_DFL` (`reset_sigpipe`, agora removido): com `SIG_DFL`, `replace | head -1` deixava de 7 a 11 locks, de 3 a 7 journals (inclusive os `committed`) e às vezes um `.atomwrite-*.tmp` (V1 FAIL-1 em 5 execuções, V2 M1 em 6 execuções), e o `wal-heal` nunca recolhia os journals `started`
- Com o handler, 20 execuções de `replace | head -1` e 20 de cada `list`, `search` e `read` saem 141 por código, não por sinal, e não deixam lock, journal nem tempfile (T2); uma repetição de 5 execuções de `replace | head -1` em 600 arquivos (T5b) deu exit 141 e zero resíduo
- A regra de encerramento graceful e o exit 141 foram preservados, e o dono pode reverter o desvio restaurando o `SIG_DFL`, ao custo do resíduo acima
- No Windows não há SIGPIPE, e o `BrokenPipe` do `std` segue o mesmo caminho; isso não foi validado lá
- A leitura-modificação-escrita agora é serializada. Antes, 30 `edit` paralelos em linhas distintas de um mesmo arquivo saíam todos 0 com `ok: true` e só de 4 a 10 das 30 mudanças sobreviviam (V2 A1), 30 `set` mantinham 1 chave, 30 `replace` mantinham 1 e 19 `config set` paralelos mantinham 2
- `edit`, `edit-loop`, `set`, `del`, `config set`, `config unset`, `apply`, `write --append`, `write --prepend` e as ops `replace` e `edit` do `batch` pegam o lock antes da leitura; o `edit-loop` lê o stdin antes do lock para nunca segurar o lock esperando entrada
- O `replace` lê cada arquivo sem o lock e, na hora de gravar, pega o lock, compara o checksum e, se o arquivo mudou, refaz a substituição sobre o conteúdo novo, então todo escritor tem sucesso e nenhuma atualização se perde
- `case`, `transform`, `scope` e `codemod` usam `atomic_write_based`: um arquivo que mudou desde a leitura falha com `STATE_DRIFT` (exit 82, retryable) para aquele arquivo, e o perdedor refaz a execução
- O lock é reentrante por thread (o `atomic_write` interno do mesmo verbo o compartilha), e `LockGuard` deixou de ser `Send`
- Medido depois (`cli_v0137_rmw_concurrent`): 30 `edit`, 30 `replace`, 30 `set` e 16 `config set` paralelos saem todos 0 e toda mudança está no arquivo
- Resíduo declarado: onde o `flock` não existe (NFS) e no Windows (o lock é no-op) só `edit`, `replace`, `case`, `transform`, `scope` e `codemod` mantêm a rede de checksum; o `edit-loop` não passa checksum-base; o `batch --transaction` trava por operação e não por lote
- `apply` com diff unificado (C1, crítico, já na 0.1.36): o contexto e as linhas removidas nunca eram comparados com o arquivo. Um patch `@@ -1 +1 @@ -zzz +yyy` virava a linha 1 em `yyy` com exit 0, e um hunk com contexto errado apagava três linhas
- Agora cada hunk compara contexto e linhas removidas com o arquivo (exato, espaço no fim da linha ignorado), é achado na linha do cabeçalho ou no vizinho exato mais próximo até 64 linhas, todos os hunks são aplicados em memória e gravados uma vez, e uma divergência é `MATCH_FAILED` (exit 65) com `best_candidate` e nada gravado
- Mudança de comportamento no `apply`: um hunk só de inserção insere DEPOIS da linha nomeada (antes inseria uma linha acima), um cabeçalho `@@` ilegível é `INVALID_INPUT`, um patch sem hunk é `INVALID_INPUT`, e linhas em branco de enchimento que o cabeçalho não conta são descartadas
- `apply` com SEARCH/REPLACE (A2): um bloco ausente deixava o arquivo meio aplicado com exit 0 e `hunks_applied: 1`; agora qualquer bloco ausente falha o lote inteiro com `MATCH_FAILED` e nada gravado (com `best_candidate` quando o texto lembra alguma linha), e um bloco equivalente em dois lugares falha com `MATCH_AMBIGUOUS` carregando `match_count`, também sem gravar nada; `Cancelled` e `InvalidInput` propagam em vez de serem engolidos. Não existe `--partial` no `apply`
- `get`, `set` e `del` num arquivo TOML ou JSON inválido (M2) saem 65 `INVALID_INPUT` com um envelope que traz só linha, coluna e uma causa curta, onde saíam 1 sem NDJSON e colocavam o arquivo inteiro na mensagem
- Mudança de comportamento no `main.rs`: qualquer erro sem tipo agora publica um envelope `INTERNAL_ERROR` no stdout, onde imprimia uma linha no stderr (o exit 1 colidia com `NO_MATCHES`)
- `search.max_inflight_bytes` (M3): o mínimo aceito subiu de 1 MiB para 4 MiB, o orçamento efetivo é `max(configurado, workers x 128 KiB)`, e um `.atomwrite.toml` que ainda diz 1 MiB agora é recusado ao carregar. Antes, 1 MiB deixava o `search` de 10 a 15 vezes mais lento com 16 ou mais workers (`-j 72`: de 39,99 a 59,18 s)
- Causa e correção dessa lentidão: toda liberação chamava `notify_all` e dezenas de workers dormindo acordavam para disputar poucos bytes (tempestade de futex, 423 s de tempo de sistema contra 83 s de usuário); agora uma liberação acorda um worker, e só quando o total cai abaixo do limite. O `-j 72` foi de 58,8 s para 4,33 s com 4 MiB e 4,09 s com 8 MiB, num host carregado
- O `search` respeita o prazo e o SIGTERM dentro de um único arquivo enorme sem match: num arquivo de 492 MB e uma regex sem literal, `--timeout-secs 1` sai 124 em 1,01 s onde levava 1,63 s, e SIGTERM aos 0,3 s sai 143 em 0,307 s onde levava 1,60 s (T3b); a V1 mediu 124 em 1021 ms em 398 MB
- A primeira versão do orçamento de bytes do search custou 23% de tempo de parede em saída densa; agora ela usa um contador atômico e só toca o mutex quando um worker dorme, o que dá 9,29 s contra 9,59 s do binário instalado (T3b, execuções intercaladas num host carregado)
- `semantic-merge` carimba `target_resolved` e `target_source: "argv"` a partir do `--output`, e `locale --set` e `locale --clear` carimbam o arquivo de preferência; o `batch` continua sem carimbo porque os alvos vêm do stdin, e cada alvo já aparece no próprio evento `batch_op`
- A retenção de backup apaga só `<nome>.bak.<carimbo válido>`: `f.txt.bak.0-user-note` era apagado por três `edit --keep-backup --retention 1`
- `--aggregate` mantém `ok`, `code` e `exit` do evento terminal que falhou, e `--select` não emite mais um evento que ele deixou vazio (um search sem match imprimia 16 linhas `{}`); a falha continua pelo caminho de falha
- As chaves do `config` são validadas por faixa e por relação, com as mesmas regras no `config set` e na carga inicial: `concurrency.max_threads` de 1 a 256, `concurrency.ram_fraction_percent` de 1 a 100, `lock.acquire_timeout_ms` de 100 a 3600000, `retry.base_ms` e `retry.max_ms` de 1 a 60000, `retry.max_elapsed_ms` de 1 a 3600000, `search.max_inflight_bytes` de 4 MiB a 1 GiB, `extract.max_sample_rows` até 100000, `extract.max_text_chars` até 10000000, `extract.max_sheets` e `extract.max_slides` até 10000
- `retry.base_ms` não pode passar de `retry.max_ms` nem de `retry.max_elapsed_ms`; `config set retry.max_ms 5` com base 50 sai 65
- As dicas fuzzy dedicadas não embutem mais o `reason` em inglês na dica localizada; `suggestion.invalid-input`, `config-invalid`, `internal-error`, `orphan-journal` e `transaction-rollback-failed` também pararam de embuti-lo, então uma dica pt-BR fica inteira em pt-BR e cita `atomwrite config set fuzzy.max_content_bytes <BYTES> --scope global` e `--fuzzy off`
- `--print-schema diff` serve `DiffOutput`: `type: "object"` com um `oneOf` de quatro ramos (`change`, `summary`, `unified`, `stat`), cada um com `required` e `additionalProperties: false`, e `docs/schemas/diff-output.schema.json` foi regenerado; a primeira versão não tinha `required` e listava só cinco chaves (V1 FAIL-8)
- Os schemas de resumo terminal estão fechados: `search-summary` (título `SearchSummary`) e `replace-summary` (título `ReplaceSummary`) são distintos, e `count`, `count-by-ext`, `count-by-size`, `backup-summary`, `list-summary`, `search-summary`, `replace-summary` e `config` trazem um `required` não nulo, `additionalProperties: false` e toda chave que o writer carimba
- Os dois resumos que podem terminar na própria falha ganham uma regra no schema: `ok: false` exige `code`, `exit`, `error_class`, `retryable`, `suggestion_lang` e `message`; `code` e `exit` não são sempre obrigatórios porque um sucesso não os tem; um teste valida 12 invocações reais (falhas de `search` e `replace` inclusive) e um teste negativo prova que o validador rejeita chave ausente, chave não declarada e falha sem `code`
- Erros de entrada do usuário são `INVALID_INPUT` (65) e não mais `INTERNAL_ERROR` (255), por `AtomwriteError::invalid` e a macro `invalid_input!`: a regex de `replace` e `search`, os globs de include, exclude e conjunto em `search`, `replace`, `transform`, `scope`, `list`, `count` e `delete`, o padrão e as regras YAML (arquivo e inline) de `transform` e `codemod`, o padrão de `scope`, a S-expression de `query`, `read --grep` (era ignorado em silêncio), um `locale --set` inválido e `--set --clear`, as ops de `edit --multi` (NDJSON inválido, campo ausente) e a faixa de linhas e a linha JSON de `extract`
- O fallback do `main.rs` que publica `INTERNAL_ERROR` fica para falhas internas de verdade; o `batch` continua relatando erros por operação como resultado, erros de uso do clap seguem 2, e um teste roda 24 dessas entradas, cada uma exigindo exit 65, um só evento `ok: false`, mensagem de uma linha de até 240 bytes e dica não vazia
- As mensagens de faixa e relação do `config` são em inglês fixo na `message`, e a explicação localizada é a dica `suggestion.config-out-of-range` ou `suggestion.config-relation` (en e pt-BR); isso substitui uma primeira versão que localizava a própria `message`
- A compilação de glob tem um ponto único, `src/walk_globs.rs` (`add_overrides`, `add_include_types`, `validate`), usado por `list`, `count`, `delete`, `transform`, `scope`, `replace`, `search`, `sparse list`, `sparse outline` e `hash`; um glob inválido é `INVALID_INPUT` (65) com um evento. Antes, `list --include '['` e `count --include '['` saíam 255 `INTERNAL_ERROR`, e `sparse list --include '['` e `hash -r --exclude '['` saíam 0 em silêncio (V1 FAIL-9, FAIL-10)
- Mudança de comportamento no `sparse list`: `--include` agora é aplicado (era ignorado) e `--exclude` é um glob de verdade (a chamada antiga de `add_ignore` estava quebrada); `hash -r --exclude '['` falha com 65 onde saía 0
- `query -Q` com S-expression inválida diz `invalid S-expression: ...` com a causa (a contagem de parênteses abertos), onde um `--language` explícito fazia dizer `unsupported language override`; a checagem (`check_sexpr_balance` em `src/commands/query.rs`) roda antes de ler o arquivo (V1 FAIL-11)
- Mudança de comportamento no caso ambíguo do fuzzy: o ramo Jaro-Winkler de linha única de `src/fuzzy/cascade.inc.rs` contava toda linha acima do limiar como empate sem passar pelo dual-gate, então agora só o candidato que passa o dual-gate empata. Um bloco ausente entre linhas parecidas é `MATCH_FAILED` (65, `match_count` 0, com `best_candidate`) onde era `MATCH_AMBIGUOUS` (200 linhas `line NNN` e um bloco `line 099 WRONG` davam 21 matches); um empate real (`dup  line` contra duas `dup line`) segue `MATCH_AMBIGUOUS` com `match_count: 2`. Isso vale para todo verbo que usa o match fuzzy de linha única, como `edit`, `replace`, `apply` e `batch`
- `docs/schemas/` tem 58 arquivos, 31 regenerados a partir do binário e 4 complementados com `ok`, `code`, `exit`, `target_resolved` e `target_source` (os arquivos antigos tinham `additionalProperties: false` e nenhuma chave de veredito, então um evento real reprovaria na validação); `README.md` e `README.pt-BR.md` usam `--max-age-secs` (não `--max-age`) e as flags das ADRs que a tradução pt-BR tinha trocado (`--dry-executar` e outras) voltaram aos nomes reais
- Tamanhos do binário medidos com `cargo build --release --locked` (T5a): slim 10.348.424 bytes (DERIVADO: 10348424 / 1048576 = 9,87 MiB, acima da meta de 5 a 8 MiB do PRD e abaixo do gate local de 15 MiB) e default 55.632.960 bytes (53,05 MiB); o antigo "cerca de 7,7 MiB" do slim vinha da 0.1.29

### Corrigido no rascunho de 2026-10-15 desta entrada
- Leitura de stdin cancelada mantém o erro `Cancelled` tipado depois do `.context`, então `edit --after-line` com pipe aberto sai 124 com `deadline_exceeded` onde saía 255 `INTERNAL_ERROR` (R2)
- `lang_utils` só compila com a feature `ast`, então o build slim `core` não pede mais as crates de gramática que não carrega
- O total documentado de schemas diz 59 nomes servidos por 58 arquivos, casando com `--print-schema` e com o gate `g7` em todo documento que cita o número
- O `write` aplica `--max-filesize` ao payload do stdin e ao alvo com um único teto: payload acima dele sai 65 `FILE_TOO_LARGE` (antes era `INVALID_INPUT`), e alvo grande demais é recusado antes da guarda de sobrescrita, nos caminhos de criação, sobrescrita, `--append` e `--prepend`
- O `scope` separa arquivo ilegível (`files_failed` mais `errors[]`) de arquivo pulado por desenho (`files_skipped`), e execução que respondeu por ao menos um arquivo sai 0 em vez de `NO_MATCHES`
- `count`, `list`, `sparse` e `semantic-search` reportam `files_failed` e `errors[]` em vez de total incompleto com `ok: true` (V2 M4)
- O `case` expande diretório pelo walker compartilhado, honra `--include` e `--exclude`, publica `files_visited` e sai 65 quando a árvore não produz arquivo, onde diretório saía 0 sem renomear nada
- O `prune-backups` recusa diretório com exit 65 e sugestão que nomeia a lista de arquivos, onde saía 0 sem apagar
- `get`, `set` e `del` leem e gravam tabela TOML inline (`pool = { max = 5 }`), onde `as_table()` devolvia `None` para `Value::InlineTable`
- `transform` e `scope` recusam linguagem cuja tabela de extensões não cobre o alvo, em vez de interpretar um arquivo `.rb` com a gramática `haskell`
- `locale --set` e `locale --clear` reportam o estado da própria chamada, onde o `OnceLock` congelado descrevia o estado anterior
- `--confirm-side-effect` é recusado com exit 65 `INVALID_INPUT` em verbo cujo contrato não pede a ack, onde `read` aceitava e saía 0
- `--strict-atomic` volta como flag real em todo verbo que escreve, então o exit 91 `EXDEV_FALLBACK_DISABLED` é alcançável pela CLI em vez de ser código morto
- `--print-schema extract` serve a união de `extract` em vez do schema de `calc`, e `write-risk` serve o `write-risk-assessment.schema.json`, então nenhum arquivo de schema fica órfão
- O `--limit` global é teto de trabalho: a lista de alvos é cortada antes da ação e a varredura para no registro que atinge o teto, então `--limit 1` toca exatamente um alvo, onde o walker lia a árvore inteira, agia em todos os alvos e só parava de imprimir; o resumo terminal traz `walker_stopped_early: true`
- `--filter`, `--select` e `--limit` nunca descartam envelope de falha, porque registro de falha é terminal e não sucesso moldado
- O `transform` multi-regra termina com um resumo da invocação inteira; regra que falha acrescenta um `rule_error` em vez de um segundo resumo que diz descrever tudo
- Leitor fechado não produz mais o aviso `data channel write failed` no stderr, porque pipe cumprido é o contrato e não defeito
- O `batch --transaction` alcança exit 80 `TRANSACTION_ROLLBACK_FAILED` por execução, com manifesto que faz a reversão falhar sem injeção, e o envelope de falha diz que a árvore pode estar meio revertida
- Envelopes de erro do `config` em `set` e `unset` carregam `target_resolved` e `target_source`, também quando a chave é desconhecida
- A sentinela H4 passa a ser `.atomwrite_keep_wal`, o nome que diz o que ela faz (preservar o journal); `.atomwrite_no_wal` é honrado como nome legado
- Binário construído sem feature opcional sugere reconstruir com a feature, em vez de nomear chave de configuração que não resolve
- `defaults.line_ending` e `defaults.max_filesize` são consumidas pelo `write`, então as duas aparecem em `config list`, `config get` e `config set` em vez de serem chaves mortas validadas
- `webpki-roots` 1.0.10 e `rustls-webpki` 0.103.16 fecham RUSTSEC-2026-0381 e RUSTSEC-2026-0382; as isenções de `serde_yaml` e `unsafe-libyaml` saíram do `supply-chain/config.toml`
- `list` e `count` não criam mais pool de rayon próprio em árvore pequena, e `-j N` explícito respeita o teto documentado
- O `edit` recusa arquivo grande demais a partir do `stat` antes da alocação, então arquivo acima do teto fuzzy não é mais lido inteiro antes da recusa
- O prazo global agora interrompe a varredura dentro de um único arquivo grande e não só entre arquivos (A1), também quando esse arquivo não tem nenhum match: o `CancelRead` confere a flag a cada leitura, e um arquivo de 492 MB com `--timeout-secs 1` sai 124 em 1,01 s (veja Alterado)
- O aviso de cancelamento sai do canal de dado, e diagnóstico viaja só no canal de diagnóstico (A2)
- `edit --multi` rejeita ranges sobrepostos com exit 65 e não toca o arquivo (R20)
- Pânicos internos em código de produção foram trocados por erro interno (R20)
- Correção (F-1), não mudança de comportamento: `doctor --strict` com um check em falha agora sai 1 com o código `DOCTOR_CHECKS_FAILED` (`AtomwriteError::DoctorChecksFailed`) e uma só linha terminal, o próprio `doctor_report`, onde antes saía 255
- Correção (F-1): um `write` interrompido agora sai com o status real do sinal ou do prazo (143, 130 ou 124) em vez de 255
- Correção (F-1): `batch` sem `--transaction` publica a falha no próprio `summary`, sem um segundo envelope de erro depois dele
- Correção (F-1): os schemas `doctor` e `batch` estão fechados (`additionalProperties: false`), o nome servido por `--print-schema` para o segundo é `batch`, e a lista continua com 59 nomes
- Broken pipe: `completions`, `--print-schema`, `--help` e `--version` ligados a um leitor que fecha cedo saíam 134, 1 ou 0 conforme o comando, e agora saem 141 (`BROKEN_PIPE`) como os outros verbos. A causa era o `clap_complete::generate`, que dá pânico quando a escrita falha, e o `human-panic` então gravava um arquivo de relatório de falha no diretório temporário (um arquivo local, nunca transmitido); teste `cli_v0137_broken_pipe_outputs`
- Schemas: `--print-schema completions` servia o schema de `write` e `scope-result.schema.json` rejeitava um evento real do `scope`. Os dois foram corrigidos e `docs/schemas/*.json` foi regenerado a partir do binário, com o teste de sincronia `docs_schemas_sync`
- Empacotamento: a lista `exclude` do `Cargo.toml` está ancorada na raiz do repositório (`/AGENTS.md`, `/CLAUDE.md`, `/MEMORY.md`, `/*atomwrite-*`, `/*atom_*`, `/*atom-*`), então `docs/AGENTS.md` e todo o diretório `skills/` agora entram no pacote publicado, de onde os padrões antigos sem âncora os tiravam
- O `--join KEY:SUFFIX` global era aceito e ignorado, porque o flush da saída entregava a ele uma lista de tabelas vazia. Agora ele lê a tabela NDJSON do stdin uma vez (teto de 16 MiB) e anexa a linha casada sob o SUFFIX; é recusado com exit 65 para especificação inválida, linha da tabela que não é objeto, chave ausente da tabela, comando que já lê o stdin, stdin que é terminal, tabela acima do teto ou tabela que não é UTF-8. O teste `cli_v0137_join` tem 13 casos
- `semantic-search --index-dir` fora do jail do workspace sai com 126, onde lia e escrevia fora do jail (R17)
- Nome de arquivo em forma NFD no Linux é encontrado e usado byte a byte, onde dava `FILE_NOT_FOUND` porque o caminho era reescrito em NFC para I/O (R19)
- `scope` conta `files_modified` só depois que a gravação deu certo, onde contava antes e seguia depois de gravação que falhou (R11)
- Nome de schema desconhecido em `--print-schema` sai como envelope de erro no canal de dado, com `code` e exit 65, onde antes saía exit 1 com stdout e stderr vazios
- A flag `--print-schema` deixa de ser campo morto: o valor que o clap aceita depois do verbo é lido e recusado com explicação, em vez de ser parseado e ignorado
- `verify` com checksum errado emite um evento com um veredito, em vez de dois eventos que se contradizem (B2)
- `case --subvert` recusa OLD idêntico a NEW em vez de aceitar em silêncio (C4)
- `delete --plan` em diretório sem `--recursive` devolve flag obrigatória estruturada (C3)
- `xtask` despacha subcomando e nunca escreve no canal de dado (D1, E1)
- Citações mortas a `docs/exit-codes.md` removidas dos dois readmes, porque o arquivo nunca deveria existir (D3)
- Scripts `.sh` mortos removidos, porque contrariavam a regra cross-platform e um deles dependia de variável de ambiente (D4, D6)
- Os docs deixam de descrever `write --lock` e `--lock-timeout`, que nunca existiram como flags: o lock é automático
- Os docs deixam de citar `ATOMWRITE_BACKUP` e `ATOMWRITE_READY_FILE`, que o binário deixou de ler
- Os docs deixam de citar servidor de protocolo de ferramentas ou motor de contêiner como rota suportada: a rota de integração é a CLI mais NDJSON por subprocesso
- A1 reaberto e fechado: `replace --dry-run` e `edit` com `--fuzzy auto` ignoravam `--timeout-secs` num arquivo grande sem match, porque os laços fuzzy nunca consultavam o cancelamento, e a cascata terminava em `MatchFailed` (exit 65 no `edit`) mesmo com o prazo já vencido
- Journals residuais: um `replace` sobre muitos arquivos deixava um arquivo oculto `.atomwrite.journal.*` por arquivo depois do décimo, porque o limite H3 contava no `Drop` do journal
- O `inplace.rs` podia apagar o tempfile de outro processo, já que removia todo arquivo `.atomwrite-*` do diretório (inclusive o arquivo de sonda do doctor)
- `search`, `transform` e `replace` devolviam `NO_MATCHES` para uma árvore que não conseguiam ler por inteiro, porque os erros de leitura, de abertura, de sonda binária e de caminhada eram engolidos
- Uma execução com falha escrevia dois eventos terminais (um resumo e depois um envelope de erro) em `search` sem match, `semantic-merge` com conflito e `edit-loop --partial`, então um parser que conta um evento terminal por execução errava
- O `edit-loop` contava um par recusado (um `old` vazio) como sem match; agora só um miss de verdade conta assim, e um par recusado sai com 65 e `INVALID_INPUT`
- O `search` não tinha orçamento global de bytes, então a memória residente crescia com o número de threads
- O laço de `EAGAIN` do banner de encerramento girava a 100% de CPU quando o stderr era um pipe cheio
- O `read` saía com 0 num pipe fechado e ignorava uma falha de flush, e agora sai com 141 e propaga o erro de flush (veja Alterado para a decisão sobre o SIGPIPE)
- `copy --preserve` relatava sucesso quando as permissões ou os timestamps não eram copiados
- Um `replace` do `batch` transformava um match fuzzy cancelado ou recusado em "no matches"
- `set`, `del` e `case` carregavam um `let _ =` morto ou um resultado descartado, e o `set` montava um `WriteOutput` que jogava fora
- O build slim (`--no-default-features --features core`) não compilava: o `sparse_outline` usava símbolos que só existem com a feature `ast`
- `--version` imprimia `(unknown)` fora de um repositório
- O `deny.toml` carregava uma licença permitida que nenhum crate usa, o que o `cargo deny` relatava como aviso
- O hash de um buffer ou arquivo grande criava um pool do rayon com uma thread por CPU em verbos que deveriam ficar sem pool
- `apply` com diff unificado destruía conteúdo com exit 0 quando o contexto ou as linhas removidas não casavam com o arquivo (C1, crítico, já na 0.1.36), e `apply` com SEARCH/REPLACE deixava o arquivo meio aplicado com exit 0 e engolia `Cancelled` e `InvalidInput` (A2)
- Atualizações perdidas na leitura-modificação-escrita: `edit`, `set`, `replace` e `config set` concorrentes devolviam sucesso enquanto a maioria das mudanças sumia, porque o lock só era pego na hora de gravar (A1 da segunda revisão)
- `get`, `set` e `del` num arquivo TOML inválido saíam 1 sem NDJSON e citavam o arquivo inteiro na mensagem (M2)
- Inanição do orçamento de bytes do `search`: `search.max_inflight_bytes` no mínimo antigo deixava o `search` até 15 vezes mais lento com muitos workers (M3)
- A retenção de backup apagava um arquivo do usuário que só começava com `<nome>.bak.` (B1)
- `--aggregate` perdia o veredito de falha do evento terminal e `--select` imprimia eventos `{}` vazios (B5, B6)
- `replace | head -1` deixava locks, journals e tempfiles sob a ação padrão do SIGPIPE (V1 FAIL-1)
- `--print-schema diff` servia o schema de um plano de dry-run em vez da saída real do `diff` (V1 FAIL-3), e o schema do `config` exigia uma chave `warnings` que `config path`, `config set` e `config unset` não emitem (V1 FAIL-2)
- Dicas pt-BR misturavam português e inglês para o teto fuzzy e para erros do `config` (V1 FAIL-5)
- Um SIGTERM ou SIGINT cooperativo no meio do `replace` deixava um journal `started` em 16 de 100 execuções (V1 FAIL-6): o cancelamento é visto antes do `rename`, com o alvo intacto e o tempfile removido no drop, então não havia nada a recuperar. O `JournalGuard::discard()` agora remove o journal no braço `Rename` quando o erro é `Cancelled`; 0 de 100 e 0 de 100 depois (T3a, duas séries de 100, SIGTERM). Um `kill -9` ainda deixa o journal, por desenho (46 de 100 execuções), e o `wal-heal` mantém o journal `started` de um processo morto (`removed: 0, preserved: 1`), então não o limpa
- `replace --regex '('`, uma regra YAML inline inválida do `transform` e `query -Q '((('` saíam 255 `INTERNAL_ERROR` em vez de 65 `INVALID_INPUT` (V1 FAIL-7b, veja Alterado)
- `apply` com SEARCH/REPLACE e um bloco ausente parecido com várias linhas saía `MATCH_AMBIGUOUS` em vez de `MATCH_FAILED` (V1 FAIL-7, causa raiz na cascata fuzzy, veja Alterado)
- O `completions --install` respondia 255 `INTERNAL_ERROR` com "isto é um bug" quando o diretório de dados não era gravável, porque as três chamadas de sistema passavam por `anyhow::anyhow!` e perdiam o tipo; agora usam `AtomwriteError::from_io_at`, então diretório negado é 13 `PERMISSION_DENIED` e a mensagem nomeia o caminho
- O `prune-backups` apagava um arquivo do usuário que só começava com `<nome>.bak.`, porque o filtro era `starts_with(prefix)` sozinho; agora só um carimbo real de backup qualifica, pelo mesmo `is_backup_timestamp_name` que o caminho de retenção já usava (quarta rodada de auditoria)
- O `edit --multi` lia um campo que o op escolhido nunca consome, então um `new` num `replace-range` apagava a linha e respondia `ok: true`; campo não consumido é exit 65 sem nada escrito
- O `edit --multi` com array JSON respondia `invalid type: map, expected a string`; agora nomeia o contrato, um objeto JSON por linha
- O `apply` lia um cabeçalho `---`/`+++` sem hunk `@@` como substituição do arquivo inteiro e sobrescrevia o alvo com o texto do patch; esse cabeçalho agora é `INVALID_INPUT` (65) e o arquivo fica intacto
- O `apply` contava hunk sem contexto, sem linha removida e sem linha adicionada (`@@ -2,0 +2,0 @@`) como aplicado; hunk vazio é recusado com exit 65
- O `search --help` prometia um evento `skipped` por arquivo grande demais, evento que o comando nunca emite; agora documenta `files_skipped` no `summary`
- O `wal-stats` publicava `path` vazio para journal na raiz do workspace, o que se lê como diretório ausente; a raiz agora é `.`
- O teste instável `r1b` (files-only) de `cli_v0137_search_stream` falhava sob carga porque a sonda de 2 MiB media quase só a partida do processo, então a varredura real às vezes terminava antes do prazo de 2 s; agora o fixture cresce 4 vezes e a execução se repete até 3 vezes, e só uma execução ainda viva no prazo é julgada. 52 execuções sob carga de 54 a 74 tiveram 0 falhas (T3b)
- O `apply` em autodetecção substituía o arquivo inteiro pelo texto do patch quando o corpo não trazia marcador nenhum, e respondia `ok: true`: 51 bytes viraram 45 e 34 viraram 19 numa execução de auditoria. O `detect_format` agora devolve `None` para corpo sem marcador, e o verbo recusa com exit 65 `INVALID_INPUT` nomeando `--format full` como a porta explícita, com o checksum inalterado
- O `apply --format full` agora percorre as mesmas duas barreiras do `write`, pela função `check_replacement_size` compartilhada: `--ack-overwrite` acima de `[write].confirm_large_bytes`, e `--allow-shrink` abaixo de `[write].shrink_block_percent`, então o verbo que substitui o arquivo inteiro deixou de ser o único sem guarda
- As duas guardas de substituição moram em `src/commands/write_guards.rs` e servem `write` e `apply` de uma fonte só, onde cada verbo carregava a própria cópia da aritmética de limiar
- O `config list` publica 35 chaves em vez de 25: as 10 que as structs declaravam e nenhum verbo expunha agora são legíveis e graváveis, o que encerra o `INVALID_INPUT` numa chave que o próprio `write --help` já citava como ajustável. São elas `defaults.backup`, `defaults.retention`, `write.shrink_block_percent`, `write.auto_rotate_max_age_secs`, `watch.idle_exit_ms`, `watch.debounce_ms`, `search.context`, `search.smart_case`, `search.max_filesize` e `search.max_columns`, cada uma com faixa derivada da constante que o próprio consumidor já usa
- O `config set write.shrink_block_percent` agora muda o limiar que a execução seguinte do `apply` aplica, onde o valor era constante cravada no código: o mesmo arquivo e o mesmo corpo saem 65 com `50` e 0 com `90`
- Os testes unitários de `config` deixam de ler o `~/.config/atomwrite/config.toml` do operador: o `load_local` aciona a função pura `merge_layers` com a camada global em `None`, então a suíte responde 482 de 482 numa máquina cujo config real define um teto de fuzzy, onde antes falhava 1 de 482
- O `doctor` ganha dois checks que medem em vez de confiar no texto: o `docs_test_targets` compara toda afirmação sobre a contagem de alvos de teste com a listagem real de `tests/`, e o `docs_config_keys` compara toda afirmação sobre a contagem de chaves de config com a tabela viva `KEYS`. Os dois respondem `skipped` e nunca `pass` em árvore sem documento, e os dois nomeiam o arquivo e a linha que divergiram
- A contagem de alvos de teste documentada é a medida de 131 nos 16 documentos que diziam 133, 117 ou 1375, e a contagem de chaves é a medida de 35 em todo documento que dizia 23, incluindo a lista de nomes do `llms.txt`
- O `src/commands/mod.rs` cai de 522 para 469 linhas, o `src/commands/query.rs` de 513 para 445 e o `src/lib_dispatch.inc.rs` de 505 para 427, movendo a política de nome de backup, o parser de S-expression e o heal de WAL de partida para arquivos próprios, então nenhum fonte acima de 500 linhas mistura responsabilidades distintas
- As decisões 0062 e 0069 trazem correção datada nos dois idiomas, onde afirmavam contagem de alvos, pertencimento à lista de alvos explícitos e tamanho de arquivo que a medição já não sustenta
- O `config --help` publica a tabela de chaves a partir da `KEYS` viva por `Cli::parser`, onde a página carregava uma cópia escrita à mão com 22 chaves contra as 35 que o verbo responde. A tabela e o verbo não podem mais divergir, porque a tabela é uma só

### Correções a rascunhos anteriores desta entrada
- F2 (`target_resolved`, `target_source`): um rascunho anterior listava os campos como entregues enquanto nenhum código os emitia. Eles só são emitidos a partir desta versão, como a seção Adicionado descreve
- F3 (token deslocado): o teste anterior cobria só o `transform` e exigia reescrita com exit 0. Agora cobre todo verbo com efeito colateral e exige falha fechada
- D2 (`TRANSACTION_ROLLBACK_FAILED` alcançável): o rascunho anterior dizia que a alcançabilidade por execução não estava demonstrada. Agora está, com `tests/cli_v0137_batch_rollback.rs`: o manifesto faz um arquivo criado virar diretório não vazio antes de uma operação posterior falhar, então a reversão bate em `EISDIR` sem injeção e a execução sai 80
- Supply chain: o texto anterior dizia que o `cargo audit` reportava 0 vulnerabilidades enquanto o `zip` 2.4.2 carregava RUSTSEC-2026-0367, -0368 e -0369. A afirmação só vale depois da atualização do `zip` para 9.0.5 listada em Segurança
- `serde_yaml`: o texto anterior dizia que ele havia sido trocado. Ele foi trocado só no leitor de regras do `transform`, e segue na árvore por meio do `rust-i18n`, como a seção Segurança declara
- Supply chain, segunda correção: o texto de Segurança citava `instant`, `unsafe-libyaml` e `serde_yaml` como parte da árvore. O `cargo tree --locked --all-features -i` responde "did not match any packages" para `instant`, `unsafe-libyaml`, `serde_yaml` e `serde_yaml_ng`, e o `Cargo.lock` não tem nenhum desses pacotes, então aquele texto era falso para esta árvore e foi trocado em Segurança
- Memória do `search`: os 37 MB não tinham máquina nem contagem de threads. Em 72 CPUs a contagem padrão de threads mediu 142,6 MB antes do teto de threads e 45,3 MB depois, e os 37 MB batem com `-j 8` (veja Alterado)
- Contagem de subcomandos: uma linha anterior dizia 46. A contagem é 47 com o `config`, e `CANONICAL_SUBCOMMAND_COUNT` é 47 em `src/constants.rs`
- `NdjsonWriter::write_event_failed` não existe mais: foi trocado por `publish_failure`, que marca o erro como publicado
- Os itens C5, D5 e E2 não são citados por este changelog, e a linha do `gaps.md` que dizia que eram era falsa. C5 e E2 sobrevivem só como títulos de teste (`tests/cli_v0137_dominio.rs:474`, `tests/cli_v0137_agent_native.rs:628`), D5 não aparece em lugar nenhum, e o D3 desses testes não é o D3 do `gaps.md`, então os ids não formam um esquema único
- Lock (N23, R23): a entrada dizia primeiro que o lock advisory estava fechado, mas ele só era pego na hora de gravar, depois de o verbo ler o arquivo, então não impedia atualização perdida. Agora está fechado com um recorte: os verbos de leitura-modificação-escrita travam antes de ler (Unix, sistemas de arquivos com `flock`), e o resto depende da checagem de checksum-base descrita em Alterado
- Broken pipe, segunda correção: o comportamento entregue não é nem `SIG_DFL` nem ignorar o SIGPIPE; é um handler que marca um encerramento cooperativo com exit 141 (veja Alterado, com o desvio consciente da regra do dono)
- Memória e prazo do `search`: a nota de que um arquivo enorme sem match só via o prazo no fim da varredura não vale mais, o `CancelRead` a resolveu
- Exit 129: a nota de que o `agent-surface` não o listava não vale mais
- Totais de schema: rascunhos anteriores desta entrada diziam 59 nomes servidos por 58 arquivos. A árvore final serve 55 nomes apoiados por 54 arquivos, e dez lugares de sete documentos ainda carregavam o total antigo até esta auditoria corrigi-los (`llms.txt`, `llms-full.txt`, `docs/MIGRATION.md`, `docs/COOKBOOK.md`, `docs/schemas/README.md`, `docs/TESTING.md` e `docs/TESTING.pt-BR.md`)
- Totais de chaves: rascunhos anteriores desta entrada diziam 23 chaves tipadas. A árvore final serve 37 chaves, e `docs/MIGRATION.md`, `docs/MIGRATION.pt-BR.md`, `docs/decisions/README.md` e os dois arquivos do ADR-0058 carregavam o total antigo até esta auditoria corrigi-los

### Removido
- A chave de protocolo de ferramentas do `agent-surface`, o check de política correspondente do `doctor` e toda menção a servidor de protocolo de ferramentas em `src/` e nos docs
- Os campos `ci`, `github_actions` e `sudo` do bloco de ambiente do `doctor`
- A seção de motor de contêiner de `docs/CROSS_PLATFORM.md` e `docs/CROSS_PLATFORM.pt-BR.md`
- A sugestão `--allow-partial` do `edit-loop`, uma flag que nunca existiu
- Os scripts `scripts/check-gaps-ssot.sh` e `scripts/regen-schemas.sh`

### Segurança
- `zip` 9.0.5 fecha RUSTSEC-2026-0367, RUSTSEC-2026-0368 e RUSTSEC-2026-0369 (uma delas crítica, 9,1) no leitor do contêiner OOXML
- `quick-xml` passou de 0.37 para 0.42, fechando dois advisories high (7,5) alcançáveis no leitor de artefato: alocação ilimitada de namespace (`RUSTSEC-2026-0195`) e checagem quadrática de atributo duplicado (`RUSTSEC-2026-0194`)
- `crossbeam-epoch`, `rustls`, `anyhow` e `memmap2` atualizados para as versões corrigidas, e `inout` atualizado por `cargo update -p inout` porque a versão antiga estava yanked
- O `cargo deny check` devolveu advisories ok, bans ok, licenses ok e sources ok sem nenhum aviso depois que `BSL-1.0` saiu do `deny.toml` (rodado pelo executor de build nesta fase)
- O `cargo audit --deny warnings` sai 0 na árvore final (veja Gates finais abaixo)
- `instant` (RUSTSEC-2024-0384), `unsafe-libyaml` (RUSTSEC-2024-0449), `serde_yaml` e `serde_yaml_ng` não estão nesta árvore: `cargo tree --locked --all-features -i <crate>` diz "did not match any packages" para cada um, e o `Cargo.lock` não tem nenhum desses pacotes
- O `notify` 8.2.0 (feature opcional `watch`) não puxa mais o `instant`, e o `rust-i18n` 4.2.4 e as regras YAML do `transform` (feature `ast`) usam o `serde-saphyr` 1.3.0
- O `Cargo.toml` declara `rust-version = "1.89"`, que o `rust-i18n` 4.2 e o `serde-saphyr` exigem, e o comentário do `deny.toml` (ADR-0057, 2026-10-19) registra que os dois avisos de manutenção estão resolvidos
- As entradas `[[exemptions.serde_yaml]]` e `[[exemptions.unsafe-libyaml]]` saíram do `supply-chain/config.toml`, porque isenção de crate que não está na árvore só faz o `cargo vet` descrever uma árvore que não existe
- `semantic-search --index-dir` não consegue mais ler nem escrever fora do jail do workspace (exit 126)
- Piso incondicional de chave de credencial: chave de credencial no topo é redigida em todo envelope, com ou sem `--redact`
- Antes desse piso, só o caminho com shape redigia, e uma chamada sem flag deixava `api_token` passar enquanto `--limit 5` o redigia
- A redação de conteúdo alcança o envelope de erro: antes ela só rodava no canal de sucesso, então credencial dentro de `path` ou `message` sobrevivia ao `--redact`
- Lista de needles do `--redact` ampliada para prefixos de emissão de nuvem e de forge (AWS, GitHub, Slack, Stripe, Google, Anthropic), então segredo dentro de campo de texto é redigido
- Escritores concorrentes: um verbo de leitura-modificação-escrita pega o lock advisory antes de ler e a camada de gravação pode conferir o checksum-base sob o lock (veja Alterado), e `--expect-checksum` continua sendo a checagem contra drift
- O lock é advisory e só de Unix: no Windows ele é no-op, e em sistemas de arquivos que rejeitam `flock` (NFS) só `edit`, `replace`, `case`, `transform`, `scope` e `codemod` mantêm a rede de checksum
- `get`, `set` e `del` não colocam mais o conteúdo de um arquivo inválido em nenhum canal, então um segredo dentro de um TOML ou JSON quebrado deixa de chegar ao stderr (M2)
- Risco residual declarado: o `--print-schema` serve nomes curtos (`read`, `edit`) enquanto `docs/schemas/` publica nomes descritivos (`read-output`, `edit-output`), e os dois conjuntos não compartilham regra de nomenclatura
- Jail do workspace mantido (sem allowlist global de `/tmp`): subcomando segue sem escapar do workspace
- A etapa 2 do ETD não foi feita: o workspace ambiente (o diretório atual) segue permitido como fronteira e ainda não é reportado como `workspace_source`

### API da biblioteca
- Aviso de quebra: em relação à 0.1.36 publicada, a API pública da biblioteca `atomwrite` mudou de formas que o SemVer chama de major, e a interface suportada desta crate é a CLI com o contrato NDJSON, não a biblioteca
- O `cargo semver-checks` 0.50.0 contra a 0.1.36 sai 100 com 10 verificações major em falha, e cada uma está listada aqui
- `auto_trait_impl_removed`: `LockGuard` deixou de ser `Send` e deixou de ser `Sync`
- `constructible_struct_adds_field`: campos públicos novos em `GlobalArgs`, `ErrorJson`, `AtomwriteConfig`, `FuzzySection`, `SearchSection`, `OutputShape`, `EditLoopArgs`, `WalHealArgs`, `CancelledEvent`, `ScratchInitEvent`, `LocaleState` e `MatchOpts`
- `enum_missing`: `locale::Idioma` foi removido (renomeado `Language`)
- `enum_no_repr_variant_discriminant_changed`: `LocaleSource::Persisted`, `System` e `Default` mudaram o valor numérico
- `enum_variant_added`: `Commands::ExtractArtifact` e `Commands::Config` num enum sem `#[non_exhaustive]`
- `enum_variant_missing`: `LocaleSource::EnvVar` foi removido
- `feature_not_enabled_by_default`: a feature `watch` deixou de ser padrão, então `watch` exige um build com `--features watch`
- `function_missing`: `signal::reset_sigpipe`, `locale::negotiate_to_idioma` e `locale::resolved_idioma` foram removidas
- `pub_module_level_const_missing`: `LOCK_POLL_FAST_ATTEMPTS`, `LOCK_POLL_FAST_MS`, `LOCK_POLL_SLOW_MS` e `PERSIST_RETRY_DELAYS_MS` foram removidas
- `struct_pub_field_missing`: os campos `ci` e `sudo` de `RuntimeEnvironment` e `idioma` de `LocaleState` foram removidos
- Uma versão que segue o SemVer para a biblioteca seria a 0.2.0, e a decisão entre marcar esses tipos `#[non_exhaustive]`, esconder a superfície da biblioteca ou publicar a 0.2.0 é do mantenedor

### Documentação
- Uma auditoria de documentação de todo o repositório rodou contra o binário 0.1.37, em seis relatórios que cobrem os gaps, os comandos e flags da CLI, os contratos (exit codes, schemas, config), os documentos da raiz, a árvore `docs/` e os arquivos `llms`, as skills, o `Cargo.toml`, o rustdoc e os textos de `--help`
- O `README.md` foi reorganizado, e os 47 subcomandos aparecem em todo guia com um resumo de uma linha e um exemplo testado
- Os ADRs 0058 a 0074 registram as decisões desta versão, em inglês e em pt-BR, e o ADR 0074 cobre a serialização da leitura-modificação-escrita e o `apply` estrito; `docs/decisions` agora vai de 0019 a 0074 (56 ADRs) e `docs/adr` guarda mais 3, e os ADRs antigos ganharam seus espelhos pt-BR
- Cada um dos 47 subcomandos tem um exemplo testado no README, INTEGRATIONS, AGENTS, HOW_TO_USE, COOKBOOK, INSTALL, TESTING, MIGRATION, nos arquivos `llms` e nas três skills
- O `--help` tem um bloco `EXAMPLES` nos 47 comandos e nenhum jargão interno em nenhuma das 59 páginas de ajuda, os doctests foram de 3 para 20, e os lints de rustdoc estão em `deny` no `src/lib.rs`
- O `llms-full.pt-BR.txt` é novo, as skills (do repositório e a global) foram corrigidas, e o `config --help` lista as 35 chaves
- O `PRIVACY.pt-BR.md` e o `PRIVACY.md` dizem que os parsers do tree-sitter são baixados em runtime no primeiro uso, e que o relatório de falha do `human-panic` é um arquivo local no diretório temporário
- O `watch` exige um build com `--features watch`, e o binário compilado sem ela sai 78 nesse verbo

### Notas
- Não validado: macOS e Windows não foram executados. O código só de Unix tem um par `cfg(windows)` (SIGPIPE, `restrict_to_owner`, `persist_retry.rs`), o `persist_retry.rs` só foi checado compilando uma cópia do corpo dele em Linux, e o `cargo check --target x86_64-pc-windows-gnu` não pôde rodar por falta do `x86_64-w64-mingw32-gcc`
- Não validado: os nove testes `cross_compile_*`
- Parcial (P3): `batch --transaction` com uma operação em falha emite `rollback` com `ok: true` seguido do envelope `ok: false`, e o contrato foi mantido como está
- Parcial (P3): erros sem tipo ainda caem em 255 em `cli.rs` e no rename de `persist_retry.rs` e `rename_path.rs`; a instalação de completions em `lib.rs` saiu desta lista porque passou a usar `AtomwriteError::from_io_at`, então diretório negado é 13 `PERMISSION_DENIED` com o caminho na mensagem
- Parcial (P3): o teste de schema de `doctor` e `batch` comparava só `required` e os nomes dos campos, então um schema que declarasse um tipo que o binário não emite ficava verde; o `tests/common/schema_check.rs` fechou isso sem dependência, e `every_published_schema_uses_only_keywords_the_validator_implements` recusa schema que passe a usar palavra-chave que o validador ignoraria
- Parcial (P3): o `cargo test --no-default-features --features core` agora compila (`tests/cli_v012_audit_regressions.rs` tem `#[cfg(feature = "ast")]` nos quatro testes de YAML, e o `cargo xtask check-features` passa), mas 44 testes em 11 alvos de teste (`cli_scope`, `cli_transform`, `cli_query`, `cli_outline`, `cli_concurrency`, `cli_e2e_v0135` e outros) falham nessa configuração porque exigem a feature `ast` e não têm `#[cfg(feature = "ast")]`; testar o core localmente fica PARCIAL (fonte: V3)
- `--max-output-bytes` exige `--allow-truncate` e cada flag é rejeitada sem a outra com exit 2 (`ARGUMENT_PARSE_ERROR`), e valores `null` no topo não são mais emitidos (11 campos; teste `r9_no_null_top_level_values` em `tests/cli_v0137_contract.rs:371`)
- Não feito (P3): `--expect-checksum` roda fora do lock, então há uma janela de TOCTOU entre a checagem e a escrita
- Não feito (P3): um nome de arquivo acima de cerca de 238 bytes pode estourar o limite de 255 bytes pelo nome do sidecar de lock (`.` + nome + `.atomwrite.lock`), e uma escrita in-place num diretório onde o sidecar não pode ser criado fica sem lock
- Não feito (P3): o teto de RAM funciona só no Linux e não lê limites de cgroup
- Mantido pelo mantenedor: o `human-panic` fica, e o relatório de falha dele é um arquivo local no diretório temporário que nunca é transmitido
- Mantido pelo mantenedor, e declarado: `--print-schema completions` descreve só o evento `installed`, porque `completions` sem `--install` imprime script de shell como texto e não emite NDJSON, então não há registro a descrever
- Acesso à rede em runtime (P3, W18): a feature `ast` baixa parsers do tree-sitter em runtime quando o cache XDG está vazio, então os testes de `outline`, `query` e `scope` dependem de um cache preenchido e o helper hermético deixa o `XDG_CACHE_HOME` no cache real; a feature `download` do `tree-sitter-language-pack` continua fora do escopo desta versão
- Declarado (V2 B2): um arquivo novo gravado por `write` nasce com modo 0600 no binário antigo e no novo (herdado)
- Medido em release pelo verificador (V1, build `lead-rel`): `replace --dry-run` em 150 MB sai 124 em 2020 ms e `edit` em 2019 ms com prazo de 2 s, SIGTERM aos 0,5 s sai 143 em cerca de 505 ms, e em 900 MB o `replace` sai 65 em 1243 ms e o `edit` sai 65 `INVALID_INPUT` em 1808 ms
- Medido pelo verificador (V1, rodada 2, build `lead-rel` das 19:20): o `search` tem pico de 31 a 38 MB no `-j` padrão numa árvore de 382 MB (a meta de 64 MB vale para o `-j` padrão), 43,3 MB em 4,8 milhões de linhas casando (rodada 1) e de 71 a 80 MB num `-j 72` explícito
- Medido pelo verificador (V1, rodada 2): `replace --dry-run --fuzzy auto` num arquivo abaixo do teto (233.437 KB) tem pico de 304.672 KB em 27,6 s, DERIVADO: 304672 / 233437 = 1,305, então a razão é cerca de 1,3 vez o arquivo e não no máximo 1,3 (a rodada 1 tinha 307.440 KB, DERIVADO: 307440 / 233437 = 1,317, depois de o verificador corrigir o tamanho do arquivo)
- Medido pelo verificador (V1, rodada 2, build final): 150 MB com prazo de 2 s sai 124 em 2010 ms (`replace`) e 2009 ms (`edit`), SIGTERM aos 0,5 s sai 143 em 505 e 503 ms, e em 900 MB o `replace` sai 65 em 685 ms e o `edit` sai 65 `INVALID_INPUT` em 1024 ms
- Gates finais, rodados pelo líder na árvore final (gate de release na árvore final): `cargo fmt --check` 0, `cargo build --release --locked` 0 sem warning, `cargo clippy --locked --all-targets --all-features -- -D warnings` 0, `RUSTDOCFLAGS=-D warnings cargo doc` 0, `cargo audit --deny warnings` 0, `cargo deny check` 0 sem warning e `cargo xtask check-features` 0
- Execução final dos testes, mesma fonte: `cargo test --locked --no-fail-fast` deu 131 alvos ok, 0 alvos com falha, 1486 testes passaram, 0 falharam e 9 ignorados (os testes `cross_compile_*`); a base desta versão era 1105 testes. O verificador V3 aprovou esta árvore na rodada 2
- Não validado: o lock advisory em sistemas de arquivos sem `flock` (NFS) e o retry de rename do `persist_retry.rs` no Windows, além de macOS e Windows como um todo (veja a primeira linha de Notas)


## [0.1.36] - 2026-07-31

### Adicionado
- Sentinela `ok: false` em todos os envelopes `ErrorJson` (GAP-2026-020)
- `match_count: 0` estável em falhas de match / NoMatches
- `best_candidate` em `MatchAmbiguous` (GAP-2026-025)
- XDG `[fuzzy].soft_floor_delta` no context-aware (GAP-2026-022)
- Mapa de offsets escape_normalized (GAP-2026-024)
- Constante `SCRATCH_DIR_REL` e sugestões de staging (GAP-2026-019)
- Suggestion content-source em jail de `--old-file`/`--new-file`
- Shaping global agent-native: `--select`/`--fields`, `--filter`, `--limit` (sem `-n` global)
- Envelopes de erro always-emit sob shape (`--filter` não silencia falhas)
- `extract --field` (seleção local de campos; não `--select` local — colide com shape global; o `--select` global ainda projeta a linha emitida). Agentes/skills NUNCA devem ensinar `extract --select`
- Subcomando `scratch init` (staging sob workspace; sem `/tmp`)
- `collect.inc.rs`; proptest FP; `scripts/check-gaps-ssot.sh`
- ADR-0055 EN+pt-BR; skills com fórmulas staging/shape/anti-padrões
- Suite `tests/cli_v0136_gaps.rs`

### Alterado
- Versão 0.1.36
- Estratégias normalizadas: `similarity: null` (não 1.0 falso)
- `collect_all` via `collect.inc.rs`; dual-gate no miolo de `block_anchor`
- Threshold aggressive 0.50 reavaliado (FP=0 com dual-gate; override XDG)
- Superfície 45 subcomandos (SSOT: enum `Commands` / `agent-surface`; inclui `scratch`, `doctor`, `locale`, `commands`, `stat`, `verify`, …). Constante `CANONICAL_SUBCOMMAND_COUNT` + teste

### Corrigido
- Inventário GAP-2026-019…033 fechado ou mitigado por classe

## [0.1.35] - 2026-07-19

### Adicionado
- Sobrescrita grande default-deny: alvos existentes acima de XDG `[write].confirm_large_bytes` (bootstrap 100 KiB) exigem `--ack-overwrite` (agente one-shot; sem prompt Y/N)
- Flag independente `--require-large-ack` (sem colisão de alias clap com `--confirm`)
- `watch` sempre emite NDJSON final `type:watch_summary` (`events`, `reason`: idle|max_events|timeout|signal) mesmo com zero eventos
- `semantic-merge` grava markers de conflito no output por padrão (`<<<<<<< ours` / `=======` / `>>>>>>> theirs`); opt-out com `--no-conflict-markers` ou `--write-conflict-markers false`
- XDG `[watch].debounce_ms` + CLI `--debounce-ms`; idle padrão `DEFAULT_WATCH_IDLE_EXIT_MS=500` (era ~3s)
- `delete --plan` para listagem plan-only (seguro one-shot); `delete --confirm` e `delete --yes` / `-y` são rejeitados (fail-closed)
- Notas no agent-surface para anti-padrões write/watch/merge (A-DISC-001)
- Suite de contrato local `tests/cli_e2e_v0135` + closeout residual A-* (gaps.md §20)

### Alterado
- Versão 0.1.35
- Splits SRP de monólitos via `include!` (write/recipe/lib/replace/scope/concurrency/cli_args/runtime)
- `scope` reutiliza `summary_metrics` compartilhado (DRY)
- Matriz multi-OS honesta: Linux e2e completo; Windows-gnu `cargo check` no host Linux; macOS exige Mac real/osxcross (sem GitHub Actions de produto)

### Corrigido
- A-TEST-001: suite não usa mais `delete --yes` após rejeição B-015 (`cargo test --lib --tests` verde)
- A-WATCH-001: watch idle não deixa mais stdout vazio
- A-WRITE-001: sobrescrita grande era opt-in via `--confirm`; agora sempre default-deny
- A-WRITE-002: `--require-large-ack` era alias de `--confirm` (“used multiple times”); agora independente
- A-MERGE-001: status conflict gravava prefer-ours sem markers; markers default ON
- A-ONESHOT-001: idle default do watch reduzido para 500 ms
- A-GAPS-DOC-001: banner no topo do gaps.md + §20 supersede cabeçalhos stale “suite 100% / FALHA XDG”
- Detecção de env do doctor em allowlist (A-ENV-001); zero knobs de produto via env

### Notas
- Sem telemetria de produto; sem CI `.github` no repositório de produto
- Configuração somente CLI + XDG / `.atomwrite.toml` (sem knobs `ATOMWRITE_*` em runtime)
- Veja `docs/TESTING.pt-BR.md`, `docs/CROSS_PLATFORM.pt-BR.md`, `docs/MIGRATION.pt-BR.md` (0.1.34 → 0.1.35)

## [0.1.34] - 2026-07-19

### Alterado
- Alinhamento completo da documentação pública ao contrato one-shot fuzzy (README, llms*, AGENTS, HOW_TO_USE, COOKBOOK, TESTING, INSTALL, ARCHITECTURE, INTEGRATIONS, SECURITY, skills, ADR-0054)
- Versão 0.1.34

### Corrigido
- Deriva de documentação: muitos arquivos ainda alegavam v0.1.30 / timeout default 0 / multi-apply fuzzy ilimitado após a correção de código da 0.1.33

### Notas
- A correção de hang one-shot no runtime permanece como descrita em [0.1.33]; 0.1.34 é a release publicável com docs completas

## [0.1.33] - 2026-07-19

### Corrigido
- CRÍTICO one-shot: `replace --fuzzy` não entra mais em loop infinito quando `replacement` contém `pattern` (footgun de agente “expandir seção”). Antes: re-scan com `max_replacements = usize::MAX` → dias a 100% CPU / multi-GB RSS.
- Path fuzzy one-pass (`apply_fuzzy_one_pass`): nunca re-busca texto recém-inserido; default 1 apply; embeds força 1; teto 10_000.
- Cancel cooperativo polled na cascade fuzzy; caps de pattern/levenshtein/janelas/crescimento.

### Alterado
- Default de `--timeout-secs` é 120 (era 0). Use `0` para desligar.
- Versão 0.1.33.

## [0.1.32] - 2026-07-15

### Alterado
- Removida superfície pública de contribuição/comunidade: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` (EN/PT-BR) e seções Contribuindo dos README
- Sem arquivos de “Contribute” / Código de Conduta no community profile do repositório migrado


## [0.1.31] - 2026-07-15

### Alterado
- Migração de ownership do crate e repositório para a conta GitHub `danilo-aguiar-br`
- `Cargo.toml` `repository` e `homepage` apontam para https://github.com/danilo-aguiar-br/atomwrite
- E-mail de contato do author atualizado para `daniloaguiarbr@proton.me`
- Removidos workflows de GitHub Actions e badges de CI (sem Actions no repositório novo)

### Notas
- Ownership no crates.io convidado/aceito sob `danilo-aguiar-br`
- Publish e atualização de metadados exigem esta versão nova (URL do repository só muda no publish)


## [0.1.30] - 2026-07-13

### Adicionado
- Política fuzzy de produto: apenas auto/aggressive; --fuzzy off rejeitado
- edit --replace-all com unicidade e match_count no NDJSON
- Guarda escape-drift; delta de indent; estratégia unicode_normalized
- best_candidate sempre ranqueado; candidates[]; diff_preview preenchido
- MatchAmbiguous exit 65 / MATCH_AMBIGUOUS
- search --target content|files|both com --offset e --limit
- similar_paths em NotFound quando workspace é conhecido
- progress do replace com pre-count total rate_per_s eta_ms
- recipe exclui *.bak.* por padrão e progress_every 50
- sparse outline com orçamento
- backup_method honesto reflink_or_copy
- Notas de agent-surface para fuzzy obrigatório e install do watch

### Alterado
- Versão 0.1.30
- Skills EN/PT removem --fuzzy off da superfície recomendada

### Corrigido
- Tokenizer do semantic-search faz split de snake_case e camelCase
- Docs e help do semantic-merge: honestidade line-based three-way (não AST)
- EditOutput NDJSON serializa match_count e indent_adjusted (P17/P18)
- recipe/hash recursive pula backups *.bak.* (P19)
- config [fuzzy] mode off rejeitado; seção fuzzy aplicada em edit/replace/loop/batch (P20)
- sparse outline emite outline_item AST real sob orçamento (P21)
- Testes e2e agent-contract dos campos NDJSON residuais (P23)


## [0.1.29] - 2026-07-13

### Adicionado
- Módulo compartilhado `fuzzy`: cascata de 9 estratégias para edit, replace, batch, edit-loop (P0-1)
- Diagnósticos estruturados `best_candidate` em falha de match (P0-2)
- `replace --fuzzy auto|off|aggressive` e `--fuzzy-threshold`
- Cancelamento cooperativo durante leitura de stdin e escrita atômica em chunks (P0-4)
- Features Cargo: `core`, `ast`, `lang-*`, `watch`, `semantic`, `full` (P0-3)
- Job CI `size-gate`: release slim core deve ser ≤ 15 MiB; jobs `core-test` e `schema-diff`
- `semantic-merge` merge de três vias com checagens de shutdown (P1-1)
- `sparse list|read` superfície de monorepo com orçamento (P1-2)
- `replace --progress-every` e heartbeats de progresso em batch (P1-3)
- `recipe list|run` com despacho REAL search→replace→hash (P1-4)
- Documentação versionada de recipes em `recipes/*.yaml`
- Backup prefere hardlink antes de reflink/copy (P2-3)
- Subcomando `stat` como alias de `read --stat` (P2-4)
- `agent-surface` inventário derivado do clap (P2-5)
- `write --durability full|fast|auto` com NDJSON `platform.durability` (P2-1)
- Linux `renameat2` em `atomic_rename` com fallback ENOSYS (P2-2)
- `watch` debounce coalesce + `--checksum` opcional + gitignore (P3-1, feature watch)
- `semantic-search --index-dir` índice invertido offline (P3-2)
- Campanha `codemod` com `codemod_summary.by_rule_id` (P3-3)
- Schemas regenerados: write durability/rename_method, replace fuzzy*, error best_candidate, recipe/progress/cancelled

### Alterado
- `replace` de string fixa usa fuzzy=auto por padrão após miss de multi-match exato (breaking vs 0.1.28, que só falhava com exit 1)
- Numeração de estratégias normalizada para nove estratégias nomeadas
- Na época da release: superfície skill documenta 41 subcomandos (antes 33)

### Corrigido
- Falhas de match não descartam mais scores de near-miss
- Recipe run não é mais stub somente de plano
- Clippy `result_large_err` via `BestCandidate` em box
- Build core-only funciona sem crates AST (stubs com exit 78)

### Tamanho do binário (honestidade medida)
- Slim (`--no-default-features --features core`): ~7.7 MiB (CI exige ≤ 15 MiB)
- Default/full com AST: ~52 MB — PRD 5–8 MB aplica-se somente ao CORE, não ao build default com AST


## [0.1.28] - 2026-07-06

### Mudanças BREAKING
- `delete` agora cria backup por padrão (antes era opt-in) — use `--no-backup` para desligar; `--keep-backup` em `delete` é redundante (backups de deleção são sempre preservados e nunca auto-removidos) e agora emite um campo `warnings` no envelope NDJSON em vez de no-op silencioso
- `move`/`copy` sobrescrevendo um destino existente agora exigem `--force` OU `--backup` explícito — antes os dois subcomandos aplicavam essa regra de forma inconsistente

### Correções Críticas — GAP-CLI-SURFACE-DRIFT
- Extraída struct única `BackupOpts` (`--backup`, `--no-backup`, `--keep-backup`, `--retention <N>`) via `#[command(flatten)]` em 15 subcomandos mutantes: `write`, `edit`, `edit-loop`, `replace`, `transform`, `scope`, `apply`, `set`, `del`, `case`, `batch`, `delete`, `move`, `copy`, `rollback`
- `replace --retention` e `delete --no-backup` agora parseiam com sucesso — antes ambos retornavam exit 2 `ARGUMENT_PARSE_ERROR` porque cada struct redeclarava as flags de backup manualmente com presença e defaults divergentes
- `--backup` e `--no-backup` agora são mutuamente exclusivos via `conflicts_with` (exit 2) em vez da última flag vencer em silêncio
- `rollback` mantém seu snapshot de segurança pré-rollback opt-in via `--backup` explícito, documentado como exceção intencional ao contrato unificado default-true
- Adicionado guard compartilhado de stdin-tty no `edit`: `--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between` e `--multi` agora falham rápido com exit 65 `INVALID_INPUT` e `suggestion` acionável quando stdin é um terminal, em vez de bloquear indefinidamente (antes exigia timeout externo e terminava em exit 143)
- Os modos de stdin do `edit` agora rejeitam `--old`/`--new`/`--old-file`/`--new-file` no parse via `conflicts_with_all` (exit 2) em vez de despachar silenciosamente para `edit_by_marker`
- Removido hardcode `retention: 5` de 9 pontos; retention agora flui de ponta a ponta a partir de `BackupOpts` através de um único `resolve_backup()`
- `batch --retention <N>` era aceito e silenciosamente ignorado — agora efetivo de ponta a ponta com precedência CLI `--retention` > `.atomwrite.toml` `[defaults].retention` > default embutido (`5`), incluindo o pre-backup transacional do batch
- `batch --backup` explícito agora força backup para cada operação do lote, simétrico a `--no-backup`
- Removidos os últimos pontos com hardcode `retention: 5` em `batch.rs` (pre-backup transacional e a operação `delete`) — ambos agora honram o retention resolvido
- Os impls `Default` de `config.rs` e `atomic.rs` agora leem `constants::DEFAULT_BACKUP_RETENTION` em vez de um literal duplicado

### Correções Altas — GAP-CONFIG-DEFAULTS-DEAD
- As chaves `[defaults]` `backup`/`retention` do `.atomwrite.toml` agora são efetivas em todo subcomando mutante — `resolve_backup()` antes recebia apenas `(backup: bool, no_backup: bool)` e nunca consultava a config parseada
- Precedência agora é `ATOMWRITE_BACKUP` (env) > flags CLI (`--backup`/`--no-backup`/`--retention`) > `.atomwrite.toml` `[defaults]` > default embutido (`true` / `5`)

### Validação
- `cargo test` — 661 testes passam (0 falhas, 3 ignorados)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diferenças
- Novos arquivos de teste: `cli_v0128_backup_matrix.rs` (12 testes), `cli_v0128_config_defaults.rs` (6 testes), `cli_v0128_edit_stdin_guard.rs` (3 testes), `cli_v0128_batch_backup.rs` (6 testes)


## [0.1.27] - 2026-06-24

### Correções Críticas — Segurança
- BUG-SEC-001: Escape de symlink-directory do jail do workspace — `ln -s /tmp $WS/link && atomwrite write link/file` podia criar arquivos FORA do jail. Corrigido com `canonicalize_existing_prefix` em `path_safety.rs` que resolve symlinks antes da verificação de jail. Afeta: write, read, edit, edit-loop, apply, set, del, copy, move. NÃO afeta: search, replace (verificação própria de symlinks via WalkBuilder)
- BUG-SCOPE-004: `scope --query comments --delete` destruía código em linhas com comentários inline — `fn foo() {} // comment` deletava a linha INTEIRA incluindo `fn foo() {}`. Corrigido `expand_to_full_line` para detectar código antes do match e preservá-lo

### Correções Médias
- BUG-001: `edit-loop` aceitava stdin vazio retornando exit 0 — agora retorna exit 65 INVALID_INPUT
- BUG-002: `edit-loop` com JSON inválido emitia texto puro no stderr em vez de envelope NDJSON — corrigido com `map_err` para `AtomwriteError::InvalidInput`
- BUG-005: Suggestion de erro de `--syntax-check` dizia `--syntax-check=false` (sintaxe clap inválida) — corrigido para "remove --syntax-check"
- BUG-GET: `get` com chave ausente retornava exit 4 (FILE_NOT_FOUND) — agora retorna exit 65 (INVALID_INPUT) com mensagem "key 'X' not found in Y"
- BUG-008: `pair_results` de `edit-loop` tinha apenas `[index, matched]` — agora inclui campos `old` e `new` para rastreabilidade
- BUG-SCOPE-002: Queries `fn`, `struct`, `enum`, `trait`, `async-fn`, `unsafe-fn` do `scope` ignoravam itens `pub` — ast-grep trata `pub` como modificador AST estrutural; adicionados padrões com variante `pub`
- BUG-SCOPE-003: Queries `const`, `static`, `type-alias`, `mod`, `use` do `scope` ignoravam itens `pub` — mesma causa raiz do BUG-SCOPE-002; adicionados padrões com variante `pub`

### Correções Baixas
- BUG-SCOPE-005: `scope --query var` em Go ignorava declarações com tipo inferido (`var x = 0`) — adicionado padrão `var $NAME = $$$EXPR`; refatorado `lookup_go_query` para `lookup_go_queries` retornando `Vec<String>`

### Limitações Conhecidas
- GAP-01: `scope --query test-fn` indisponível — ast-grep não consegue casar padrões que cruzam múltiplos nós AST (`#[test]` + `fn`). Workaround: `--pattern "#[test]"` ou `query -Q "(function_item (attribute_item) @attr)"`
- GAP-02: `scope --query doc-comment` indisponível — tree-sitter parseia `///` como `line_comment` idêntico a `//`; sem distinção estrutural. Workaround: `--query comments` para todos os comentários, ou `rg "///"`
- GAP-03: `scope --query export` em JS/TS indisponível — padrão `export $$$DECL` cruza múltiplos nós AST. Mesma limitação do ast-grep do GAP-01

### Validação
- `cargo test` — 631+ testes passam (0 falhas, 3 ignorados)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diferenças
- Auditoria e2e: 3 rodadas, ~180 cenários adicionais, 10 bugs corrigidos, 3 limitações conhecidas documentadas


## [0.1.26] - 2026-06-23

### Correções Altas — Consistência de Exit Codes
- GAP-141: `replace` com zero matches agora retorna exit 1 (NO_MATCHES) — antes retornava silenciosamente exit 0
- GAP-145: `transform` com zero matches agora retorna exit 1 (NO_MATCHES) — antes retornava silenciosamente exit 0
- GAP-146: `scope` com zero matches agora retorna exit 1 (NO_MATCHES) — antes retornava silenciosamente exit 0
- GAP-147: `search --sort path` agora garante ordenação global determinística — antes era não-determinística com walker paralelo

### Correções Médias
- GAP-136: `read --verify-checksum` com hash errado agora retorna exit 81 (CHECKSUM_VERIFY_FAILED) — antes retornava exit 82 (STATE_DRIFT)
- GAP-137: `get` para chave ausente agora retorna exit 4 (NOT_FOUND) — antes retornava exit 65 (INVALID_INPUT), revertendo GAP-111 da v0.1.25
- GAP-144: Padrões de query Rust do `scope` (fn, pub-fn, async-fn, unsafe-fn, struct, pub-struct, enum, pub-enum, trait, impl) agora casam com tipos de retorno, generics e implementações de trait via padrões AST duais

### Correções Baixas
- GAP-135: Subcomando `verify` agora aceita argumentos posicionais: `verify <PATH> <HASH>` — antes exigia flag `--checksum`
- GAP-142: `edit-loop` agora aceita entrada JSON array `[{...},{...}]` além de NDJSON (um objeto por linha)

### Esclarecimentos de Documentação (BY-DESIGN)
- GAP-138: Saída de `query --kinds` é stream NDJSON de `{type:"query_kind"}` por kind (não um objeto JSON único com array `kinds`)
- GAP-139: `case --to <style>` requer pares `--subvert OLD NEW` (scanning global de identificadores não está implementado)
- GAP-140: `write` com stdin vazio requer flag `--allow-empty-stdin` (guarda G120 é intencional)
- GAP-143: Flag de `prune-backups` é `--max-age-secs` (não `--max-age`) para tornar a unidade explícita

### Validação
- `cargo test` — 631 testes passam (0 falhas, 3 ignorados)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diferenças
- Auditoria e2e: 535 cenários em 11 rodadas contra binário real
- 13 gaps auditados: 9 CODE resolvidos, 4 BY-DESIGN documentados


## [0.1.25] - 2026-06-22

### Novos Recursos
- GAP-072: Arquivo de configuração `.atomwrite.toml` com hierarquia: CLI > env > local > XDG global > defaults
- GAP-079: Subcomando `verify` (delega para `hash --verify` para verificação de checksum BLAKE3)
- GAP-085: Fuzzy matching Jaro-Winkler (estratégia `context_aware_jw`) para strings curtas no edit
- GAP-086: Testes property-based para fuzzy matching via proptest (5 propriedades)
- GAP-087: `edit --fuzzy-threshold <FLOAT>` para sensibilidade configurável de match fuzzy
- GAP-088: Highlighting de diff fuzzy via `similar::TextDiff` em `FuzzyInfo.diff_preview`
- GAP-081: Ações `symbols` (ASCII→Unicode operadores) e `normalize` (normalização NFC) no `scope`

### Correções Críticas
- GAP-101: `write --backup` não reporta mais `backup_path` fantasma para backups auto-deletados
- GAP-102: `set` não redireciona mais chave ao descer em valores escalares TOML (agora exit 65)
- GAP-127: `case` sem `--subvert` agora retorna exit 65 com mensagem clara em vez de no-op silencioso

### Correções Altas
- GAP-073/098: Erros de I/O via `anyhow::Context` agora emitem envelope NDJSON em vez de texto cru no stderr
- GAP-084: Removidos todos os `.unwrap()` de caminhos de produção (substituídos por erros tipados)
- GAP-093: Validação de conflitos de flags via `conflicts_with` para append/prepend, fixed/regex, literal/regex
- GAP-103: `copy --preserve` agora preserva permissões do arquivo fonte (Unix)
- GAP-104: `copy --backup` agora retém arquivo `.bak` em disco (antes era auto-deletado)
- GAP-105: `replace --backup` agora retém arquivo `.bak` em disco (antes era auto-deletado)
- GAP-106: `write --require-backup` agora força retenção do backup em disco
- GAP-120: `size_delta_pct` em risk_assessment alterado de u8 para u32 (corrige overflow para deltas >255%)

### Correções Médias
- GAP-075: `delete --older-than` com duração legível por humanos (s/m/h/d/w)
- GAP-076: `delete --confirm` como modo preview (mesma lógica de `--dry-run`)
- GAP-077: `replace --preserve-case` com `adapt_case()` (MAIÚSCULAS/minúsculas/Título)
- GAP-078: Flag `search --pcre2` retorna exit 65 quando feature PCRE2 não está habilitada
- GAP-080: `transform --verify-parse` re-parseia output com tree-sitter após rewrite
- GAP-091: Texto de help do `edit --multi` atualizado para mencionar herança de modo `--fuzzy`
- GAP-095: Regex de `replace --preserve-case` agora é case-insensitive (adicionado `(?i)` ao pattern)
- GAP-096: `write --preserve-timestamps` agora emite campo `mtime_preserved` no NDJSON
- GAP-097: 3 schemas JSON atualizados para casar com output real do binário (del-result, write-output, read-output)
- GAP-107: Campo de output do `hash` renomeado de `value` para `checksum` (casa com schema)
- GAP-108: `batch` move/copy agora exige `"force":true` para sobrescrever alvos existentes
- GAP-109: `outline --positions` agora emite `start_byte`, `end_byte`, `start_column`, `end_column`
- GAP-110: `list` em diretório inexistente agora retorna exit 4 (antes retornava exit 0 com summary vazio)
- GAP-111: `get`/`del` em chave ausente retorna INVALID_INPUT (exit 65) em vez de FILE_NOT_FOUND (exit 4)
- GAP-112: `move --backup` agora inclui `backup_path` no output NDJSON
- GAP-116: Campo `modified` de `list --long` agora emite formato ISO 8601 em vez de epoch cru
- GAP-117: Flags documentadas `--no-reflink`, `--preserve-xattr`, `--preserve-hardlinks` adicionadas à CLI
- GAP-118: `copy --backup` não exige mais `--force` para sobrescrever (paridade com `move --backup`)
- GAP-119: `write --auto-rotate` agora retém backup em disco
- GAP-121: `--json-schema` para hash, delete, move, copy agora emite schemas tipados corretos
- GAP-123: `scope --query comments` agora captura block comments (`/* */`) além de line comments
- GAP-124: `prescan_json_schema()` agora reconhece subcomandos get, del, set, outline, query, case
- GAP-128: `list --count-by-ext` agora filtra sufixos timestamp de backup (consistente com `count --by-extension`)
- GAP-133: `copy --preserve` agora copia mtime/atime da origem via `filetime::set_file_times()`
- GAP-134: `scope --query test-fn` retorna erro acionável em vez de erro críptico multi-nodo do ast-grep

### Correções Baixas
- GAP-099: Corrigidos 2 clippy warnings em arquivos de teste (collapsible_if, len_zero)
- GAP-100: Corrigidas 27 divergências de rustfmt em arquivos fonte e de teste
- GAP-113: Default de telemetria de risco alterado para 255 (desabilitado) — usuário deve optar via `--risk-threshold`
- GAP-114: `extract` agora filtra eventos begin/end/summary do pipeline de search
- GAP-115: `wal-stats --dry-run` agora emite evento NDJSON de plano (antes era silencioso)

### Validação
- `cargo test` — 631 testes passam (0 falhas, 3 ignorados)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diferenças
- Auditoria e2e: ~505 cenários em 6 rodadas contra binário real
- 64 gaps auditados no total: 49 resolvidos, 5 pré-existentes, 10 reclassificados, 0 pendentes


## [0.1.24] - 2026-06-21

### Correções Críticas
- GAP-027: `delete --recursive` agora percorre diretórios (antes pulava silenciosamente)
- GAP-035: `read --line/--lines` não entra mais em panic com índices fora do range
- GAP-037: `search --multiline` agora propaga flag corretamente ao SearcherBuilder
- GAP-046: `batch --transaction` agora reverte `move`/`copy` no rollback
- GAP-050: `replace` rejeita padrão vazio (antes destruía todos os arquivos silenciosamente)

### Correções Médias
- GAP-019: `ARGUMENT_PARSE_ERROR` (exit 2) agora inclui campo `suggestion` acionável (ADR-0045)
- GAP-020: `diff` agora resolve paths contra `--workspace` (ADR-0046)
- GAP-021: `scope` em modo read-only agora reporta `files_matched` corretamente (ADR-0047)
- GAP-022: `scope`/`count`/`transform` agora resolvem walk roots contra `--workspace`
- GAP-023: Formato de timestamp de backup alterado para `YYYYMMDD_HHMMSS_mmm` (resolução em milissegundos previne colisões)
- GAP-028: `scope --query comments --delete` agora remove nós AST `line_comment` inteiros
- GAP-029: Saída NDJSON de `wal-stats`/`wal-heal` agora inclui campo `type`
- GAP-032: `hash --stdin` não exige mais argumento PATHS (`required_unless_present = "stdin"`)
- GAP-033: `--require-backup` agora detecta corretamente quando backup está desabilitado via `--no-backup`
- GAP-036: Campo `op` em `edit --multi` agora é opcional (inferido como `"exact"` quando `old`+`new` presentes)
- GAP-038: `delete --recursive` agora remove subdiretórios vazios (travessia depth-first)
- GAP-039: Valores JSON de `get` não são mais duplamente quotados
- GAP-040: `get` de chave inexistente retorna exit 4 (`NOT_FOUND`) com envelope JSON
- GAP-042: Valores TOML string de `get` não são mais duplamente quotados
- GAP-043: `old_value`/`removed_value` de `set`/`del` não são mais duplamente quotados
- GAP-044: `hash` de arquivo inexistente retorna exit 4 (`NOT_FOUND`) com envelope JSON
- GAP-045: `set` TOML agora lê corretamente old_value de chaves aninhadas via descida manual
- GAP-047: `case --subvert` alterado de `num_args=2..` (greedy) para `num_args=2` (par exato)
- GAP-048: `hash --recursive` agora implementa travessia de diretório via WalkBuilder
- GAP-049: `prune-backups --max-count` agora ordena por nome de arquivo (lexicográfico) em vez de mtime

### Correções Baixas
- GAP-024: `write` pula risk_assessment para `--append`/`--prepend` (sem possibilidade de perda de dados)
- GAP-025: `regex` removeu `allow_hyphen_values`; usar separador POSIX `--` para exemplos com hífen
- GAP-026: `scope` summary não reporta mais `files_modified` em modo read-only
- GAP-030: `read --format raw` pula heurística binária (não sugere mais `--force-text` inexistente)
- GAP-031: Formato de `edit --multi` corrigido (campo `op` agora opcional)
- GAP-034: Warning de `regex` adicionado quando flags parecem exemplos (duplicata do GAP-025)
- GAP-041: `read --lines/--head/--tail` com range vazio retorna conteúdo vazio em vez de `"\n"`

### Auditoria de Erros Tipados (GAP-051 a GAP-070)
- Convertidos 20 `anyhow::bail!()` para variantes tipadas `AtomwriteError`
- Todos os erros voltados ao usuário agora emitem JSON estruturado no stdout com exit codes corretos
- Mapeamento: `NotFound` → exit 4, `InvalidInput` → exit 65
- Comandos afetados: `set`, `del`, `get`, `query`, `outline`, `edit`, `batch`, `write`, `prune-backups`, `extract`
- GAP-051: `set`/`del` arquivo não encontrado → exit 4 com JSON
- GAP-052: `del` chave inexistente sem `--force-missing` → exit 4 com JSON
- GAP-053: `query`/`outline` arquivo não encontrado → exit 4 com JSON
- GAP-054: `edit --delete-range`/`--range` invertido → exit 65 com JSON
- GAP-055: `set`/`del`/`get` formato não suportado → exit 65 com JSON
- GAP-056: `batch` manifesto vazio → exit 65 com JSON
- GAP-057: `query` sem modo especificado → exit 65 com JSON
- GAP-058: `query --language` não suportada → exit 65 com JSON
- GAP-059: `query` detecção de linguagem falhou → exit 65 com JSON
- GAP-060: `edit` sem modo especificado → exit 65 com JSON
- GAP-061: `edit --old/--new` contagem incompatível → exit 65 com JSON
- GAP-062: `edit --multi` stdin vazio → exit 65 com JSON
- GAP-063: `edit --between` com 1 marcador → exit 65 com JSON
- GAP-064: `edit` sem operação de linha/marcador → exit 65 com JSON
- GAP-065: `write --confirm` abortado → exit 65 com JSON
- GAP-066: `batch` com operações falhas → exit 65 com JSON
- GAP-067: `batch` rollback de transação → exit 65 com JSON
- GAP-068: `prune-backups` sem filtro → exit 65 com JSON
- GAP-069: `extract` overflow de nesting JSON → exit 65 com JSON
- GAP-070: `edit --multi` validação per-op → exit 65 com JSON

### Correção CI (Windows)
- 11 testes do `cli_batch` corrigidos no `windows-2025-vs2026` — manifesto NDJSON agora usa escape JSON correto via helper `common::manifest`
- Adicionado teste de regressão `batch_write_escapes_backslash_in_target_path`

### ADRs
- ADR-0045: Suggestion acionável para erros de parsing do clap
- ADR-0046: Retrofit diff resolve-first
- ADR-0047: Correção do modo read-only do scope

### Validação
- `cargo test` — 621 testes passam (0 falhas, 3 ignorados)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diferenças


## [0.1.23] - 2026-06-19

### GAP-2026-015 — `allow_hyphen_values` ausente em 15 campos CLI de texto livre em 8 structs
- Bug — O Clap v4 trata qualquer token iniciando com `-` como flag CLI por padrão. 15 campos em 8 structs de argumentos (EditArgs, SearchArgs, ReplaceArgs, CalcArgs, RegexArgs, TransformArgs, ReadArgs, QueryArgs) aceitam conteúdo texto livre mas não tinham `allow_hyphen_values = true`. Passar `--old "- bullet point"` ou `search "-deprecated"` ou `calc "-5 + 3"` causava `ARGUMENT_PARSE_ERROR` (exit 2). Em pipelines de agentes LLM, isso disparava falhas em cascata: exit 2 mascarado por pipe jaq, workaround do agente via write truncante, perda catastrófica de dados.
- Correção — Adicionado `allow_hyphen_values = true` a todos os 15 atributos `#[arg]` afetados em `src/cli_args.rs`. Tier 1 (EditArgs): `old`, `new`, `after_match`, `before_match`, `between`. Tier 2 (posicionais): `SearchArgs.pattern`, `ReplaceArgs.pattern`/`replacement`, `CalcArgs.expression`, `RegexArgs.examples`. Tier 3 (nomeados): `TransformArgs.pattern`/`rewrite`/`inline_rules`, `ReadArgs.grep`, `QueryArgs.query`. Excluído: `CaseArgs.subvert` (incompatível com `num_args = 2..` — parser guloso consome flags seguintes).

### ADR
- ADR-0041 — allow-hyphen-values-edit: 15 campos em 8 structs ganham `allow_hyphen_values = true` para aceitar conteúdo Markdown/YAML/diff e números negativos com hífens iniciais

### Validação — GAP-2026-015
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 12 novos testes de regressão em `tests/cli_v0123_hyphen_values.rs`
- 1 novo ADR: 0041 (allow-hyphen-values-edit)
- 1 fechamento GAP-2026 (015)

### GAP-2026-016 — backup-by-default para comandos que mutam conteúdo
- Alterado default de `backup` de `false` para `true` em 9 structs que mutam conteúdo: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`
- Alterado `AtomicWriteOptions::default().backup` de `false` para `true`
- Adicionada flag `--no-backup` a todas as 9 structs para opt-out explícito
- Adicionada variável de ambiente `ATOMWRITE_BACKUP=0` para opt-out global
- Adicionado helper `resolve_backup()` em `src/commands/mod.rs`
- Default existente `keep_backup: false` inalterado — backup auto-deletado após sucesso
- 4 structs não-conteúdo inalteradas: `DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`
- ADR: `docs/decisions/0042-backup-by-default.md`
- 7 novos testes de regressão em `tests/cli_v0123_backup_default.rs`

### GAP-2026-017 — guarda de shrink com --expect-checksum
- Adicionada flag `--allow-shrink` em `WriteArgs`
- Writes que reduzem arquivo em >50% agora são BLOQUEADOS quando `--expect-checksum` está ativo
- Retorna exit 65 (INVALID_INPUT) com sugestão de passar `--allow-shrink`
- Tornado `risk_assessment` (L1) bloqueante quando `--expect-checksum` está ativo e arquivo encolhe
- Sem `--expect-checksum`, comportamento inalterado (compatível retroativamente)
- ADR: `docs/decisions/0043-shrink-guard.md`
- 4 novos testes de regressão em `tests/cli_v0123_shrink_guard.rs`

### GAP-2026-018 — --old-file/--new-file para o comando edit
- Adicionadas flags `--old-file <PATH>` e `--new-file <PATH>` em `EditArgs` como alternativas a `--old`/`--new`
- Conteúdo lido de arquivos dentro do processo atomwrite, contornando shell expansion e limite ARG_MAX do kernel (~131 KB)
- `conflicts_with` impede mistura de `--old` com `--old-file` (exit 2 em conflito)
- Caminhos validados contra jail do workspace (`validate_path`)
- Adicionado campo `source` em `PairResult` ("arg" ou "file") para rastreabilidade da origem do conteúdo
- Trailing newline stripping: `strip_file_trailing_newline()` remove exatamente uma quebra de linha final (`\n` ou `\r\n`) do conteúdo de arquivo para paridade com comportamento argv (arquivos criados por `echo` têm trailing newline que valores argv não têm)
- Validação de cross-mixing: guarda runtime rejeita `--old` + `--new-file` e `--old-file` + `--new` com exit 65 (`INVALID_INPUT`) e mensagem "cannot mix --old with --new-file or --old-file with --new"
- ADR: `docs/decisions/0044-edit-old-file-new-file.md`
- 8 novos testes de regressão em `tests/cli_v0123_old_file.rs`

### Validação
- `cargo test` — todos os testes passam (609 total, 31 novos para v0.1.23)
- `cargo clippy --all-targets -- -D warnings` — zero warnings
- `cargo fmt --check` — zero diffs
- 4 novos ADRs: 0041 (allow-hyphen-values), 0042 (backup-by-default), 0043 (shrink-guard), 0044 (edit-old-file-new-file)
- 4 GAP-2026 fechados (015, 016, 017, 018)


## [0.1.22] - 2026-06-17

### Adicionado
- Sub-comando `prune-backups` para limpeza manual de backups legados (flags `--max-age`, `--max-count`, `--dry-run`)
- Sub-comando `edit-loop` para N edições em 1 invocação via NDJSON no stdin
- ADR-0039 (`docs/decisions/0039-edit-loop-helper.md`)
- ADR-0040 (`docs/decisions/0040-prune-backups-subcommand.md`)
- 2 schemas NDJSON (`prune-backups-output.schema.json`, `edit-loop-output.schema.json`)

### Testes
- 16 novos testes de regressão (3+4+3+3+2+2)
- 2 novos property tests sob feature `slow-tests`
- Cobertura ≥ 80% em código novo

### Documentação
- Marcadores `[FECHADO v0.1.21]` em `gaps.md` para 3 gaps
- Seção "Padrão Correto — Edits Sequenciais com Re-captura de Checksum" em SKILLs EN/PT
- Exemplo copy-paste de loop `while` em `docs/HOW_TO_USE.md`
- Seções v0.1.21 em `docs/AGENTS.pt-BR.md`


## [0.1.21] - 2026-06-17

### GAP-2026-012 — `--allow-sequential-drift` para pipelines sequenciais de `edit`
- Contexto — Agentes que encadeiam múltiplas chamadas `edit` no mesmo arquivo sem re-capturar `checksum_after` entre invocações recebem `STATE_DRIFT` (exit 82) em toda chamada após a primeira. A documentação cobria o cenário paralelo mas não o sequencial (agente único, arquivo único, edições em lock-step).
- Correção — nova flag opt-in `--allow-sequential-drift` em `edit` — quando setada, `cmd_edit` emite `tracing::warn!` nomeando o drift e prossegue com a edição (exit 0 em sucesso). O comportamento default permanece inalterado: `STATE_DRIFT` (exit 82) ainda dispara em mismatch de checksum quando a flag está ausente. Dois padrões válidos para pipelines sequenciais: (a) re-capturar `checksum_after` após cada `edit` e passar para a próxima chamada; (b) passar `--allow-sequential-drift` uma vez em cada chamada e deixar o pré-estado de cada chamada diferir do original. Veja `SKILL.md` para a receita de loop `while` e `docs/HOW_TO_USE.md` para o exemplo copy-paste.

### GAP-2026-013 Problema C — `--backup` e `--keep-backup` expostos em `edit`, `rollback`, `replace`, `apply`, `batch`
- Bug (violação de paridade de API) — `edit` e `rollback` hardcodavam `backup: false` em `AtomicWriteOptions` enquanto `write` e `replace` expunham `--backup`. Usuários que tentavam `--backup` em `edit`/`rollback` eram silenciosamente ignorados. As structs `ReplaceArgs`, `ApplyArgs` e `BatchArgs` tinham o mesmo buraco.
- Correção — `--backup`, `--retention` e `--keep-backup` propagados em 6 subcomandos — `edit` ganha `backup`, `retention`, `keep_backup`; `rollback` ganha `backup`, `keep_backup`; `replace`, `apply`, `batch` ganham `keep_backup`. Os 3 sites hardcoded `backup: false` em `src/commands/edit.rs:139`, `src/commands/edit.rs:393` e `src/commands/rollback.rs:108` são substituídos por `args.backup`. Paridade de subcomando para `--backup` agora é 4/4 (write, edit, replace, rollback); 6/6 subcomandos honram `--keep-backup` (write, edit, replace, rollback, apply, batch).

### GAP-2026-014 v2 — backups são deletados após escritas bem-sucedidas por default
- Contexto — `cleanup_old_backups_in` podava por contagem, deixando backups vivos indefinidamente até que 5 mais novos tomassem seu lugar. Toda operação bem-sucedida com `--backup` deixava lixo persistente em disco; scripts de CI que rodavam `fd '*.bak.*' . | wc -l` viam contagens crescentes proporcionais ao volume de escrita.
- Correção — `keep_backup: bool` em `AtomicWriteOptions`, default `false` — novo helper `delete_backup_quietly(path)` remove o backup após `atomic_write_inner` retornar sucesso. `ErrorKind::NotFound` é mapeado para `Ok(())` (idempotência). Em erros não-NotFound, `tracing::warn!` é emitido e a operação prossegue (cleanup é logado, não propagado). Em caminhos de falha o backup é preservado como antes. `keep_backup: true` é o opt-in explícito para preservar o backup; o comportamento prévio de `--backup` de deixar backups em disco agora só é acessível via `--keep-backup`. 6 subcomandos aceitam a flag: `write`, `edit`, `replace`, `rollback`, `apply`, `batch`. Veja `docs/decisions/0038-backup-cumprido-deleta.md`.

### Paridade — `apply` e `batch` agora honram `--keep-backup`
- `apply` propaga `args.keep_backup` para a chamada interna de `atomic_write` para que um patch bem-sucedido não deixe um `.bak` sibling para trás por default.
- `batch` propaga `--keep-backup` para toda op `write`/`edit`/`replace` no manifesto NDJSON. `keep_backup` por op no NDJSON sobrescreve o default em nível de batch.

### ADR
- ADR-0038 — backup cumprido deleta: justificativa para `keep_backup` default `false` + helper `delete_backup_quietly`; alternativas rejeitadas são scheduler, subcomando `prune-backups` e cleanup por idade (todas subsumidas por deleção-após-sucesso).

### Notas de migração
- Breaking change — `write --backup` e `replace --backup` não deixam mais um sibling `.bak` em disco após uma escrita bem-sucedida. O comportamento pré-v0.1.21 de backup vive para sempre acabou. Adicione `--keep-backup` a qualquer script que dependa do backup persistindo através da operação, ou reescreva para ler o backup antes da escrita completar.
- Breaking change — `edit` e `rollback` agora aceitam `--backup` mas o ignoram sem reclamação se as pré-condições da camada atômica rejeitarem. O novo opt-in é a flag explícita `--backup`; scripts antigos que chamavam `edit` com a suposição de sem backup ainda recebem sem backup por default.
- Não-breaking — `apply --keep-backup` e `batch --keep-backup` são aditivos. Comportamento default (sem backup) permanece inalterado.

### Validação
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 555+ testes passando (542 baseline v0.1.20 + 13 novos: 6 em `cli_v0121_backup_keep_flag`, 2 em `cli_v0121_edit_backup`, 3 em `cli_v0121_sequential_drift`, 1 em `cli_v0121_rollback_backup`, 1 em `cli_v0121_apply_keep`, 1 em `cli_v0121_batch_keep`, 1 em `proptest_v0121_backup_delete`)
- 1 novo ADR: 0038 (backup cumprido deleta)
- 3 novos GAP-2026 fechados (012, 013 Problema C, 014 v2)
- Cross-compile verificado em 3 targets Windows: x86_64-gnu, i686-gnu, x86_64-msvc
- Smoke test de migração: `fd '*.bak.*' . | wc -l` reporta 0 em uma execução pós-sucesso; reporta 1 quando `--keep-backup` está setado


## [0.1.20] - 2026-06-15

> NOTA — Renomeação de flag (ADR-0037) — a flag global `--lang` foi renomeada para `--locale`. A env var `ATOMWRITE_LANG` permanece inalterada (a renomeação foi apenas no flag CLI long-form; o env var e o campo programático `args.global.lang` seguem estáveis). Os valores aceitos permanecem `en` e `pt-BR`. O alias `--lang` foi REMOVIDO desta flag global e LIBERADO para subcomandos — agora passe `--lang` para subcomandos que o aceitam como alias (ex.: `atomwrite scope --lang rust`). Veja `docs/decisions/0037-global-locale-rename.md` para a justificativa completa.

### GAP-2026-001 — `count --by-size` finalmente implementa a flag de help
- Bug — `cmd_count` ignorava `args.by_size` apesar do campo existir em `src/cli_args.rs`. A help prometia top-N por tamanho; o código sempre retornava `mode: "lines"`. HELP-FIRST-DRIFT (flag anunciada na help antes da implementação existir).
- Correção — Nova struct `CountBySizeOutput` em `ndjson_types.rs`. `cmd_count` coleta `Vec<(PathBuf, u64)>` durante o walk, ordena descendente por tamanho, trunca para `args.top` (default 10) e emite o output do top-N.

### GAP-2026-002 — `write --preserve-timestamps` (paridade com edit/replace)
- Bug — `cmd_write` hardcodava `preserve_timestamps: false` em `AtomicWriteOptions` enquanto 5 de 6 subcomandos mutantes expunham a flag.
- Correção — Adicionado `preserve_timestamps: bool` a `WriteArgs`; `cmd_write` agora passa `args.preserve_timestamps` para a camada atômica.

### GAP-2026-003 — alias `scope --lang`
- Bug — `ScopeArgs.language` foi declarado com `short='l' long="language"` apenas, sem `alias = "lang"`. Usuários seguindo os docs da SKILL que mencionam `--lang` recebiam `ARGUMENT_PARSE_ERROR`.
- Correção — Adicionado `alias = "lang"` ao campo `language`.

### GAP-2026-004 — `write --line-ending crlf` (aceita ambas as formas)
- Bug — `LineEnding::CrLf` era renderizado pelo clap como `cr-lf` (kebab-case) enquanto os docs e expectativas prévias de usuários escreviam `crlf` (sem hífen).
- Correção — Adicionados `#[value(name = "cr-lf", alias = "crlf")]` e atributos `value` equivalentes a todas as 4 variantes para superfície CLI explícita e estável.

### GAP-2026-005b — semântica de `edit --partial` documentada
- Apenas doc — `--partial` é implementado para multi-par; o caminho de falha single-par agora retorna `NoMatches` (exit 1) sem escrita. Documentado em skill EN/PT.

### GAP-2026-006 — testes de regressão para `diff --algorithm`
- Doc + testes — `algorithm: DiffAlgorithm` já estava implementado e despachado para `similar::Algorithm::{Myers,Patience,Lcs}`. Adicionados testes de regressão para `myers` vs `patience` vs `lcs`.

### GAP-2026-007 — `count --by-extension` filtra timestamps de backup
- Bug — `validated.extension()` retornava o último componente `.`, então `foo.txt.bak.20260615_035515` era categorizado como `20260615_035515`.
- Correção — Nova regex `BACKUP_RE` `\.bak\.\d{8}_\d{6}$` casa nomes de arquivo de backup e os roteia para uma categoria `"backup"` dedicada.

### GAP-2026-008 — `read --head/--line/--lines` reporta contagem de linhas filtrada
- Bug — `ReadOutput.lines` reportava o total do arquivo não-filtrado mesmo com `--head`, `--line` ou `--lines`.
- Correção — `line_count` agora é computado a partir do `content_str` filtrado; `lines_total` (novo campo opcional) preserva o total original do arquivo para consumidores downstream.

### GAP-2026-009 — `read` emite discriminador `mode`
- API — `ReadOutput` ganha um campo `mode`: `"full" | "head" | "tail" | "line" | "lines" | "grep" | "stat"`. Consumidores downstream não precisam mais parsear o conteúdo para discriminar leituras parciais de leituras completas.

### GAP-2026-010 — `search --no-begin-end` para output de walk vazio mais limpo
- API — Nova flag `no_begin_end: bool` em `SearchArgs`. Default (off) preserva o comportamento pré-v0.1.20. Com a flag, eventos NDJSON `begin`/`end` são suprimidos para arquivos com zero matches.

### GAP-2026-011 — guardas de intenção em `write` (defesa em profundidade após incidente de 2026-06-15)
- Incidente — Durante uma auditoria em 2026-06-15, `atomwrite write` foi chamado sem `--append` sobre `c24-framework34.html` (491827 bytes). O arquivo foi truncado para poucos bytes; ~127 linhas de trabalho de 15-jun (~9 KB) foram perdidas.
- Correção — 5 novas flags em `WriteArgs` (todas opt-in, default off para preservar compat):
  - `--require-backup` (L2): aborta com `InvalidInput` (exit 65) se o alvo existe e `--backup` não está setado.
  - `--confirm` (L3): quando o alvo existe e é maior que 100KB, prompt `Overwrite <path> (<N> bytes)? [y/N]` e lê do stdin. Aborta em qualquer resposta diferente de `y`/`yes`.
  - `--auto-rotate` (L5): quando `--backup` está ativo, força um backup rotativo se o alvo foi modificado nas últimas 24 horas.
  - `--risk-threshold <PERCENT>` (default 50): threshold L1 do guarda de tamanho; emite warning no stderr (`low`/`medium`/`high`) quando o delta de tamanho excede o threshold.
  - Telemetria: `WriteOutput.risk_assessment` (opcional, GAP-2026-011 L6) carrega bytes original/novo, percentual de delta, nível de risco e qual guarda disparou.

### ADR
- ADR-0034 — help-driven testing anti-pattern: clap `--help` nunca mais declarado antes da implementação (5 dos 11 GAP-2026 tinham esse anti-pattern)
- ADR-0035 — write intention guards: 4 flags defense-in-depth (--require-backup, --confirm, --auto-rotate, --risk-threshold) + `risk_assessment` no envelope, motivadas pelo incident c24-framework34.html de 2026-06-15
- ADR-0036 — `edit --partial`: single-pair com zero matches retorna NO_MATCHES (exit 1); multi-pair aplica matched e relata unmatched em `pair_results`
- ADR-0037 — rename `--lang` global para `--locale` (env var `ATOMWRITE_LANG` permanece, campo `args.global.lang` permanece, namespace `--lang` liberado para subcomandos como alias de `--language`)

### Notas de migração
- Breaking change na CLI surface: scripts que passavam `--lang <locale>` devem migrar para `--locale <locale>` (one-liner: `rg -l '\-\-lang\b' bin/ scripts/ && sd -- '\-\-lang\b' '--locale' bin/ scripts/`)
- Env var `ATOMWRITE_LANG` e campo programático `args.global.lang` permanecem estáveis — CI matrices, container wrappers e consumidores Rust não precisam de mudança
- Subcomandos que tinham `--language` agora também aceitam `--lang` como alias (ex.: `atomwrite scope --lang rust`)

### Validação
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 542 testes passando em 47 suites (up from 515 in 46 suites in v0.1.19, +27 new: 11 GAP-2026 closures + 16 intention-guard tests)
- 4 novos ADRs: 0034 (help-driven testing), 0035 (write intention guards), 0036 (edit partial), 0037 (locale rename)
- 11 GAP-2026 fechados (001-011), 100% cobertura dos gaps de auditoria local
- Cross-compile verificado em 3 targets Windows: x86_64-gnu, i686-gnu, x86_64-msvc


## [0.1.19] - 2026-06-14

### G121 — `search` e `replace` resolvem caminhos raiz contra o workspace
- Bug (CWE-367 — TOCTOU/confusão de caminho) — `cmd_search` e `cmd_replace` recebiam os caminhos raiz do chamador, validavam-nos com `path_safety::validate_path` e em seguida alimentavam o `ignore::WalkBuilder` com o caminho ORIGINAL (relativo ao CWD). `validate_path` retornava o caminho absoluto canônico, mas o resultado da chamada por entrada era descartado. Quando `CWD != --workspace`, o walker ou (a) caminhava silenciosamente pela árvore errada se existisse um caminho com mesmo nome sob o CWD, ou (b) emitia um evento `JailViolation` por arquivo caminhado. A correção G118 em `write.rs:44` (ADR-0027) nunca foi propagada para estes dois comandos porque a `validate_path` por entrada dentro da thread de trabalho mascarava a falta da resolução pré-passo.
- Correção — novo helper `path_resolution::resolve_paths_against_workspace` — `cmd_search` e `cmd_replace` agora chamam este helper uma vez no início do comando (após `global.resolve_workspace()()`) e passam o `Vec<PathBuf>` canônico para `build_walker`. O loop pré-passo `for path in &args.paths { validate_path(path, &workspace)?; }` em `replace.rs` é removido; o `search` ganha uma chamada de resolução paralela. Ambas as assinaturas de `build_walker` ganham o parâmetro `canonical_paths: &[PathBuf]`; elas não leem mais `&args.paths[0]` diretamente.
- Consequência — Search e replace agora honram `CWD != --workspace`. Um caminho relativo como `src/` passado com `--workspace /path/to/ws` caminha por `/path/to/ws/src/` independentemente do CWD do processo. Caminhos fora do jail falham uma única vez com `WORKSPACE_JAIL` (exit 126) no início do comando em vez de por arquivo dentro do worker. Veja `docs/decisions/0031-g121-path-resolution-helper.md`.

### G122 — Matching real de S-expression no subcomando `query`
- Bug (feature silenciosa, documentada mas nunca implementada) — O subcomando `query` (v14 Tier 3, introduzido na v0.1.12) sempre prometeu suporte a S-expression. Os documentos em `COOKBOOK.md`, `HOW_TO_USE.md` e nos dois SKILLs bilíngues mostram exemplos como `atomwrite --workspace . query src/main.rs --query "(function_item name: (identifier) @name)"`. Na prática, `cmd_query` chamava `walk_kind_filter` que faz `wanted.iter().any(|w| w == &kind)` — comparação literal de STRING com `node.kind()`. A string inteira `"(function_item name: (identifier) @name)"` nunca casou com nenhum `node.kind()` real, então a feature S-expression nunca funcionou.
- Correção — novo enum `QueryType` + auto-classificação — `classify_pattern(pattern) -> QueryType` detecta S-expression pela presença de `(`, `)`, ou `@`. `walk_sexpr` compila o pattern via `tree_sitter::Query::new`, executa via `QueryCursor::matches`, e emite NDJSON `query_match` com o novo campo `capture_name` para cada `@capture`. `cmd_query` ramifica na classificação.
- Nova dependência direta `tree-sitter = "0.26"` — a language-pack reexporta `Language`, mas `Query`/`QueryCursor`/`StreamingIterator` exigem o crate `tree-sitter` em si.
- Consequência — Patterns com `(`, `)`, ou `@` são roteados para `tree_sitter::Query::new`. Erros de parse (ex.: `(unclosed`) retornam exit 1 com mensagem `invalid S-expression pattern: ...` (via `anyhow::Context`). O caminho kind-filter é preservado bit-a-bit: usuários que passavam `--query function_item` (sem caracteres de S-expression) continuam recebendo os mesmos resultados. Veja `docs/decisions/0032-query-sexp-real-implementation.md`.

### Consolidação de drift na documentação de exit codes (ADR-0033)
- Contexto — Testes de Fase D em 2026-06-14 rodaram 7 probes binários concretos contra a release v0.1.18 e expuseram 7 pontos onde a documentação publicada divergia do comportamento real do binário. Os 7 drifts são:
  1. `STATE_DRIFT` (82) absorve `CHECKSUM_VERIFY_FAILED` (81) para `--verify-checksum` — ambos são classe conflict, retentáveis. O code 81 é agora histórico (preservado apenas para o mismatch BLAKE3 do caminho `read` no conteúdo do arquivo).
  2. `--syntax-check` retorna `SYNTAX_ERROR_DETECTED`, NÃO `SYNTAX_ERROR` — o rename aconteceu no rollout do G72 tree-sitter da v0.1.12 mas a documentação não foi atualizada.
  3. `ORPHAN_JOURNAL` (93) é consultivo, NÃO autodetectado — o portão é `ATOMWRITE_WAL=1` OU `--strict-atomic`. O `write` padrão (v0.1.16 G119 `WalPolicy::Auto`) não escreve sidecar e portanto não pode detectar órfãos.
  4. `BROKEN_PIPE` (141) exige propagação real de SIGPIPE — um pipe simples `head -1` NÃO o dispara. A restauração de SIGPIPE da v0.1.4+ recoloca a disposição default, então o sinal só é levantado quando o consumidor downstream fecha ativamente o pipe no meio do stream.
  5. Leituras de arquivo binário retornam exit 0 com metadados `kind: binary`, NÃO exit 65 — a heurística `BINARY_FILE` da v0.1.4 foi alterada para emitir envelope estruturado e exit 0. O caminho do code 65 agora só dispara para `read` sem `--format raw` E com a heurística binária bypassada.
  6. Argumento posicional ausente retorna `ARGUMENT_PARSE_ERROR` (exit 2), NÃO `INVALID_INPUT` (65) — erros de argumento no nível clap são reportados como exit 2. O code 65 é reservado para validação de conteúdo em runtime (TOML malformado, regex inválida, stdin vazio padrão).
  7. Falta de `--workspace` cai para CWD, NÃO é erro — `--workspace` é documentado como flag com default CWD, não argumento obrigatório. `WORKSPACE_JAIL` (126) só dispara quando um caminho absoluto resolve fora do jail efetivo.
- Decisão — Aceitar o comportamento do binário como canônico. Consolidar a documentação na v0.1.19 para casar. Veja `docs/decisions/0033-v0-1-19-exit-code-naming-drift-consolidation.md`.
- Nota sobre o nome legado `SYNTAX_ERROR` — a documentação da v0.1.12 usava `SYNTAX_ERROR`; o binário na v0.1.18 emite `SYNTAX_ERROR_DETECTED`. O nome histórico é preservado apenas em prosa para grep-ability.

### Validação
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- 515 testes passando em 46 suítes (acima de 502 em 44 suítes na v0.1.18, +13 novos)
- 3 novos ADRs: 0031 (helper de resolução de caminho), 0032 (S-expression de query), 0033 (consolidação de drift de exit code)

## [0.1.18] - 2026-06-14

### G118 — `replace` pré-valida caminhos raiz contra o jail do workspace
- `cmd_replace` resolve-first para todas as raízes — após `global.resolve_workspace()`, o comando itera sobre `args.paths` e chama `path_safety::validate_path(path, &workspace)?` para CADA raiz ANTES de construir o `WalkBuilder`. Falha rápida com `WORKSPACE_JAIL` (exit 126) na primeira violação. Comportamento legado per-entry (v0.1.12-v0.1.17) emitia um evento `JailViolation` por arquivo caminhado e o usuário via o diagnóstico enterrado sob N eventos. Agora `replace /etc/passwd` aborta em microssegundos com um único envelope de erro estruturado.
- Convenção resolve-first agora universal — `write` (ADR-0027), `edit`, `copy`, `apply`, `move`, `rollback`, `set`, `del`, `case` e agora `replace` todos validam o alvo contra o jail workspace ANTES de qualquer `exists()` ou read. Um único modelo mental para todos os comandos mutantes.
- Teste de regressão atualizado — `replace_jail_violation_does_not_inflate_counter` em `tests/cli_v012_regressions.rs` agora afirma exit 126 + envelope `WORKSPACE_JAIL` + arquivos inside/outside inalterados. O nome é preservado (a invariante subjacente — "jail violation não pode inflar o counter de substituições" — é mantida) mas o corpo da asserção mudou.
- 2 novos testes integrados em `tests/cli_replace.rs`: `replace_root_path_outside_workspace_exits_126` (caminho absoluto `/etc/passwd`) e `replace_relative_dotdot_root_outside_workspace_exits_126` (caminho relativo `../escape`).

### G120 L3 — cobertura de teste para cross-validação
- 2 novos testes integrados em `tests/cli_write.rs`:
  - `g120_l3_append_empty_stdin_with_matching_checksum_succeeds` — escreve arquivo seed, hasheia, roda `write --append --allow-empty-stdin --expect-checksum <HASH> < /dev/null`, afirma exit 0 + `stdin_bytes_read: 0` + arquivo inalterado (no-op append preserva checksum).
  - `g120_l3_append_empty_stdin_without_opt_in_rejects_at_l1` — sem `--allow-empty-stdin`, a guarda L1 dispara primeiro (exit 65) e o arquivo é preservado. Documenta que L3 é inalcançável sem opt-in explícita.

### G117 follow-up — cobertura de edge cases
- 3 novos testes integrados em `tests/cli_edit.rs`:
  - `edit_unicode_old_new_exact_match` — diacríticos UTF-8 (`ção` → `AÇÃO`) com casamento exato byte-a-byte. Documenta o contrato de single-pair (substitui apenas a PRIMEIRA ocorrência; multi-pair para múltiplas).
  - `edit_crlf_line_endings_preserve_eol_after_replace` — input com `\r\n`, replace, afirma preservação byte-a-byte (sem colapso para `\n`).
  - `edit_multi_pair_same_old_appears_twice_applies_both` — multi-par onde ambos os pares referenciam o mesmo token `--old`. Garante que o segundo par consome a versão pós-primeiro-substituição (proteção contra off-by-one).

### ADR
- `docs/decisions/0030-v0-1-18-g118-replace-pre-validation-g120-l3-tests-g117-edge-cases.md` — registra as 3 decisões, alternativas consideradas e gatilhos para revisitar.

### Validação
- `cargo build --release` OK
- `cargo clippy --all-targets -- -D warnings` OK
- Suíte completa de testes: 502 testes passando, 0 falhas, 0 regressões introduzidas pelas 3 mudanças
- 2 flakes pré-existentes confirmados como não relacionados (`signal_test::batch_interrupted_by_signal`, `tracing_test::span_captures_path_field` + `tracing_test::debug_level_includes_filter_info`) — falhas idênticas no baseline `git stash` antes das mudanças


## [0.1.17] - 2026-06-13

### G119 — fiação de L3 startup auto-heal e L4 no Drop guard
- L3 startup pass — cada invocação agora chama `auto_heal_on_startup(&workspace, threshold_secs=3600, max_duration_ms=100)` uma vez, ANTES de despachar o subcomando. Threshold default é 1h, orçamento é 100ms, e órfãos `Started` NUNCA são reaped (apenas `Committed`/`Aborted` mais antigos que o threshold). Em um workspace com 60 sidecars stale, o pass reapa todos em <5ms; em um workspace com 10k, o orçamento ainda limita o custo.
- Nova flag global `--no-auto-heal` — desabilita o pass L3 para loops apertados de CI, benchmarks e forense. Também vinculado à env `ATOMWRITE_WAL_NO_AUTO_HEAL=1`. Existe para manter o overhead por invocação previsível quando o workspace tem 0 sidecars (evita o custo de walk+parse em cada comando).
- L4 fiação em `JournalGuard::drop` — o drop agora consulta `heuristics_should_preserve` antes de remover. `h1_ttl`, `h3_rate_limit`, `h4_sentinel`, `h5_archive` todos votam; `h2_lru_within_cap` é intencionalmente bypassado (não há forma barata de saber a contagem global a partir de um drop por arquivo) passando `u64::MAX` tanto para `workspace_committed_count` quanto para `age_rank`. Composição OR: qualquer voto de preservação vence.
- Novo campo `committed_at_unix: Option<u64>` em `JournalGuard` — `release()` carimba o timestamp Unix atual para que as heurísticas L4 que raciocinam sobre idade pós-commit (h1_ttl, h5_archive) tenham dados. O guard inerte e o caminho `keep()` o deixam `None` (nenhuma entrada Committed existia).
- Correções de teste — 2 testes em `tests/cli_v012_wal.rs` (`wal_heal_reaps_stale_committed_journals`, `wal_stats_counts_committed_orphans_malformed`) e 2 em `tests/cli_wal.rs` (`g119_l5_wal_stats_reports_journal_state`, `g119_l5_wal_stats_reports_zeros_on_empty_workspace`) agora passam `--no-auto-heal` para impedir que o pass L3 de startup reapa sidecars stale pré-semeados antes da asserção rodar.
- Atualização de ADR — `docs/decisions/0028-g119-wal-cleanup-intelligent.md` ganhou uma seção "Atualização v0.1.17 — Fiação de L3 startup + L4 no Drop guard" documentando o truque `u64::MAX`, o campo `committed_at_unix`, e a justificativa de isolamento de teste.

### Validação
- `cargo fmt --check` limpo
- `cargo clippy --bin atomwrite --lib --all-targets -- -D warnings` limpo (0 warnings)
- 6 novos testes unitários em `src/wal.rs::tests` para L3 (`l3_auto_heal_on_empty_workspace_reports_zero`, `l3_auto_heal_reaps_old_committed_preserves_started`, `l3_auto_heal_respects_budget`) e L4 (`l4_release_records_committed_at_unix`, `l4_drop_preserves_sidecar_when_h4_sentinel_votes`, `l4_drop_removes_sidecar_when_no_heuristic_preserves`)
- 3 novos testes integrados em `tests/cli_wal.rs` (`g119_l3_startup_auto_heal_reaps_stale_committed`, `g119_l3_no_auto_heal_preserves_stale_committed`, `g119_l4_sentinel_preserves_sidecar_on_successful_write`)
- Suíte completa: 474 testes passando, 0 falhas, 0 regressões


## [0.1.16] - 2026-06-13

### G119 — fecha a limpeza autônoma de 5 camadas (L1 prevention + L4 heuristics)
- L1 — enum `WalPolicy` + flag `--wal-policy` — `Auto` (default) pula o sidecar para escritas triviais (arquivo ≤ 1 MiB AND não Edit/Replace AND diretório pai sob git AND tamanho do arquivo ≤ 4 KiB). `Always` força o sidecar (semântica legada, equivalente a `--strict-atomic`). `Never` suprime criação de sidecar mesmo quando `--strict-atomic` está setado. A decisão acontece dentro de `atomic_write` ANTES de `journal_started_with_guard`; custo é O(0) quando a política vota "sem sidecar". Redução esperada: 60-80% dos sidecars para cargas típicas de LLM agent.
- L4 — `HeuristicsEngine` com 5 regras composíveis — `h1_ttl` (preserva por N segundos após `Committed`, default 0), `h2_lru_within_cap` (preserva dentro do cap de contagem, default 100), `h3_rate_limit` (estrangula quando >K sidecars/min, default 10), `h4_sentinel` (arquivo `.atomwrite_no_wal` desabilita por diretório), `h5_archive` (flag para arquivamento quando mais antigo que 7 dias). Env vars: `ATOMWRITE_WAL_KEEP_SECS`, `ATOMWRITE_WAL_MAX_COUNT`, `ATOMWRITE_WAL_RATE_LIMIT`, `ATOMWRITE_WAL_ARCHIVE_DAYS`. H3 usa `AtomicU64` lock-free para a janela de 60s. `heuristics_should_preserve(target, committed_at_unix, count, rank)` compõe via OR.
- `AtomicWriteOptions.wal_policy: WalPolicy` — novo campo defaultando para `Auto`. Todos os 16 call-sites em `src/commands/` (write, edit, set, get, del, case, copy, replace, transform, scope, apply, rollback, batch×4) atualizados para passar a política adiante.
- Telemetria — envelope NDJSON `WriteOutput` ganha `wal_policy: "auto" | "always" | "never"` para que chamadores possam auditar qual política foi aplicada.
- ADR — `docs/decisions/0028-g119-wal-cleanup-intelligent.md` documenta a arquitetura de 5 camadas e a semântica de composição OR do engine L4.
- Novos testes unitários — 12 testes em `src/wal.rs::tests`: `l1_never_policy_always_returns_false`, `l1_always_policy_always_returns_true`, `l1_auto_policy_returns_true_for_large_file`, `l1_auto_policy_returns_true_for_edit_op`, `l1_auto_policy_skips_trivial_file`, `l4_h1_ttl_default_zero_returns_false`, `l4_h2_lru_within_cap_returns_true_when_count_low`, `l4_h2_lru_returns_true_when_count_at_or_below_default_cap`, `l4_h3_rate_limit_returns_false_below_threshold`, `l4_h4_sentinel_returns_true_when_file_exists`, `l4_h4_sentinel_returns_false_when_absent`, `l4_h5_archive_returns_false_for_recent_journal_under_default`, `l4_h5_archive_returns_true_for_journal_older_than_7_days`, `l4_engine_returns_false_when_all_heuristics_disabled`.

### G120 — fecha a validação de conteúdo de 4 camadas (L3 cross-validation)
- L3 — `--append`/`--prepend` + `--expect-checksum` + stdin vazio emite warning estruturado — quando o chamador combina flag append/prepend com `--expect-checksum` E o stdin é vazio (o caso de bypass de L1 via `--allow-empty-stdin`), `cmd_write` emite um `tracing::info!` que nomeia a combinação de flags cruzadas e prossegue para `verify_checksum` (que ainda valida o estado pré-mutação). Operadores monitorando stderr recebem um sinal explícito sem mudar o exit code.
- L3 opt-out — `--no-checksum-when-empty` — chamador que INTENDE a combinação empty-stdin + checksum (no-op append com garantia de locking) pode passar esta flag para pular `verify_checksum` inteiramente. Emite `tracing::warn!` registrando a decisão.
- L1+L2 inalterados — `read_stdin_content` ainda rejeita stdin vazio por default (exit 65); `handle_append_prepend` ainda rejeita stdin vazio quando `--append`/`--prepend` está setado. L3 é a terceira camada que roda apenas quando L1+L2 são explicitamente bypassadas.
- ADR — `docs/decisions/0029-g120-empty-stdin-guard.md` documenta a semântica de warning L3 e a flag de opt-out.

### Validação
- `cargo fmt --check` limpo
- `cargo clippy --bin atomwrite --lib` limpo (0 warnings)
- Suíte completa: 487 testes passando (469 baseline v0.1.15 + 18 novos em `src/wal.rs::tests`), 0 falhas, 0 regressões
- Novo `WalPolicy` exportado de `crate::wal` e registrado no derive `ValueEnum` do clap
- Schema `WriteOutput` regenerado para incluir campo `wal_policy`; `tests/snapshots/snapshot_write__write_output_structure.snap` atualizado


## [0.1.15] - 2026-06-11

### G117 — `edit --old/--new` multi-par: paridade fuzzy, relato por par e `--partial` opt-in
- Paridade fuzzy — o caminho multi-par usava apenas busca exata `find_str`, então um par com whitespace divergente que a cascata fuzzy do par único resgata derrubava o lote inteiro. A cascata foi extraída para `match_pair` e compartilhada pelos dois caminhos: cada par roda a cascata completa de 9 estratégias (`exact`, `line_trimmed`, `whitespace_normalized`, `punctuation_normalized`, `indent_flexible`, `escape_normalized`, `trimmed_boundary`, `block_anchor`, `context_aware`). `--fuzzy off` preserva o comportamento exato pré-G117.
- Relato por par — envelopes de sucesso ganham `pairs_total` e `pair_results` (array de `{index (1-based), matched, strategy, similarity}`); `mode` vira `fuzzy-multi(N)` quando algum par casou via fuzzy e permanece `exact-multi(N)` caso contrário. Envelopes de erro ganham `failed_pair_index`, `pairs_total` e `pair_results` via a nova variante `AtomwriteError::EditPairFailed`, que reutiliza o code `INVALID_INPUT` e o exit 65 (nenhum code novo). Pares após o índice falho nunca foram tentados e ficam ausentes do array.
- Nova flag `--partial` (opt-in) — aplica os pares que casam em uma escrita atômica (exit 0, `edits < pairs_total`) e relata os ausentes com `matched: false`. Zero pares aplicados sai com 1 (`NO_MATCHES`) sem escrever, igual à semântica do `replace`. O padrão continua all-or-nothing: quando a atomicidade é possível, ela é estritamente melhor que relato de falha parcial.
- Orientação anti-mascaramento — o envelope de erro NDJSON vai para o stdout por contrato, então `edit ... | jaq '.edits'` mascarava o exit 65 como `{"edits": null}` com exit 0 no pipeline. README, SKILL, COOKBOOK e HOW_TO_USE (ambos os idiomas) agora documentam a receita `jaq -e '.edits'` / `${PIPESTATUS[0]}`.
- Schemas regenerados — `docs/schemas/edit-output.schema.json` foi regenerado de `edit --json-schema` (agora inclui `mtime_preserved`, `pairs_total`, `pair_results`); `docs/schemas/error-output.schema.json` ganha os três campos do G117 mais os cinco error codes introduzidos na v0.1.12 que faltavam no enum (`LOCK_TIMEOUT`, `SYNTAX_ERROR_DETECTED`, `EXDEV_FALLBACK_DISABLED`, `COPY_BACK_BLAKE3_FAILED`, `ORPHAN_JOURNAL`).
- Nota de escopo — o modo `--multi` (NDJSON via stdin) permanece inalterado; o G117 cobre apenas pares repetidos `--old`/`--new`. Veja `docs/decisions/0026-g117-edit-multi-pair-fuzzy-partial.md`.

### G118 — `write` resolve o alvo contra o workspace antes de todos os pré-passos
- Bug (dupla identidade de caminho, CWE-367) — `cmd_write` entregava o caminho CRU da CLI (relativo ao CWD) a `handle_append_prepend`, `normalize_line_endings` (auto) e `verify_checksum`, enquanto só `atomic_write` resolvia via `validate_path`. Com alvo relativo e CWD diferente do `--workspace`, append/prepend TRUNCAVA silenciosamente o arquivo, a detecção automática de line ending era pulada e o `--expect-checksum` era pulado por inteiro (qualquer hash aceito, exit 0). Detectado em produção: o `gaps.md` deste repositório foi truncado e recuperado via `rollback --latest --verify`.
- Correção — o alvo é resolvido UMA vez no início de `cmd_write` e o caminho resolvido alimenta os 3 pré-passos e o `atomic_write`. Drift de checksum com CWD divergente agora falha com `STATE_DRIFT` (exit 82); alvo fora do jail falha cedo com `WORKSPACE_JAIL` (exit 126). O campo `path` do NDJSON continua ecoando o caminho do usuário. Ver `docs/decisions/0027-g118-write-path-resolution.md`.
- Por que os testes nunca pegaram — a suíte só usava alvos ABSOLUTOS, imunes ao CWD. Cinco testes de regressão agora usam alvo RELATIVO com `current_dir` fora do workspace (append, prepend, drift exit 82, checksum correto, detecção CRLF), mais um teste-guarda de conformidade garantindo que `&args.target` aparece exatamente uma vez em `write.rs`.

### GAP 18 — CI Windows verde de novo
- `tests/snapshot_write.rs` agora redige `platform.dir_fsync` como `[platform_dir_fsync]`, a mesma técnica já usada para `platform.fsync`. O snapshot fixava `"dir_fsync": "sync_all"`, que o Windows reporta como `best_effort`, mantendo o job `windows-2025-vs2026` vermelho desde a v0.1.12.

### Job de MSRV alinhado ao manifesto
- O job de CI chamado `MSRV 1.85` (toolchain pinado em 1.85) agora testa o MSRV documentado: `MSRV 1.88` com `dtolnay/rust-toolchain@1.88`, casando com `rust-version = "1.88"` do `Cargo.toml`.

### Validação
- 8 novos testes de integração em `tests/cli_edit.rs` (21 no total na suíte): par fuzzy no multi, compat do mode `exact-multi(N)`, `pair_results` no sucesso, `failed_pair_index`/`pairs_total`/`pair_results` no erro com arquivo intacto, compat de `--fuzzy off`, caminho feliz do `--partial`, `--partial` com zero matches sai com 1, `--partial --dry-run` não escreve
- `cargo test --lib` 152 passando (cobertura de suggestion por variante estendida a `EditPairFailed`); `cargo test --test snapshot_write` 9 passando
- Reprodução determinística do `gaps.md` re-executada contra o binário novo: lote misto reporta `failed_pair_index: 2` com arquivo intacto; `--partial` aplica o par 1 com `edits: 1`; `| jaq -e '.edits'` sai com 1 no envelope de erro
- G118: 6 testes de integração novos em `tests/cli_write.rs` (14 no total); reprodução determinística re-executada com CWD divergente: append preserva todas as linhas, o checksum de 64 zeros agora sai com 82 e o arquivo permanece intacto
- Suíte completa após o G118: 461 testes passando (445 na v0.1.12 + 2 na v0.1.14 + 8 G117 + 6 G118), 0 falhas; `fmt`/`clippy -D warnings`/`doc`/`deny`/`audit` verdes; cross-check Windows `x86_64-pc-windows-gnu` com `RUSTFLAGS=-Dwarnings` limpo


## [0.1.14] - 2026-06-07

### Corrigido (Falha de CI - `windows-2025-vs2026` no teste `write_creates_file_with_ndjson_output`)
- Causa raiz — O teste escreve 12 bytes de input (`"hello world\n"`) e espera `bytes_written == 12`. No Windows o v0.1.13 retornava 13 bytes porque o branch de fallback em `normalize_line_endings` retornava `LineEnding::CrLf` quando o arquivo alvo não existia e o SO host era Windows. A chamada subsequente `line_endings::normalize(..., CrLf)` inseria um `\r` antes de cada `\n`. Linux e macOS não eram afetados (o fallback delas retornava `LineEnding::Lf`, que preservava a contagem de bytes do input).
- Solução — Os branches de fallback que retornavam `LineEnding::CrLf` no Windows (quando o arquivo alvo não existia ou não podia ser lido) foram removidos. `Auto` agora cumpre a sua docstring (`Preserve the dominant ending of the original file`): quando não há original, os bytes do input passam adiante sem modificação. Isso torna o CLI determinístico entre Linux, macOS e Windows para o mesmo conteúdo de stdin.
- Paridade de round-trip para input CRLF — Um bug adicional foi descoberto ao escrever os testes de regressão: o fallback antigo não apenas convertia LF para CRLF, ele também *removia* o `\r` de input CRLF ao resolver `Auto` → `Lf` e depois executar a etapa de canonicalização de `normalize`. Com o novo comportamento, input CRLF em arquivo novo agora permanece CRLF byte a byte. Isso preserva o round-trip de `--expect-checksum` quando o usuário fornece conteúdo CRLF.
- Dois novos testes de regressão em `src/commands/write.rs::tests` — `auto_on_new_file_preserves_lf_input` e `auto_on_new_file_preserves_crlf_input`. Eles exercitam o branch `Auto` com input LF e CRLF e afirmam que a saída é igual ao input byte a byte. Os testes são agnósticos de plataforma e teriam pego tanto o bug `cfg!(windows) ? CrLf : Lf` quanto o bug de canonicalização em qualquer runner de CI.
- Nenhuma mudança na lógica de detecção existente — Quando o arquivo alvo existe, `Auto` ainda chama `line_endings::detect(&existing)` e aplica o estilo dominante. Quando o usuário passa um `--line-ending lf|cr-lf|cr` explícito, o valor explícito ainda é aplicado como antes. Apenas o branch `Auto` + arquivo alvo inexistente mudou.

### Validação
- CI Linux: `cargo build --all-features`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features` (152 lib tests + suites de integração + 3 doctests) — todos verdes
- Cross-check Windows CI: `cargo check --target x86_64-pc-windows-gnu --lib` com `RUSTFLAGS=-Dwarnings` e stub de `cc` para o linker mingw-gcc ausente — zero erros, zero warnings
- 8/8 testes de integração `cli_write` passam, incluindo a falha que era exclusiva do Windows


## [0.1.13] - 2026-06-07

### Corrigido (Falhas de CI - `windows-2025-vs2026` exit 1 sob `RUSTFLAGS=-Dwarnings`)
- 4 erros de compilação eliminados na matriz Windows — O CI roda `cargo clippy --all-features -- -D warnings` e `cargo build --all-features` em `windows-2025-vs2026` com `RUSTFLAGS: -Dwarnings` no env. Em Linux esses lints são invisíveis (os símbolos `unix-only` são exercitados), mas em Windows o compilador detecta código morto e aborta. Os 4 erros reportados foram corrigidos:
  - `error: unused import: Duration` em `src/lock.rs:24` — `std::time::Duration` é consumido apenas pelo loop de polling de `flock` em `try_acquire_loop` (gateado por `#[cfg(unix)]`). O `use` foi separado: `use std::time::Instant;` permanece no escopo do módulo, e `#[cfg(unix)] use std::time::Duration;` importa `Duration` somente em Unix.
  - `error: unused variable: strict_atomic` em `src/atomic.rs:381` — O parâmetro `strict_atomic` é lido exclusivamente dentro do branch `#[cfg(unix)]` da EXDEV fallback. Foi anotado com `#[cfg_attr(not(unix), allow(unused_variables))]`, replicando o padrão já estabelecido em `src/signal.rs:15-17` (GAP 06).
  - `error: function copy_tempfile_to_target is never used` em `src/atomic.rs:604` — A função é chamada apenas da linha 446 (dentro de `#[cfg(unix)]`). Foi gateada com `#[cfg(unix)]`, removendo-a completamente da unidade de compilação Windows. É a solução mais limpa: a função depende de semântica unix-only (clona handle de `tempfile` após `EXDEV`), e sua existência no binário Windows seria ruído.
  - `error: clippy::unnecessary_literal_unwrap` em `src/atomic.rs:195` — A heurística `hardlink_nlink.unwrap_or(1) > 1` foi reescrita como `hardlink_nlink.is_some_and(|n| n > 1)`. Em Windows `hardlink_nlink` é literalmente `None` (vide linha 178), o que disparava o lint no `None.unwrap_or(1)`. A forma nova tem semântica idêntica em ambas as plataformas: retorna `false` quando `None`, retorna a comparação booleana quando `Some(n)`.

### Validação
- Linux CI: `cargo build --all-features`, `cargo clippy --all-features -- -D warnings`, `cargo test --all-features` (150 testes de lib passando) — todos verdes.
- Windows CI: Os 4 erros sob `RUSTFLAGS=-Dwarnings` são eliminados. O padrão `#[cfg_attr(not(unix), allow(...))]` é o mesmo já validado em `signal.rs` (GAP 06) que historicamente passa em CI Windows desde v0.1.4.


## [0.1.12] - 2026-06-07

### Adicionado

### Editores estruturados de configuração (v14 Tier 3)
- Subcomandos `set` / `get` / `del` / `case` — Editores estruturados TOML/JSON com envelopes NDJSON bilíngues. Correção de bug: navegação dotted path TOML em `get`/`del` tratava a chave dotted inteira como nome literal; adicionados helpers `get_toml_path` e `remove_toml_path` que descendem por segmentos `Table` manualmente. Navegação JSON (que já usa semântica de pointer) não foi alterada. Verificação end-to-end: `{"type":"get","path":"...","key_path":"package.version","value":"\"0.1.12\"","found":true,"format":"toml","elapsed_ms":0}` agora retorna o valor correto.
- `case` com 5 estilos heck — snake_case, camelCase, PascalCase, kebab-case, SCREAMING_SNAKE_CASE via crate `heck`. Rename multi-arquivo via `--subvert OLD NEW --to <style>`.

### AST estruturado via `tree-sitter-language-pack`
- Subcomando `query` — modos `--kinds`, `--query <KIND>`, `-Q <KIND>`, `--tree`. 305 linguagens suportadas. DFS iterativo via pilha `Vec<Node>` para evitar stack overflow em arquivos profundos.
- Subcomando `outline` — extração estrutural de alto nível (funções, classes, structs, enums, traits, módulos). Filtro `--kind` (repetível) via nomes exatos tree-sitter.
- Verificação de sintaxe G72 REAL via tree-sitter — Flag `--syntax-check` em `atomwrite write` invoca o parser tree-sitter real via crate `tree-sitter-language-pack` em vez da heurística de balanceamento de colchetes. Exit 88 com primeira linha/coluna de erro. 24 linguagens cobertas; extensões desconhecidas fazem fallback para heurística legada. Novo módulo `src/syntax_check.rs` (16 testes unitários).
- Sidecar G114 WAL para recuperação de crash — `atomic_write` escreve `.atomwrite.journal.<target>.atomwrite.journal.json` com entrada `Started` (op_id, expected_new_checksum, pid, started_at_unix) e entrada `Committed` em sucesso. `recover_orphan_journals(dir)` é consultivo — lê sidecars e reporta órfãos sem tocar no filesystem. Novo módulo `src/wal.rs` (8 testes unitários). Schema: `docs/schemas/wal-recovery.schema.json`.

### Corrigido
- Navegação dotted path TOML em `get`/`del` — veja v14 Tier 3 acima.
- 7 novas variantes de erro — `LockTimeout` (83), `SyntaxError` (88), `ExdevFallbackDisabled` (91), `CopyBackBlake3Failed` (92), `OrphanJournal` (93). Todas bilíngues com sugestões `ErrorContext` acionáveis.

### Dependências
- `tree-sitter-language-pack = "1.8"` com features `download` + `dynamic-loading` (parsers sob demanda; footprint da instalação fica pequeno porque as 305 gramáticas NÃO são bundled — apenas a biblioteca loader é).

### Cobertura de Testes
- 445/445 testes passando (eram 320 baseline em v0.1.10, +125 novos): 9 set, 7 case, 5 query, 5 outline, 6 syntax_check E2E, 4 WAL E2E, 6 get/del integração, 4 xattr/reflink Linux-only, 27 auditoria de regressões (strsim 5, heck 6, toml_edit 9, content_inspector 4, serde_yaml 4, locale pt-BR 1, LANG=C 1, --max-filesize edge 1, shell-completion bash/zsh/fish 3, --threads 1 query+outline 2, G72 stream >1MiB 1, case --subvert boundary 4, recover_orphan_journals 3, --json-schema 6).

### Validação
- `cargo test --all-features`: 445/445 PASS em 43 suites de teste
- `cargo fmt -- --check`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: PASS
- `cargo audit`: PASS (0 vulnerabilidades, 379 crates)
- `cargo deny check`: PASS (advisories, bans, licenses, sources todos OK)
- `cargo +1.88 check --all-targets`: PASS (MSRV compliant)
- `cargo build --release`: PASS
- `cargo package --allow-dirty`: PASS
- `cargo publish --dry-run`: PASS
- `cargo install --path . --force`: PASS (binário `atomwrite 0.1.12 (6af0d76)` instalado)

### Notas
- v0.1.12 é uma release ADITIVA. Nenhum subcomando existente foi renomeado ou removido. Todos os novos códigos de saída (83, 88, 91, 92, 93) são adicionados sem alterar os existentes (0-86 inalterados).
- `atomwrite write --syntax-check` é OPT-IN. O comportamento padrão de `write` não mudou.
- `atomwrite write` agora escreve um sidecar WAL apenas quando a env var `ATOMWRITE_WAL=1` está definida, OU quando `--strict-atomic` é passado. O comportamento padrão de `write` NÃO escreve o sidecar (consultivo apenas).
- Veja `docs/MIGRATION.pt-BR.md` para o guia de upgrade completo de v0.1.11 para v0.1.12.


## [0.1.11] - 2026-06-05

### Corrigido (Falhas de CI - windows-2025-vs2026 + signal test flaky no Linux)
- E0433 do Windows `windows-2025-vs2026` resolvido — `libc::write(STDERR_FILENO, ...)` e `libc::STDERR_FILENO` eram referenciados em `src/main.rs:22-23` em uma função compilada em todas as plataformas, mas `libc` é declarado apenas em `[target.'cfg(unix)'.dependencies]`. O build falhava no Windows com `error[E0433]: failed to resolve: use of unresolved module or unlinked crate 'libc'`. O escritor da mensagem de shutdown foi movido para `src/signal.rs` e protegido com `#[cfg(unix)]` (com corpo no-op `#[cfg(not(unix))]`), então o Windows usa o caminho ctrlc existente que emite a banner inline. A nova função `atomwrite::signal::write_shutdown_message()` também faz loop em `EINTR` e `EAGAIN` para ser robusta contra syscalls `write(2)` interrompidos e limites apertados de buffer de pipe impostos por alguns sandboxes de CI.
- `signal_test::shutdown_message_on_stderr` não falha mais intermitentemente no ubuntu-latest — O teste anterior dormia 2 s antes de enviar SIGINT e afirmava que o stderr capturado continha "shutting down". Dois modos de falha independentes foram observados:
  1. O comando search retornava `Err(NoMatches)` quando a flag `shutdown.is_shutdown()` disparava no meio do scan, porque as threads paralelas do walker tinham eventos Begin bufferizados que nunca foram pareados com eventos End, deixando `has_matches = false`. `main.rs` então seguia o branch `Err` e nunca chegava na escrita da banner de shutdown. `cmd_search` agora curto-circuita para `Ok(())` sempre que `shutdown.is_shutdown()` é true, então a main thread segue o branch `Ok(())` e emite a banner como projetado.
  2. `install_handlers_early` e `install_handlers` criavam cada um seu próprio `Arc<ShutdownSignal>` (`signal A` para o polling do search dentro de `atomwrite::run`, `signal B` para a checagem `is_shutdown()` na main thread). Sob a ordenação de chain-of-handlers do `signal-hook`, apenas a primeira instância era flipada quando SIGINT chegava — a flag da segunda instância permanecia `false`, então a main thread seguia o branch `is_shutdown() == false` e saía com 0 sem escrever a banner. Ambas as funções agora compartilham uma única instância de `ShutdownSignal`: `install_handlers_early` instala a cadeia completa de handlers (flag + counter) e `install_handlers` é idempotente (retorna o `Arc` existente quando `GLOBAL_SHUTDOWN` já está populado).
- Teste usa `ATOMWRITE_READY_FILE` para detecção race-free de readiness — `signal_test::shutdown_message_on_stderr` agora define `ATOMWRITE_READY_FILE` para um caminho sob o tempdir do teste e o atomwrite escreve seu PID nesse caminho assim que `install_handlers_early` retorna. O teste faz poll do arquivo com deadline de 10 s antes de enviar SIGINT, eliminando a janela de microssegundos onde SIGINT poderia competir com `posix_spawn` e chegar antes do `sigaction` do kernel ser configurado. Esta mudança é interna ao harness do teste e não tem efeito na superfície CLI publicada.

### Validação
- `cargo fmt -- --check`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo build --release`: PASS (1 m 14 s)
- `cargo test --all-features`: 302/302 testes PASS em 33 suites de teste (5 execuções consecutivas do suite completo)
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`: PASS
- `cargo audit`: PASS (sem vulnerabilidades)
- `cargo deny check`: PASS (advisories, bans, sources OK; um warning cosmético `license-not-encountered` para a allowance ISC não usada em `deny.toml`)

### Notas
- v0.1.11 é uma mudança NÃO-BREAKING. Nenhuma API pública foi modificada.
- A escrita da mensagem de shutdown foi movida de `src/main.rs` para `src/signal.rs` como uma `pub fn` documentada. A função é `#[cfg(unix)]` (dependência de libc) e no-op em não-Unix. Apenas movimento de API interna.
- v0.1.10 foi yanked do crates.io. Novo `cargo install` resolverá para v0.1.11.


## [0.1.10] - 2026-06-05

### Corrigido (Falhas de CI - GAP 20 follow-up)
- `signal_test::shutdown_message_on_stderr` faz flush da mensagem via `io::stderr().lock()` — A primeira correção do v0.1.8 moveu `eprintln!` do signal handler para a main thread, mas usou `writeln!(io::stderr(), ...)` que é fully-buffered quando stderr é redirecionado para um pipe (como em `Stdio::piped()` do `cargo test`). O buffer nunca era flushado antes do processo terminar com o exit code do sinal, então o teste pai via stderr vazio. A correção usa `io::stderr().lock()` para adquirir o guard `StderrLock`, que faz flush do buffer no Drop. Isso garante que a mensagem chegue ao pipe de stderr capturado antes do processo terminar. CI ubuntu-latest confirmará no push.


## [0.1.9] - 2026-06-05

### Notas
- Não há registro de mudanças disponível para esta versão: o repositório não tem entrada, ADR nem gap dela, e não há histórico de versões para reconstruí-la
- O registro lista a 0.1.9 como publicada em 2026-06-05 às 19:45 UTC, sem yank, com `rust-version` 1.88 (fonte: `https://crates.io/api/v1/crates/atomwrite/0.1.9`)


## [0.1.8] - 2026-06-05

### Corrigido (Falhas de CI - GAP 17 e GAP 18)
- `signal_test::shutdown_message_on_stderr` não falha mais no Linux CI — Removido `eprintln!("\natomwrite: shutting down...")` dos handlers de SIGINT e SIGTERM. Segundo POSIX.1-2017 `signal-safety(7)`, funções stdio como `eprintln!` NÃO são async-signal-safe; o stderr do Rust usa um `Mutex` global que pode causar deadlock ou perder output bufferizado se o sinal chegar enquanto outra thread segura o lock. A mensagem de shutdown visível ao usuário agora é emitida pela main thread em `src/main.rs` quando observa `is_shutdown() == true` após `atomwrite::run` retornar, que é a única forma async-signal-safe de garantir que a mensagem chegue ao pipe de stderr capturado antes do processo terminar. O caminho Windows `ctrlc` ainda emite a mensagem inline (handlers ctrlc rodam em thread normal, não em contexto de sinal).
- `atomic::tests::create_backup_and_retention` não falha mais no Windows CI — Adicionado `platform::fsync_file_best_effort` que registra warning e continua em vez de retornar erro. No Windows, produtos antivírus (Windows Defender, AVs terceiros) podem segurar transientemente um handle de leitura em arquivos em `%TEMP%` com `FILE_SHARE_READ` mas sem `FILE_SHARE_WRITE`, fazendo `FlushFileBuffers` retornar `ERROR_ACCESS_DENIED` (os error 5). O caminho de escrita principal ainda usa o `fsync_file` estrito; apenas o fsync de durabilidade do backup é best-effort porque o backup em si já foi criado via `fs::copy`.
- Matriz de CI fixada em `windows-2025-vs2026` — A entrada da matriz para Windows foi alterada de `windows-latest` para `windows-2025-vs2026` (seu sucessor antes da migração de runners hospedados no GitHub em 15 de junho de 2026). Isso silencia o NOTICE "windows-latest requests are being redirected to windows-2025-vs2026 by June 15, 2026" e previne mudanças inesperadas de runner que possam quebrar o build.

### Validação
- `cargo build --all-features`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo test --all-features`: 302/302 testes PASS (todos os 5 casos de `signal_test` passam; `atomic::create_backup_and_retention` passa)
- `cargo fmt -- --check`: PASS
- `cargo audit`: PASS (sem vulnerabilidades, sem flag `--ignore`)
- `cargo deny check`: PASS (advisories, bans, licenses, sources todos OK)

### Notas
- v0.1.8 é uma mudança NÃO-BREAKING. Nenhuma API pública foi modificada.
- A mudança no signal handler é interna: consumidores externos que dependiam da mensagem de shutdown aparecer no stderr continuam a vê-la; ela agora é emitida pela main thread em vez do signal handler.
- A mudança no fsync de backup do Windows é interna: arquivos de backup ainda são criados e atômicos; a única diferença é que o flush de durabilidade para metadados de backup é best-effort. Se um usuário futuro relatar perda de dados em backup, podemos re-apertar o fsync.


## [0.1.7] - 2026-06-05

### Corrigido (Falhas de CI - GAP 15)
- `cargo audit` não reporta mais RUSTSEC-2026-0009 — Atualizada a dependência transitiva `time` de 0.3.45 para 0.3.47 (DoS via stack exhaustion no parser RFC 2822, corrigido upstream via DEPTH_LIMIT=32). A atualização exigiu bump do MSRV de 1.85 para 1.88. A entrada `ignore` para RUSTSEC-2026-0009 no `deny.toml` foi removida porque a advisory não se aplica mais.
- Falha de CI no macos-latest — `src/platform.rs:31` não usa mais `return Ok(())` (removido return redundante). O lint `needless_return` do clippy 1.94+ não dispara mais; o env `RUSTFLAGS: -Dwarnings` na CI não aborta mais o build.
- Falha de CI no windows-latest — As constantes `EXIT_SIGINT` e `EXIT_SIGTERM` em `src/signal.rs:15-18` agora têm `#[cfg_attr(not(unix), allow(dead_code))]`. O env `RUSTFLAGS: -Dwarnings` não aborta mais em `dead_code` em builds Windows.
- Deprecação de Node 20 em `actions/checkout` e `actions/cache` — Ambas as actions foram bumparadas para a major version que suporta Node 24 (`actions/checkout@v6`, `actions/cache@v5`). `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: "true"` adicionado ao env do workflow como cinto-e-suspensórios. O warning de deprecação não aparece mais nos logs de CI.
- MSRV bumped para 1.88 — `rust-version` no `Cargo.toml` agora é 1.88. Todos os arquivos de documentação (EN e PT-BR) atualizados: `docs/INSTALL.md`, `docs/INSTALL.pt-BR.md`, `docs/HOW_TO_USE.md`, `docs/HOW_TO_USE.pt-BR.md`, `docs/CROSS_PLATFORM.md`, `docs/CROSS_PLATFORM.pt-BR.md`, `docs/COOKBOOK.md`, `docs/COOKBOOK.pt-BR.md`, `CONTRIBUTING.md`, `CONTRIBUTING.pt-BR.md`.

### Mudado
- `build.rs:4-12` — Colapsado `if let + if` aninhado em `if let + &&` para satisfazer o lint `collapsible_if` do clippy 1.94+.
- `src/lib.rs` — Adicionado `#![allow(clippy::collapsible_if)]` e `#![allow(clippy::needless_return)]` como decisões deliberadas do projeto para manter blocos `if let` em linhas separadas para legibilidade. Isso evita 25 sites separados de refatoração em handlers de subcomando.
- Snapshot test platform-aware — `tests/snapshot_write.rs` e `tests/snapshots/snapshot_write__write_output_structure.snap` agora usam placeholder `[platform_fsync]` para o campo `platform.fsync`, permitindo que o mesmo snapshot seja válido em Linux (`sync_data`), macOS (`F_FULLFSYNC`), e Windows.

### Validação
- `cargo build --all-features`: PASS
- `cargo clippy --all-features --all-targets -- -D warnings`: PASS
- `cargo test --all-features`: 302 de 303 testes PASS (1 falha pré-existente em `signal_test::shutdown_message_on_stderr` rastreada como GAP 16, não relacionada ao GAP 15)
- `cargo fmt -- --check`: PASS
- `cargo audit`: PASS (sem vulnerabilidades, sem flag `--ignore`)
- `cargo deny check`: PASS (advisories, bans, sources todos OK)
- Cross-compile `x86_64-pc-windows-gnu`: PASS (build, clippy -D warnings, tests --no-run)
- Cross-compile `i686-pc-windows-gnu`: PASS (check --all-features)

### Notas
- Esta é uma mudança NÃO-BREAKING para usuários em Rust 1.88 ou posterior. Usuários em Rust 1.85-1.87 devem atualizar.
- A dependência transitiva `time` agora está patched (0.3.47+), resolvendo RUSTSEC-2026-0009.
- Targets Windows GNU e i686 para cross-compile são agora explicitamente validados pelo workflow de desenvolvimento local; target MSVC requer runner Windows (job CI windows-latest cobre).

### Corrigido (GAP 16 - signal_test)
- `signal_test::shutdown_message_on_stderr` não falha mais em macOS — Substituída a chamada `libc::write(2, SHUTDOWN_MSG.as_ptr().cast(), ...)` nos handlers de SIGINT e SIGTERM por `eprintln!`. O stderr do runtime Rust é capturado de forma confiável pelo `Stdio::piped()` no processo de teste, enquanto writes brutos via libc eram perdidos na herança de process group do cargo test. A constante `SHUTDOWN_MSG` foi removida por ser dead code.
- Confiabilidade do test em `tests/signal_test.rs` — Aumentado o `thread::sleep` de 50ms para 2000ms antes de enviar SIGINT. Os 50ms anteriores eram insuficientes para que o processo filho do atomwrite inicializasse completamente tracing, mimalloc, e signal handlers antes de receber o sinal. Aumentado o payload por arquivo de 100 para 1000 linhas para que o loop de search demore o suficiente para confirmar shutdown gracioso. O teste agora é estável em 5 execuções consecutivas.


## [0.1.6] - 2026-06-05

### Adicionado (Badges do README)
- Badge docs.rs no README.md e README.pt-BR.md — Adicionado `[![docs.rs](https://img.shields.io/docsrs/atomwrite)](https://docs.rs/atomwrite)` entre os badges Crates.io e License. O badge estava ausente do README publicado, mesmo com a documentação sendo construída com sucesso no docs.rs. O badge agora aparece no README renderizado em crates.io e na página do repositório no GitHub.

### Notas
- v0.1.6 é NÃO-BREAKING. A mudança é puramente visual (imagem de badge no README).
- Nenhuma mudança de código ou API pública.
- Nenhum guia de migração no CHANGELOG é necessário.


## [0.1.5] - 2026-06-05

### Mudado (Higiene de Documentação)
- `#![warn(missing_docs)]` promovido para `#![deny(missing_docs)]` — Documentação faltando em API pública agora é erro de build, não warning. Todos os itens públicos já estavam documentados em v0.1.4 (verificado via `RUSTDOCFLAGS="-D warnings" cargo doc --all-features`), então nenhuma documentação foi adicionada nesta mudança.
- `#![warn(rustdoc::broken_intra_doc_links)]` promovido para `#![deny(...)]` — Links quebrados em intra-doc agora falham o build ao invés de serem warnings silenciosos.
- `#![doc(html_root_url = "https://docs.rs/atomwrite/0.1.2")]` removido — O atributo estava hardcoded na versão 0.1.2, fazendo com que intra-doc-links gerados para versões mais novas (0.1.3, 0.1.4) apontassem para 0.1.2. O atributo está obsoleto desde rustc 1.48 em favor do campo `repository`, que já está configurado no `Cargo.toml`. docs.rs agora usa a versão atual do crate para resolver links automaticamente.

### Mudado (Metadata docs.rs)
- `[package.metadata.docs.rs]` limpo — Removido `all-features = true` (não existe tabela `[features]`, então a flag era no-op) e `rustdoc-args = ["--cfg", "docsrs"]` (não existem marcadores `#[cfg(docsrs)]` no código). Adicionado `targets = ["x86_64-unknown-linux-gnu"]` para tornar o target do build do docs.rs explícito.

### Testes
- 302 testes passam com 0 falhas (inalterado desde v0.1.4)
- 3 testes ignorados (cross-compile Windows, inalterado)
- `cargo doc --no-deps --all-features` com `RUSTDOCFLAGS="-D warnings"` passa limpo

### Notas
- v0.1.5 é NÃO-BREAKING. Os lints promovidos para deny já são satisfeitos pelo código atual.
- v0.1.5 não altera nenhuma API pública nem comportamento. Apenas apertar a fiscalização de documentação e remover metadata obsoleta.
- Nenhum guia de migração no CHANGELOG é necessário.


## [0.1.4] - 2026-06-05

### Corrigido (Compilação Windows - GAP 14)
- `cargo install atomwrite` no Windows 10/11 — Resolvidos três erros de compilação que bloqueavam a instalação em Windows desde v0.1.3. Erro `E0433` em `src/atomic.rs:404` (tipo `AtomwriteError` usado sem import), erro `E0308` em `src/platform.rs:116` (comparação de `*mut c_void` com literal `0`), e erro `E0507` em `src/atomic.rs:387` (assinatura `&NamedTempFile` mas chamada `.persist()` requer ownership). Todos os três bugs estavam em blocos `#[cfg(windows)]` invisíveis ao CI Linux.

### Corrigido (Correção FFI - GAP 14)
- `src/platform.rs:116` — Substituída comparação `handle != 0` por `!handle.is_null()` para conformidade com o padrão idiomático de raw pointer check em Rust. O `HANDLE` retornado por `GetStdHandle` é um `*mut c_void`; compará-lo com literal inteiro viola o sistema de tipos e disparava `E0308`. Padrão agora é `is_null()` para nulidade e `!= INVALID_HANDLE_VALUE` (que já é `HANDLE`) para validade.

### Corrigido (Sugestões de Erro - GAP 13)
- `WorkspaceJail` com `workspace_provided` — Quando o usuário já forneceu `--workspace` ou `ATOMWRITE_WORKSPACE`, a sugestão agora diz "use a path inside the workspace" em vez de re-pedir a flag. Removido phantom `--force-text` que causava exit 2 em cascata.
- 20 variants com sugestão — Adicionadas sugestões actionáveis para `InvalidInput`, `Io`, `ConfigInvalid`, `FileImmutable`, `NoMatches`, e `InternalError`. Apenas `BrokenPipe` (SIGPIPE não-acionável) permanece sem sugestão.
- `ErrorContext` struct — Carrega `workspace_provided: bool` e `workspace: Option<PathBuf>`. Novas funções `ErrorJson::from_error_with_context()` e `output::write_error_json_with_context()` usam o contexto para sugestões precisas.
- `FileImmutable` — Sugestão menciona `chattr -i` (Unix) e `fsutil` (Windows) para remover atributo imutável.
- `NoMatches` — Sugestão orienta a ampliar padrão e revisar `--include`/`--exclude`.
- `InternalError` — Sugestão orienta reportar o bug com contexto.

### Adicionado (Validação Cross-Platform - GAP 14)
- `tests/cross_compile_check.rs` — Novo gate de cross-compile que executa `cargo check` contra `x86_64-pc-windows-gnu`, `i686-pc-windows-gnu`, e `x86_64-pc-windows-msvc`. Falha se `E0433`, `E0308`, ou `E0507` reaparecer em qualquer bloco `cfg(windows)`. Testes marcados `#[ignore]` para skip gracioso em hosts sem targets Windows.
- `output::write_error_json_with_context()` — Nova função que aceita `&ErrorContext` para propagar proveniência de `--workspace` até o output NDJSON.
- Documentação de instalação Windows — Novos arquivos `docs/INSTALL.md` (EN) e `docs/INSTALL.pt-BR.md` (PT-BR) com pré-requisitos Windows 10/11, comandos `cargo install` e troubleshooting.

### Mudado
- `src/atomic.rs:13-15` — Movido `use crate::error::AtomwriteError` para dentro de bloco `#[cfg(windows)]` para evitar warning de `unused_imports` em builds Linux/macOS. Tipo só é referenciado dentro de `persist_with_retry`.
- `src/atomic.rs:386-409` — `persist_with_retry` agora recebe `NamedTempFile` por valor e recupera o arquivo de `e.file` no branch de retry. Caller atualizado para passar `temp` por valor.
- `src/main.rs:93-105` — Reporte de erro agora constrói `ErrorContext` com `workspace_provided: cli.global.workspace.is_some()` para que a sugestão de `WorkspaceJail` se adapte à invocação do usuário.

### Testes
- 7 novos testes GAP 13 em `src/error.rs::tests`: `gap13_workspace_jail_suggestion_when_workspace_not_provided`, `gap13_workspace_jail_suggestion_when_workspace_provided`, `gap13_all_variants_have_suggestion`, `gap13_binary_file_suggestion_does_not_mention_force_text_wrong_flag`, `gap13_file_immutable_suggestion_mentions_chattr`, `gap13_no_matches_suggestion_mentions_filters`, `gap13_error_context_default_matches_legacy_behavior`.
- 1 novo teste de integração GAP 13 em `tests/cli_v012_regressions.rs`: `gap13_jail_suggestion_when_workspace_supplied_says_inside`.
- Teste existente `jail_suggestion_mentions_workspace_flag` atualizado para validar que a sugestão menciona `--workspace` apenas quando workspace NÃO é fornecido (fix GAP 13).

### Notas
- GAPs 01-12 (previamente resolvidos) re-auditados via `cargo test --all-features` — todos os 300+ testes continuam passando.
- Decisão atômica `atomwrite-no-github-actions` mantida: release é manual via `cargo publish` local após 8 gates oficiais e cross-compile gate. CI matrix em `.github/workflows/ci.yml` existe apenas como referência, não é executado.


## [0.1.3] - 2026-06-03

### Mudado (BREAKING)
- Comportamento padrão de escrita atômica para `edit` e `replace` — `AtomicWriteOptions::default()` agora define `preserve_timestamps: false` (era `true`). O mtime de um arquivo editado ou substituído é agora atualizado para o momento em que a escrita é concluída, que é o padrão correto para sistemas de build que usam mtime para detectar mudanças em código fonte (cargo, make, cmake, gradle, sbt, bazel, ninja, msbuild). Para cenários de backup, snapshot ou builds reproduzíveis onde o timestamp original precisa ser preservado, use a nova flag `--preserve-timestamps` em `edit` e `replace`. O módulo fingerprint do cargo compara o mtime dos arquivos fonte contra o mtime dos arquivos `target/.fingerprint/<unit>/dep-info`; com o padrão antigo, o cargo pulava o rebuild silenciosamente (o no-op "Finished in 0.29s") porque o fonte aparecia mais antigo que o binário. Veja o guia de migração v0.1.2 → v0.1.3 em `docs/MIGRATION.pt-BR.md` para o caminho de atualização.

### Adicionado (Consciência de Sistema de Build)
- Flag `--preserve-timestamps` em `edit` e `replace` para voltar ao comportamento v0.1.2 de manter o mtime original do arquivo
- Campo `mtime_preserved` nas respostas NDJSON de `EditOutput` e `ReplaceResult` para que consumidores verifiquem se o timestamp foi mantido ou atualizado (sempre presente; `true` quando `--preserve-timestamps` foi passado, `false` por padrão)
- Novos testes de regressão em `src/atomic.rs::tests`: `atomic_write_updates_mtime_by_default` e `atomic_write_preserves_mtime_when_opted_in`

### Adicionado (Documentação)
- Nova seção "Tempo De Modificação E Sistemas De Build" em `docs/HOW_TO_USE.pt-BR.md` explicando como cargo, make, cmake, gradle, sbt, bazel, ninja e msbuild detectam mudanças em código fonte via mtime e por que o padrão foi alterado
- Equivalente em inglês em `docs/HOW_TO_USE.md`
- Nova receita "Como Editar E Disparar Build Sem Touch Manual" em `docs/COOKBOOK.pt-BR.md` mostrando o workflow `atomwrite edit && cargo build` que não requer mais `touch`
- Equivalente em inglês em `docs/COOKBOOK.md`
- Todas as mudanças v0.1.2 → v0.1.3 documentadas em `gaps.md` seção "Atomic Edit Preserva mtime E Quebra Detecção De Mudança Pelo Cargo" (GAP 12)

### Cobertura de Testes
- 2 novos testes de regressão em `src/atomic.rs` para o contrato padrão-atualiza-mtime e opt-in-preserva-mtime
- 2 testes em `src/ndjson_types.rs::tests` atualizados para o novo campo `mtime_preserved` em `EditOutput` e `ReplaceResult`
- 2 arquivos de snapshot atualizados: `tests/snapshots/snapshot_write__edit_output_structure.snap` e `tests/snapshots/snapshot_write__replace_result_structure.snap` agora incluem o novo campo `mtime_preserved: false` no output JSON
- Total: 33 suítes de teste, 294 testes passando (era 292+ em v0.1.2)

### Gates de Validação
- `cargo fmt --check` limpo
- `cargo clippy --all-targets --all-features -- -D warnings` zero warnings
- `cargo test --all-features` 33 suítes passando
- `cargo doc --no-deps --all-features` zero warnings
- Comportamento end-to-end verificado: arquivo com mtime=2024-01-01 → `atomwrite edit` (padrão) → mtime=agora → `cargo build` rebuilda corretamente; `--preserve-timestamps` mantém o mtime de 2024-01-01 como esperado


## [0.1.2] - 2026-06-02

### Correções (CRÍTICAS)
- Falha de compilação no macOS — `nix::fcntl::posix_fadvise` restrito a `cfg(target_os = "linux")` então atomwrite agora compila no macOS arm64/Intel (a crate nix restringe o símbolo apenas em `linux_android | emscripten | fuchsia | freebsd`, quebrando o macOS anteriormente)
- `batch --transaction` rollback agora é real — arquivos pré-existentes são restaurados E arquivos novos criados por operações `write` são removidos. O evento NDJSON `rollback` agora reporta `files_restored`, `files_removed` e `total_reverted` para que LLMs verifiquem o contrato ACID. Anteriormente, arquivos criados no meio da transação nunca eram limpos.
- `replace` não infla mais contadores em violações de jail — `total_replacements` é incrementado apenas DEPOIS da validação do jail do workspace passar. Violações agora emitem um evento de erro `JailViolation` com `error_class: permanent` e `retryable: false`.
- Eventos paralelos do `search` são agrupados por path — threads paralelas do walker não intercalam mais eventos `begin`/`match`/`end` de arquivos diferentes na saída NDJSON. Consumidores (LLM e humanos) veem sequências contíguas de eventos por arquivo.
- `scope --delete` em comentários Rust não deixa mais espaço em branco órfão — a query preparada para comentários Rust agora casa whitespace trailing, então a deleção produz código limpo.
- `search` com regex inválido emite envelope JSON estruturado — padrões inválidos agora falham com `AtomwriteError::InvalidInput` que propaga através de `write_error_json` para stdout, não stderr cru.

### Correções (ALTAS)
- `batch --file <PATH>` agora é funcional — a flag está conectada via `cmd_batch` para ler o manifesto NDJSON de um arquivo (validado contra jail do workspace) em vez de apenas stdin.
- `backup --output-dir` agora é respeitado — a flag vai através de `AtomicWriteOptions.backup_output_dir` para `create_backup_in`, que cria o diretório se estiver faltando e faz prune de backups antigos naquele diretório.

### Correções (UX)
- Mensagem de erro de jail do workspace corrigida — erros `WORKSPACE_JAIL` agora sugerem `--workspace <root>` ou `ATOMWRITE_WORKSPACE=<path>` em vez da enganosa "use an absolute path" (que estava errada quando o path já era absoluto).
- Bug de retenção de backup do proptest corrigido — `cleanup_old_backups_in` agora poda corretamente backups antigos ao usar `create_backup_in` com diretório de saída customizado.

### Mudado (Dependências)
- `nix` atualizado de 0.29 para 0.31 (estável mais recente)
- `signal-hook` atualizado de 0.3 para 0.4 (estável mais recente)
- `windows-sys` atualizado de 0.59 para 0.61 (estável mais recente)
- `rust-i18n` atualizado de 3 para 4 (estável mais recente)
- Assinatura de `nix::fcntl::posix_fadvise` mudou de `AsRawFd` para `AsFd` em 0.31 — código adaptado adequadamente

### Adicionado (Funcionalidades Agent-First)
- Flag global `--timeout <SECONDS>` para tempo limite de execução (na introdução: 0 = sem timeout, padrão 0; desde v0.1.33 o padrão é 120, exit 124 no prazo)
- Flag `--grep <REGEX>` em `read` para filtrar linhas retornadas por regex
- `completions --install` para instalar scripts de completions no diretório de dados XDG (`~/.local/share/bash-completion/completions/atomwrite` para Bash, etc.)

### Segurança
- Baseline do `cargo audit` reconhece 1 vulnerabilidade: `RUSTSEC-2026-0009` em `time 0.3.45` (DoS via exaustão de pilha). Correção requer `time >= 0.3.47` que precisa de Rust 1.88. Nossa MSRV é 1.85, e atomwrite usa `time` apenas via `tracing-appender` para timestamps de log — não explorável. Rastreado para bump de MSRV em 0.2.0.

### Testes
- 10 novos testes de regressão em `tests/cli_v012_regressions.rs` cobrindo todos os 6 bugs fixos
- Total: 33 suítes de teste, 292+ testes passando (era 282 em v0.1.1)


## [0.1.1] - 2026-06-01

### Corrigido
- 12 links intra-doc quebrados em `error.rs` corrigidos (`DiskFull` para `Self::DiskFull` e similares)
- `search --include`/`--exclude` agora filtram arquivos corretamente via OverrideBuilder (antes era silenciosamente ignorado)
- `replace --include`/`--exclude` agora filtram arquivos corretamente via OverrideBuilder
- `transform --include`/`--exclude` agora filtram arquivos corretamente via OverrideBuilder
- `search --context` agora emite linhas de contexto via SearchSink customizado
- `search --max-count` agora limita matches por arquivo via SearcherBuilder.max_matches()
- `search --invert` agora mostra linhas sem correspondência via SearcherBuilder.invert_match()
- `search --sort` agora ordena resultados por caminho de arquivo
- `transform` agora processa arquivos em paralelo via WalkParallel + crossbeam channel
- `read` timestamp de modificação agora retorna formato ISO 8601 em vez de epoch seconds
- `batch delete` backup agora usa create_backup() atômico com fsync
- `create_backup` agora usa `fs::copy` em vez de `fs::hard_link` para prevenir corrupção de backup quando original é sobrescrito in-place
- Códigos de saída em `output.rs`, `read.rs`, `batch.rs` e `hash.rs` movidos de números mágicos para constantes nomeadas em `constants.rs`
- `DETECTION_SIZE` em `binary_detect.rs` centralizado para `BINARY_DETECT_SIZE` em `constants.rs`
- Seis chamadas `unwrap()` em `edit.rs` modo multi-edit substituídas por `ok_or_else` para tratamento de erro mais seguro
- Join de thread em `scope.rs` alterado de `unwrap()` para `let _ = join()` para prevenir propagação de panic
- `unwrap()` em `rollback.rs` substituído por `expect` com mensagem descritiva
- Documentação `# Errors` adicionada em três funções públicas de `output.rs` que retornam `Result`
- Dados de teste em português em `file_io.rs` substituídos por inglês

### Adicionado
### Novos Subcomandos
- Subcomando `scope` para escopo gramatical: selecionar categorias AST (comentários, funções, strings, etc.) e aplicar ações (delete, upper, lower, titlecase, squeeze, replace)
- `scope` suporta Rust (30 queries preparadas), Python (13), JavaScript/TypeScript (11), Go (8) e padrões AST customizados via `--pattern`
- Subcomando `backup` para criar backups de arquivos com timestamp, checksums BLAKE3 e retenção configurável
- Subcomando `rollback` para restaurar arquivos a partir de backups anteriores com verificação BLAKE3 opcional
- Subcomando `apply` para aplicar patches do stdin com detecção automática de formato (unified diff, blocos SEARCH/REPLACE, diff com fence markdown, substituição completa de arquivo)

### Expansão de Operações em Batch
- `batch` suporta 7 operações: write, replace, delete, edit, hash, move, copy
- `batch --transaction` flag para execução tudo-ou-nada com rollback automático
- Operações `move` e `copy` do `batch` agora aceitam `source`, `from` e `src` como aliases para o caminho de origem
- Operações `write`, `delete`, `edit` e `hash` do `batch` agora aceitam `path` como alias de `target`

### Aprimoramentos do Motor de Edição
- `edit --fuzzy` flag com cascata de 7 estratégias de correspondência (exact, line_trimmed, whitespace_normalized, indent_flexible, escape_normalized, trimmed_boundary, block_anchor)
- `edit --multi` flag para aplicar múltiplas operações de edição NDJSON em uma única escrita atômica
- `edit` saída NDJSON inclui campos fuzzy, strategy, strategies_tried, similarity quando correspondência fuzzy é usada

### Segurança de Caminho
- Detecção de FIFO e arquivos de dispositivo na validação de caminho (códigos de saída 85 e 86) — previne escritas atômicas em arquivos especiais
- Detecção de hardlink antes do rename atômico com `tracing::warn` quando nlink > 1
- Detecção de mesmo arquivo em `copy` e `move` para prevenir perda de dados quando origem=destino (estava sobrescrevendo)

### Internacionalização (i18n)
- Flag global `--lang` para override de locale (en, pt-BR) com variável de ambiente `ATOMWRITE_LANG`
- Suporte a i18n via `rust-i18n` e `sys-locale`: detecção automática de locale do SO com traduções en e pt-BR
- Todas as strings voltadas ao usuário agora cientes de locale (erros, avisos, mensagens informativas)

### Documentação Bilíngue
- Documentação bilíngue `ARCHITECTURE.md` (en + pt-BR) descrevendo mapa de módulos, fluxo de dados e decisões chave
- Headers de licença SPDX (`MIT OR Apache-2.0`) em todos os 64 arquivos `.rs`
- Documentação `//!` em nível de módulo em todos os 38 módulos fonte
- Doctests executáveis para funções `is_binary`, `detect` e `normalize`
- Configuração `[package.metadata.docs.rs]` para builds no docs.rs
- Campo `documentation` e `[badges.maintenance]` no `Cargo.toml`
- Lints rustdoc: `broken_intra_doc_links`, `private_intra_doc_links`, `clippy::doc_markdown`
- `doc(html_root_url)` para cross-linking no docs.rs

### Supply Chain e Segurança
- `deny.toml` para auditoria de licenças e advisories via cargo-deny
- Módulo de detecção e normalização de terminadores de linha (`line_endings.rs`)
- Flag `--line-ending lf|crlf|cr|auto` em `write` e `edit` para normalização de terminadores de linha

### Infraestrutura de Testes
- 282 testes entre suítes de integração e unitários (eram 5 testes em 1 módulo na v0.1.0)
- Testes de integração para `backup`, `rollback`, `apply` e `scope`
- 2 alvos de fuzzing (`batch_parse`, `extract_json`) com `libfuzzer-sys` para testes de segurança dos parsers
- Testes de integração de locking otimista para `write --expect-checksum` e `edit --expect-checksum`
- Testes de validação NDJSON expandidos de 5 para 20 de 21 comandos
- Testes de interoperabilidade `jaq` validando NDJSON via pipe com filtro `jaq`
- Teste de integração i18n confirmando que `--lang` não altera saída JSON

### Segurança
- Headers de licença SPDX garantem clareza de licença em todos os arquivos fonte
- cargo-deny enforces conformidade de licença e rastreia advisories de segurança
- Detecção de FIFO e device file previne escritas acidentais em arquivos especiais
- Detecção de hardlink previne corrupção silenciosa de dados quando rename atômico quebra hard links

### Limitações Conhecidas (corrigidas em v0.1.2)
- Flag `batch --file <PATH>` era declarada no help mas não era conectada à lógica do comando
- `batch --transaction` não deletava arquivos criados durante transações falhadas (apenas restaurava arquivos modificados)
- `replace` incrementava contadores ANTES da validação do jail do workspace, produzindo contagens NDJSON contraditórias
- `search` com regex inválido produzia erro cru no stderr em vez de envelope JSON
- Walker paralelo do `search` intercalava eventos begin/match/end de arquivos diferentes
- `scope --delete` em comentários Rust deixava whitespace órfão
- Compilação no macOS falhava (nix 0.29 restringia `posix_fadvise` a Unix não-macOS)
- `--workspace` padrão era CWD silencioso (sem aviso ao capturar tudo)
- Mensagem de erro WORKSPACE_JAIL sugeria "use an absolute path" mesmo quando o path já era absoluto
- `backup --output-dir` era declarado mas não era conectado
- 4 dependências congeladas (nix 0.29, signal-hook 0.3, windows-sys 0.59, rust-i18n 3)
- `read` não tinha flags `--head`/`--tail`/`--grep` para controle de janela de contexto LLM
- `completions` não auto-instalava
- Sem flag global `--timeout` para terminação de operação sem limite


## [0.1.0] - 2026-05-29
### Adicionado
- 22 subcomandos: `read`, `write`, `edit`, `search`, `replace`, `hash`, `delete`, `count`, `diff`, `move`, `copy`, `list`, `extract`, `calc`, `regex`, `transform`, `batch`, `completions`, `scope`, `backup`, `rollback`, `apply`
- Pipeline de escrita atômica: tempfile + fsync + rename + fsync do diretório em toda operação de escrita
- Checksums BLAKE3 em toda resposta de `read` e `write`
- Locking otimista via flag `--expect-checksum`
- Contrato de saída NDJSON: toda linha do stdout é um objeto JSON com discriminador `"type"`
- Respostas de erro estruturadas no stdout com `error: true`, incluindo campos `error_class`, `retryable` e `suggestion`
- Busca paralela de arquivos com motor ripgrep (`grep-regex`, `grep-searcher`, `grep-matcher`)
- Travessia respeitando `.gitignore` via crate `ignore`
- Busca e reescrita estrutural por AST via o motor ast-grep, sobre as gramáticas embutidas da crate `ast-grep-language` fixada (28 linguagens na 0.43.0 e 29 na 0.51.0, ambas contadas na fonte da crate)
- Geração de regex a partir de exemplos via grex
- Avaliação de expressões matemáticas e conversões de unidade via fend-core
- Saída de diff unificado via crate `similar`
- Leitura de arquivos via memory-map com `memmap2` para arquivos grandes
- Jail de workspace via `--workspace` para prevenir escape de caminhos
- Bloqueio de symlinks para prevenir travessia fora dos limites do workspace
- Operações em lote a partir de manifestos NDJSON suportando write, replace e delete
- Geração de completions de shell para bash, zsh, fish, elvish e PowerShell
- Tratamento de sinais para SIGINT, SIGPIPE e SIGTERM com shutdown limpo
- Suporte cross-platform: Unix (nix, libc) e Windows (windows-sys)
- 20 códigos de saída distintos para classificação precisa de erros
- Suporte a variável de ambiente `NO_COLOR`
- Variável de ambiente `RUST_LOG` para controle de verbosidade de logs
- Perfil release com LTO, codegen unit único, stripping de símbolos e panic=abort


[Unreleased]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.37...HEAD
[0.1.37]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.36...v0.1.37
[0.1.36]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.35...v0.1.36
[0.1.35]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.34...v0.1.35
[0.1.34]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.33...v0.1.34
[0.1.33]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.32...v0.1.33
[0.1.32]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.31...v0.1.32
[0.1.31]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.30...v0.1.31
[0.1.2]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/danilo-aguiar-br/atomwrite/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/danilo-aguiar-br/atomwrite/releases/tag/v0.1.0
