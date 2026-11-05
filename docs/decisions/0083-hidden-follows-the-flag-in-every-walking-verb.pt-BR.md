# ADR-0083 — `--hidden` segue a flag em todo verbo que caminha


## Status
- Aceita (2026-11-03)
- Emenda a ADR-0082, que deixou `hash` e `copy` fora da regra de ocultos, e completa a ADR-0077, que é dona do walk compartilhado


## Contexto
- `hash` caminhava com `WalkerFlavor::AlwaysVisitHidden` em `src/commands/hash.rs`, e esse sabor chega a `src/commands/mod.rs` como `.hidden(false)`
- `semantic-search` usava o mesmo sabor em `src/commands/semantic_search.rs`, e `copy -r` usava `WholeTree`, cujo nome prometia a visita de tudo
- O crate `ignore` liga o filtro de ocultos por padrão, então um verbo que fixa `hidden(false)` torna a flag global inerte por construção, e o chamador não distingue um dotfile coberto de um dotfile ausente
- O comentário em `src/commands/mod.rs` justificava o sabor dizendo que um hash ou uma cópia de árvore que pulasse `.env` responde a pergunta errada, e esse raciocínio valia enquanto nenhuma flag pedia o dotfile de propósito
- A regra do alvo explícito já respondia o caso que o comentário protegia: um caminho nomeado na invocação é lido qualquer que seja a política do walk
- `sparse list` mudou o padrão na rodada anterior e não deixou caminho de volta dentro do verbo para quem queria o inventário oculto


## Decisão
- `hash` e `semantic-search` usam `RespectGlobalFlags`, então `--hidden` decide e o padrão esconde dotfile
- `WholeTree` passa a se chamar `CopyTree`, e ela devolve `!global_hidden` em `skips_hidden` e mantém `honors_gitignore` falso, então `copy -r` continua copiando o que o `.gitignore` lista
- Um caminho nomeado na invocação continua sendo lido sem a flag, porque a invocação já nomeou o sujeito
- `sparse list` e `sparse outline` aceitam `--all`, que seleciona `AlwaysVisitHidden` dentro do verbo, e o padrão continua escondendo
- O novo nome é seguido em todo lugar onde a variante aparece, então nenhum sítio carrega a promessa antiga no nome


## Consequências
- Um script que hasheava, copiava ou buscava uma árvore sem `--hidden` agora recebe o inventário menor, e isso é mudança incompatível publicada no changelog e no guia de migração
- A mesma árvore responde a mesma contagem de arquivos em todo verbo que caminha, então um número de um prevê os outros
- `copy -r` continua copiando o caminho listado no `.gitignore`, porque a resposta da cópia é a árvore que o chamador nomeou e não a árvore que o git rastreia
- `hash .env` e `copy .env` continuam funcionando, então a regra do alvo explícito sobrevive à mudança de política
- Quem quiser o inventário antigo do `copy -r` passa `--hidden`, e quem quiser o inventário antigo do `sparse list` passa `--all`


## Alternativas
- Manter o sabor e documentar que `--hidden` é inerte nos três verbos: rejeitada, porque flag que não muda nada é a divergência que esta rodada fecha
- Remover `--hidden` da superfície global e deixar cada verbo com a própria política: rejeitada, porque multiplicaria a política e deixaria a regra do alvo explícito sem documento
- Fazer `copy -r` honrar `.gitignore` na mesma mudança: rejeitada, porque a resposta da cópia sempre foi a árvore nomeada, e esta rodada trata só da regra de ocultos


## Gatilho para revisitar
- Um verbo cuja resposta precisa incluir dotfile mesmo quando ninguém pediu, o que exigiria sabor próprio e razão própria
- Uma versão do `ignore` que mude o padrão do filtro de ocultos, o que faria esta decisão descrever comportamento que o crate não tem mais
