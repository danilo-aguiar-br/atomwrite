# ADR-0091 — a transação empresta a cópia dela para uma reescrita


## Status
- Aceita (2026-11-03)
- Emenda a ADR-0088 e a ADR-0090, que emprestaram o snapshot só para `delete` e `move`


## Contexto
- Uma execução transacional tira o snapshot de todo alvo antes da primeira mutação, e esse snapshot é o caminho de volta de toda operação
- `execute_delete` e `execute_move` já pediam a cópia à transação em vez de escrever uma segunda
- `execute_write`, `execute_replace`, `execute_edit` e `execute_copy` nunca receberam a lista de snapshots, então cada uma criava a própria cópia
- Com `--keep-backup` a execução deixava duas cópias dos mesmos bytes, e a operação publicava um caminho enquanto `backups_preserved` publicava o outro
- `collect_target_paths` também tirava snapshot do source de `move` e `copy`, que nenhum caminho de rollback lê
- `destructive_paths` nomeava só `delete`, `move` e `copy`, então a reescrita sobrevivia ao commit por acidente do `else if`


## Decisão
- Um helper só, `copy_decision`, responde à pergunta da cópia para toda operação que reescreve um alvo
- Quando existe snapshot e `--keep-backup` pede a cópia, o writer roda com `backup: false`, o registro publica o caminho do snapshot, e `warnings` diz que a transação já preservou o alvo
- Quando existe snapshot e a cópia é temporária, o writer também roda com `backup: false`, o registro não publica caminho, e o commit descarta o snapshot
- Quando não existe snapshot, o writer cria a cópia como antes, e `backup_path` vem do writer
- Um alvo que já é backup não ganha segunda cópia, pela mesma regra que o `delete` já aplicava
- O commit sempre chama `discard_residue`, e `destructive_paths` nomeia uma reescrita só quando `--keep-backup` pediu e `--no-backup` não proibiu
- A transação tira snapshot do destino de `move` e `copy` e deixa de tirar do source deles


## Consequências
- `--keep-backup` numa reescrita transacional deixa um arquivo, e `backup_path` é igual ao caminho que sobrevive em `backups_preserved`
- Sem `--keep-backup` nenhuma cópia sobrevive, e o registro deixa de prometer um caminho que o commit remove
- Uma execução de `move` ou `copy` deixa de escrever um snapshot de source que a reversão nunca lê
- Escrever num arquivo de backup continua sendo uma operação só naquele arquivo, sem cadeia `.bak.bak`
- A descrição publicada de `backup_path` passa a cobrir as duas origens do campo


## Medição
- Cenário: um `write` de um arquivo de 8 MB por `batch --transaction --keep-backup`, executado no Linux com o binário que o install colocou em `~/.cargo/bin/atomwrite`
- Antes desta mudança, o mesmo cenário deixava 2 arquivos casando `.bak.` no diretório e informava `elapsed_ms` 11 no summary
- Depois desta mudança, o mesmo cenário deixa 1 arquivo de 8,0 MB e informa `elapsed_ms` 1 na operação e 7 no summary
- O binário anterior não está mais no disco, porque o install substituiu `~/.cargo/bin/atomwrite` e `target/release/atomwrite` em 2026-11-03 18:34, e o repositório não guarda fonte anterior
- A linha de base acima é a execução registrada na rodada que mediu, e a contagem de cópias é o número em que a decisão se apoia


## Alternativas
- Manter a cópia por operação e remover o snapshot: rejeitada, porque o snapshot é o caminho de volta da transação e existe antes da primeira mutação
- Publicar o caminho do snapshot mesmo quando o commit o remove: rejeitada, porque o agente perseguiria um caminho que não existe mais
- Deixar `destructive_paths` implícito: rejeitada, porque a regra dependeria da forma de um `if` e não de uma decisão nomeada
- Tirar snapshot do source de `move` e `copy` por simetria: rejeitada, porque nenhum caminho de rollback o lê e a cópia custa um arquivo inteiro


## Gatilho para revisitar
- Uma operação que muta o próprio source sem um rename que a reversão consiga desfazer
- Um verbo que precise da cópia da transação sem um `--keep-backup` que a mantenha
