# ADR-0087 — o snapshot da transação segue a política de backup e o rollback nomeia os alvos


## Status
- Aceita (2026-11-03)
- Estende a ADR-0086, que publicou o registro de rollback e o caminho de backup do `apply`
- Estendida pela ADR-0088, que mantém o snapshot de um alvo que a transação excluiu e faz o registro de rollback nomear as cópias que uma reversão falhada preservou


## Contexto
- O `snapshot_file` tirava uma cópia com `create_backup` de todo alvo antes do `batch` transacional e nunca consultava `--no-backup`, `--keep-backup` nem `--backup`
- Nada removia esses snapshots depois do commit bem-sucedido, então `batch --transaction --no-backup` largava `f1.txt.bak.<carimbo>` ao lado de cada arquivo que acabara de gravar
- O snapshot é a única fonte de onde o `rollback_transaction` restaura, então ele não pode ser pulado no início sem perder o desfazer da transação
- O `delete_backup_quietly` já trata cópia ausente como sucesso e nunca mascara a escrita confirmada, então a remoção não precisa de regra nova
- O registro `rollback` do `apply` e do `batch --transaction` não carregava `target_resolved`, e o writer carimba um valor único que nenhum dos dois verbos tinha definido antes da reversão
- O writer já publica vários sujeitos como array por `set_targets`, e o schema publicado já aceita string ou array em `target_resolved`


## Decisão
- A transação sempre tira o snapshot antes da mutação, porque ele é o caminho do desfazer, e o remove quando a execução termina
- A condição de remoção é `!keep_backup || no_backup`: `--keep-backup` guarda a cópia, e `--no-backup` vence sobre ele, a mesma precedência que as operações já usam
- A remoção roda depois que toda operação confirma e depois de um rollback que restaurou tudo, por `delete_backup_quietly`
- O rollback que falha mantém os snapshots, porque eles são o último estado conhecido de arquivos cuja recuperação deixou de ser conhecida
- O `rollback_after_failure` do `apply` define os alvos revertidos no writer antes de publicar, e o rollback do batch faz o mesmo, com a origem `argv` ou `stdin` do manifesto
- Os alvos são ordenados e deduplicados e viajam como array, na ordem em que o rollback os tocou
- Nenhum struct ganha o campo, porque o writer já é o único lugar que carimba `target_resolved`, e o schema já descreve o array


## Consequências
- `batch --transaction --no-backup` deixa no workspace exatamente os arquivos que o manifesto mudou, e nenhum resíduo
- A transação padrão e todos os outros verbos concordam numa regra: a cópia é transitória a menos que a invocação peça para guardar
- Um agente que recebe o registro de rollback acha todo caminho revertido em `target_resolved`, sem uma segunda chamada para listar o diretório
- Um `target_resolved` com um elemento continua sendo string, que é a forma que o writer já publica


## Alternativas
- Pular o snapshot quando `--no-backup` está presente: rejeitada, porque o snapshot é a única fonte do desfazer da transação, e uma transação que não pode ser desfeita é pior que um arquivo transitório
- Guardar o conteúdo original em memória: rejeitada, porque coloca o tamanho de todo alvo na RAM e quebra a regra de memória limitada que o batch segue
- Acrescentar `target_resolved` nos dois structs de rollback: rejeitada, porque nenhum struct de saída declara chave carimbada, e o writer já publica array
- Manter o snapshot sempre e documentar: rejeitada, porque contradiz a ADR-0038 e a promessa de que a cópia some no sucesso


## Gatilho para revisitar
- Um snapshot guardado em diretório fora do workspace, que mudaria onde a cópia do desfazer mora
- Um rollback que relate alvos por arquivo em vez da lista resolvida, que exigiria outro campo
- Uma transação sobre alvo cujo diretório recusa a cópia mas permite a escrita
