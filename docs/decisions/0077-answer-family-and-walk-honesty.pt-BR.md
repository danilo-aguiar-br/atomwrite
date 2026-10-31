# ADR-0077 — Família de resposta, honestidade do arquivo de ignore e carimbo de alvo nos verbos de leitura

## Status
Aceito (2026-10-30)

## Contexto
- A auditoria da 0.1.37 mediu o `scope` publicando `files_matched` como `modified + failed`, então um diretório que nunca foi lido contava como casamento
- `transform --rules` varre a árvore uma vez por regra, então um arquivo ilegível falhava uma vez por regra e o `errors[]` repetia o mesmo caminho
- `.gitignore` com sintaxe quebrada ou modo ilegível encerrava a execução com exit 0, `ok: true` e nada no stderr, porque o `ignore` guarda esse erro dentro do próprio valor
- `case` saía 13 depois de renomear um arquivo, enquanto `search`, `replace`, `transform` e `scope` respondiam com exit 0 na mesma situação
- `read`, `count`, `list`, `hash` e `diff` não publicavam `target_resolved` nem `target_source`, então o agente não confirmava o caminho que leu

## Decisão
### 1. Casamento é arquivo que respondeu
- O `scope` mantém um contador dedicado `matched_files`, incrementado onde `scopes_matched` é somado, e o publica como `files_matched`
- Arquivo que falhou, arquivo pulado e arquivo sem casamento nunca chegam a esse contador, então `files_matched` nunca passa de `files_visited`
- A mesma regra já valia no `search`, onde o arquivo só conta quando publicou registro de resposta

### 2. Uma falha por arquivo distinto, qualquer que seja o número de regras
- `transform --rules` mantém um `BTreeSet` de caminhos falhos, e `files_failed` é o tamanho desse conjunto mais as falhas que nenhuma entrada pôde nomear mais as raízes ausentes
- O `errors[]` lista cada caminho uma vez, e o evento `rule_error` por regra continua nomeando a regra que falhou, então a deduplicação nunca esconde qual regra quebrou
- `files_visited`, `files_matched` e `total_replacements` continuam somando por regra, porque cada regra é uma passagem completa, e a documentação diz isso

### 3. Arquivo de ignore que não pode ser lido é falha reportada
- `ignore_file_failures(roots)` lê o `.gitignore` e o `.ignore` do topo de cada raiz com `GitignoreBuilder::add`, que devolve o erro de sintaxe ou de I/O com o caminho do arquivo anexado
- Erro de sintaxe é `INVALID_INPUT` e arquivo ilegível é o código de I/O, ambos registrados pelo `FailureTally` com o caminho do próprio arquivo de ignore
- A checagem roda uma vez por raiz, antes da varredura, e cobre os onze verbos que já carregam tally
- O `scope` registra a mesma falha no contador atômico e no próprio canal de eventos, então o verbo reporta sem tally

### 4. A regra de família é a resposta, não o verbo
- Verbo cuja resposta é por item mantém exit 0 e publica `partial: true` quando publicou ao menos um registro de resposta: `search`, `replace`, `transform`, `case`, `scope`, `sparse`, `semantic-search` e `backup`
- Verbo cuja resposta é total sobre a árvore sai com a primeira falha, porque total calculado sobre árvore parcial é falso: `count`, `list`, `hash`, `copy`, `delete` e `prune-backups`
- Raiz nomeada na invocação que não existe continua saindo 4 em todo verbo, mesmo quando o resto casou
- A guarda é a resposta do próprio verbo, nunca uma flag: `case` e `scope` exigem `files_modified > 0`, `sparse` exige ao menos uma entrada emitida, `semantic-search` ao menos um hit, e `backup` ao menos uma cópia em disco
- O `scope` ganhou o escape que o `transform` já tinha, então reescrita que aconteceu não é descartada por erro de escrita posterior, e o `backup` deixou de perder o resumo quando uma origem falhou
- O `prune-backups` fica na família de total: ele responde "quantos backups saíram", e esse total é falso enquanto uma remoção falhou

### 5. Verbos de leitura carregam o mesmo carimbo de alvo
- `read_target_roots` devolve as raízes de `read`, `stat`, `count`, `list`, `verify`, `get`, `query`, `outline`, `sparse`, `semantic-search`, `watch`, `extract-artifact`, `hash` e `diff`, e `resolve_read_targets` as resolve por `resolve_mutation_targets`
- `target_source` é `argv` para toda raiz digitada e `workspace` para o padrão `.` que `count`, `list`, `sparse list`, `sparse outline`, `semantic-search` e `watch` carregam
- `hash --stdin` carimba `-` com `target_source: "stdin"`, porque o sujeito da execução é o fluxo e nomeá-lo é mais expressivo que o silêncio
- Um operando `-` do `diff` fica como token em vez de ser resolvido como caminho do workspace, então `diff a.txt -` nomeia um arquivo e um fluxo juntados por `,` na ordem de argv
- O envelope de erro consulta o mesmo resolvedor, então execução que falha nomeia o sujeito com a mesma origem que a execução bem-sucedida usa, em vez de cair para `argv`
- Verbo sem caminho próprio fica sem carimbo e é nomeado como tal: `extract`, `calc`, `regex`, `batch`, `agent-surface`, `doctor`, `commands`, `completions` sem `--install`, `wal-stats`, leitura de `locale` e `recipe list`

### 6. A descrição do campo descreve todo produtor
- `files_skipped` diz que a execução não respondeu aquele arquivo, e nomeia os casos que os verbos usam
- `files_visited` diz total de arquivos examinados em todo lugar, e `search --target files` conta o que a varredura entregou em vez dos nomes que casaram
- `files_matched` é donde vive a contagem dos nomes que casaram, então a página omitida por `--offset` e `--limit` fica visível como `files_matched` contra `total_matches`
- `files_modified` nomeia todo verbo que o publica: `replace`, `transform`, `case`, `scope`, `codemod`, `delete` e `copy`
- `total_matches` nomeia as substituições do `transform` e os escopos casados do `scope`, que são os registros de resposta que esses verbos publicam
- O `list` publica `total_bytes` em toda execução, e a descrição não afirma o contrário
- O `delete` publica `files_skipped` como a contagem pulada por idade, e a expressão tautológica saiu

### 7. O inventário de gaps não é ponteiro vivo
- `gaps.md` saiu do `include` do pacote, e `**/gaps.md` fica no `exclude`, porque nota privada nunca é empacotada
- Todo documento que o citava como fonte viva agora aponta para a série de ADRs, e as menções históricas do `MIGRATION` continuam como narrativa de releases antigas

### 8. A regra vale em todo modo do mesmo verbo
- O `transform` de uma regra respondia exit 0 quando uma substituição foi feita e outro arquivo falhou, enquanto o `transform --rules` saía com o código da primeira falha na mesma árvore
- O caminho multi-regras agora carrega o mesmo escape, protegido por `total_replacements > 0`, e o `errors[]` continua nomeando todo arquivo que ele não tocou
- Execução que não reescreveu nada mantém a primeira falha, porque campanha sem resposta não tem o que preservar

### 9. A lista de falhas é orçamento, não constante
- `errors[]` para em `summary.errors_max_entries`, 50 por padrão, e `files_failed` guarda sempre a contagem total
- A chave aceita de 1 a 1000, então quem precisa de mais causas pede, e quem quer menos corta o registro
- O teto existe porque lista sem limite deixa uma árvore quebrada empurrar megabytes de texto de falha num registro só, que é o custo que o teto evita

## Consequências
- Agente que soma `files_matched` e `files_visited` recebe dois números comparáveis, e `files_matched` nunca conta falha
- Campanha dividida em mais regras deixa de inflar `files_failed`, então orçamento de falha significa a mesma coisa em qualquer número de regras
- Árvore com arquivo de ignore quebrado agora diz isso, com o caminho do arquivo de ignore e a linha que quebrou
- O `case` deixa de jogar fora a renomeação que realizou, e o CI de quem chama para de falhar por um arquivo que a execução nem tocou
- Campanha de várias regras mantém o mesmo código de saída da mesma campanha com uma regra, então dividir o trabalho nunca muda o que o código de saída significa
- Os cinco verbos de leitura respondem a pergunta de ETD que os verbos de escrita já respondiam
