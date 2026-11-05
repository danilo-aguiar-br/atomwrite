# ADR-0090 — a transação é dona da cópia, e a escrita que falha nomeia o que deixou


## Status
- Aceita (2026-11-03)
- Emenda a ADR-0089, que declarou a cópia duplicada e a cópia não nomeada da falha como pendências para uma rodada seguinte


## Contexto
- Um `batch --transaction` fotografa todo alvo antes da primeira mutação, e o `execute_delete` e o `execute_move` criavam outra cópia do mesmo arquivo, então duas cópias carimbadas de um arquivo sobreviviam ao commit
- A cópia da operação não entrava na lista de snapshots, então nenhuma regra do commit a removia, e o `backup_path` nomeava só ela enquanto o resumo nomeava a outra
- O `atomic_write_based` criava a cópia no meio da escrita e só a anexava ao `WriteResult` no fim, então uma falha no intervalo deixava o arquivo no disco sem registro que o nomeasse
- O `rollback_after_failure` recebia só o `WriteResult` dos arquivos que deram certo, então a cópia do arquivo que falhou nunca chegava a `backups_preserved`
- O `apply` prepara todo alvo antes da primeira escrita, então bloco que não casa nunca chega ao commit, e a falha no segundo arquivo que os check-points precisavam não tinha indutor permitido


## Decisão
- O `BatchOpOptions` carrega a lista de snapshots da execução por empréstimo e não por cópia, então o fecho da fan-out continua `Copy` e nenhum caminho é clonado por operação
- O `execute_delete` e o `execute_move` procuram o alvo que copiariam e, quando a transação já o guarda, publicam esse caminho em `backup_path` e respondem `TRANSACTION_SNAPSHOT_SUPERSEDES_COPY` em `warnings` em vez de escrever um segundo arquivo
- Fora da transação a lista é vazia, então a operação escreve a própria cópia exatamente como antes
- O `atomic_write_based` coleta a cópia que cria num parâmetro de saída e a anexa a qualquer falha como contexto do anyhow, então o erro tipado embaixo continua alcançável por downcast e todo outro chamador segue propagando sem mudança
- O `apply` lê o caminho da falha, acrescenta uma frase ao erro nomeando onde a cópia ficou, e passa o caminho ao `rollback_after_failure`, que o publica depois das cópias dos arquivos confirmados
- A descrição de `backups_preserved` cobre a cópia do arquivo que falhou, e os 54 schemas publicados foram regerados pelo binário


## Consequências
- Um `delete` ou um `move` dentro da transação deixa uma cópia no disco, e a execução a nomeia duas vezes com o mesmo caminho, uma no registro da operação e uma no resumo
- O `--keep-backup` dentro da transação guarda uma cópia em vez de duas, que é justamente o caso em que os bytes importam
- Um `apply` que falha depois de criar cópia a nomeia na mensagem de erro, e também no registro de reversão quando algum arquivo já tinha sido confirmado
- O destino de `write` e de `replace` continua sem cópia, por decisão do dono nesta rodada, então a assimetria com as operações destrutivas fica como a ADR-0088 fixou


## Alternativas
- Tirar `delete` de `destructive_paths`, para o commit descartar o snapshot da transação e sobreviver a cópia da operação: rejeitada, porque as duas cópias continuariam sempre que o `--keep-backup` estiver ligado, que é exatamente o caso para o qual a regra existe
- Apagar a cópia quando a escrita falha: rejeitada, porque a estratégia em lugar pode falhar no meio com o alvo já modificado, e aí a cópia é o único caminho de volta
- Nomear a cópia da falha varrendo o diretório: rejeitada, porque um nome carimbado não se reconstrói, e a varredura nomearia cópia criada por outra execução
