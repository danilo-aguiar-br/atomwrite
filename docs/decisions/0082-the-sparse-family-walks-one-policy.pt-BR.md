# ADR-0082 — a família sparse caminha por uma política só


## Status
- Aceita (2026-11-03)
- Emenda a ADR-0077, que é dona da política de walk compartilhada, e refina a medição citada pela ADR-0080
- Emendada pela ADR-0083, que trouxe `hash`, `copy -r` e `semantic-search` para a mesma regra e deu aos verbos sparse o próprio `--all`


## Contexto
- `sparse list` caminhava com `WalkerFlavor::AlwaysVisitHidden` em `src/commands/sparse.rs`, e esse sabor chega a `src/commands/mod.rs` como `.hidden(false)`, então `--hidden` não mudava nada nesse verbo
- Na pasta que a ADR-0080 mediu, `sparse list` respondeu 3 enquanto `sparse outline` respondeu 2, e os dois saíram 0, então a divergência era silenciosa
- `sparse outline` e `sparse list` mantinham a cópia `.bak.` com carimbo enquanto `semantic-search` e `recipe` já a descartavam por `is_backup_path` em `src/commands/backup_names.rs`
- A descrição de `files_scanned` falava só do orçamento de `--max-files`, então quem lê o schema não previa a diferença entre rodar com e sem `--hidden`
- `hash` e `copy` também ignoram `--hidden`, e o comentário deles diz o motivo: um hash ou uma cópia de árvore que pulasse `.env` responderia pergunta que o chamador não fez


## Decisão
- `sparse list` usa `RespectGlobalFlags`, então a regra comum de ocultos decide e `--hidden` é o chamador pedindo todas as entradas
- `sparse list` e `sparse outline` descartam todo caminho que `is_backup_path` aceita, antes do encontro de `--include`, com e sem `--hidden`
- A exclusão é o mesmo predicado dos outros verbos de varredura, então a regra mora numa função em vez de duas
- A descrição de `files_scanned` diz que dotfile só conta sob `--hidden`
- `hash` e `copy` mantêm o sabor deles, porque a resposta deles é sobre a árvore inteira e esta decisão não os alcança


## Consequências
- Um `sparse list` padrão passa a esconder dotfile, o que é mudança de comportamento para quem contava com a listagem antiga para inventariar um diário ou um `.env`
- Os três verbos sparse respondem a mesma árvore, então a contagem de um prevê a dos outros dois
- Uma cópia de backup deixa de ocupar vaga de `--max-files` e de chegar ao leitor como se fosse fonte
- A descrição do schema responde a pergunta que o número levanta, e `docs/schemas/sparse.schema.json` é regravado do binário
- A frase da ADR-0080 que reporta `sparse list` com 3 descreve a árvore antes desta mudança, e `sparse list --hidden` continua respondendo 3 nela


## Alternativas
- Manter `AlwaysVisitHidden` e documentar que `--hidden` é inerte: rejeitada, porque flag que não muda nada é a divergência silenciosa que esta rodada fecha
- Acrescentar `--no-hidden` como segundo interruptor: rejeitada, porque `--hidden` já existe e um par de interruptores para uma regra é superfície nova
- Excluir backup só no `sparse outline`: rejeitada, porque criaria entre os dois verbos exatamente a divergência que a primeira decisão remove


## Gatilho de revisão
- Um verbo sparse que precise ver dotfile por padrão, o que dividiria a família de novo e exigiria motivo próprio
- Um esquema de nome de backup que `is_backup_path` não reconheça, o que faria a exclusão vazar cópias
