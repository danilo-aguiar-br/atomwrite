# ADR-0089 — o alvo é sempre array, e o batch nomeia as cópias dele


## Status
- Aceita (2026-11-03)
- Emenda a ADR-0088, que manteve a forma de string para um em todo verbo menos no registro de rollback
- Emendada pela ADR-0090, que implanta as duas pendências que este registro deixou para uma rodada seguinte


## Contexto
- `set_target` gravava string, `set_targets` gravava string para um sujeito e array para vários, e o envelope de erro repetia a mesma regra em `targets_value`, então um campo carregava dois tipos
- O consumidor precisava ler a contagem antes de escolher como ler o campo, e o schema declarava a união das duas formas
- `BatchOpResult` não tinha `warnings`, então o `--keep-backup` redundante avisava no `delete` isolado e sumia dentro do batch
- `execute_delete` e `execute_move` descartavam o `PathBuf` que `create_backup` devolvia, então a cópia existia no disco e nenhum registro a nomeava
- `execute_move` pulava a cópia quando o destino já era um backup e não preenchia `backup_skipped`, enquanto o `delete` do mesmo batch declarava essa recusa
- `destructive_paths` casava só `delete`, então o commit descartava o snapshot de um destino que `move` ou `copy` sobrescreveu
- O resumo de uma transação confirmada não nomeava nem os snapshots que guardou nem as cópias que removeu, e o chamador listava o diretório para achar
- O rollback do `apply` restaura conteúdo e não remove nada, então um `.bak` feito para um arquivo confirmado sobrevivia sem nome


## Decisão
- `target_resolved` é sempre array, produzido por `set_target` e por `set_targets` igualmente, e `set_targets_array` deixa de existir
- `targets_value` sempre devolve array, e o schema publicado declara um `array` de strings em vez da união
- `BatchOpOutcome` e `BatchOpResult` carregam `backup_path` e `warnings`, e o registro da operação publica os dois
- `execute_delete` preenche `warnings` com `KEEP_BACKUP_REDUNDANT_FOR_DELETE` antes do retorno de prévia, então `--dry-run` também responde à flag
- `execute_move` preenche `backup_skipped` com `BACKUP_OF_BACKUP_SKIPPED` quando o destino é um backup, e `backup_path` quando fez cópia
- `destructive_paths` casa os alvos de `delete` e o destino de `move` e de `copy`; a origem do `move` fica fora, porque o conteúdo dela vive no destino
- O commit coleta os snapshots que sobreviveram a ele, e `BatchSummary` os publica como `backups_preserved`
- `rollback_after_failure` recebe o `WriteResult` de cada arquivo confirmado, coleta o `backup_path` cujo arquivo ainda existe e o publica em `ApplyRollbackEvent.backups_preserved`


## Consequências
- O consumidor de qualquer envelope terminal lê uma forma só para `target_resolved`, e a contagem deixa de trocar o tipo
- O agente acha o desfazer de uma operação do batch e as cópias que um `apply` falho deixou sem listar o diretório
- A transação relata o snapshot que guardou, então o caminho de recuperação viaja na saída em vez de só no sistema de arquivos
- Um destino sobrescrito dentro da transação guarda o conteúdo anterior, porque o commit deixa de tratá-lo como resíduo
- A mudança quebra o consumidor externo da forma antiga de string, o que `0.y.z` permite e o `docs/MIGRATION.pt-BR.md` cobre com a receita de leitura


## Alternativas
- Manter a união e aceitar as duas formas: recusada, porque um campo cujo tipo depende de uma contagem que o chamador ainda não leu obriga um ramo em todo consumidor
- Manter a string nos verbos isolados e o array só no rollback: recusada pelo dono, que escolheu uma forma para todo verbo
- Suprimir a cópia da operação quando a transação já tirou snapshot do mesmo arquivo: recusada nesta rodada, porque mudaria o significado de `--backup` dentro da transação, e a duplicação fica declarada como pendência
- Publicar a cópia da escrita falhada no braço de erro do `apply`: recusada nesta rodada, porque `commit_target` não devolve o `WriteResult` dela na falha e o valor não é alcançável sem mudança maior
- Guardar o snapshot da origem de um `move`: recusada, porque o conteúdo é legível no destino e a cópia dobraria bytes sem acrescentar caminho de volta


## Gatilho para revisitar
- Um chamador que ainda precise da forma de string, o que reabre o contrato com um campo versionado
- Uma transação que pague duas cópias pelo mesmo arquivo, o que reabre o significado de `--backup` dentro da transação
- Uma falha de `apply` que deixe um `.bak` sem nome, o que reabre o braço de erro de `commit_target`
