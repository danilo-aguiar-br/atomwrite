# ADR-0088 — a exclusão guarda o próprio desfazer, a reversão declara o que preservou, e o apply nomeia o fluxo


## Status
- Aceita (2026-11-03)
- Estende a ADR-0087, que removia todo snapshot depois do commit e nomeava os caminhos revertidos


## Contexto
- `collect_target_paths` tira snapshot de todo alvo de uma transação, inclusive do alvo de uma operação que o exclui
- O commit descartava todo snapshot por `discard_snapshots`, então uma transação que excluía um arquivo removia o arquivo e a única cópia dele, mesmo sem `--no-backup`
- O `delete` standalone trata a cópia como artefato de recuperação e avisa que `--keep-backup` é redundante, então o batch e o verbo standalone discordavam sobre para que serve a cópia de uma exclusão
- Uma reversão que parava no meio não publicava registro `rollback` nenhum: o processo respondia com o envelope genérico de erro, e os snapshots deixados no disco não tinham nome na saída
- `rollback_transaction` devolvia só um erro, então a contagem parcial dos arquivos que voltaram se perdia
- `cmd_apply` sempre lê o patch da entrada padrão, e o patch pode trazer a própria linha de caminho, mas os três carimbos creditavam `argv`
- `set_targets` publica um sujeito só como string, então a forma do rollback dependia de quantos arquivos voltaram


## Decisão
- O commit da transação descarta o snapshot de todo alvo que ela reescreveu e mantém o snapshot de todo alvo que ela excluiu, porque esse arquivo não vive em nenhum outro lugar
- `--no-backup` continua vencendo e remove todo snapshot, e `--keep-backup` continua mantendo todos, então a flag mantém o significado
- Uma reversão bem-sucedida descarta todo snapshot, porque o arquivo excluído voltou ao disco e a cópia virou resíduo de novo
- A reversão relata a verdade parcial: `rollback_transaction` devolve as contagens que alcançou, o primeiro caminho que não conseguiu restaurar e o diagnóstico, em vez de só um erro
- Uma reversão que não consegue restaurar um arquivo publica um registro `rollback` pelo caminho de falha, com `failed_paths` nomeando o que não voltou e `backups_preserved` nomeando os snapshots que ficaram
- O `apply` carimba `stdin` quando o patch nomeia o próprio arquivo e `argv` quando `--file` nomeia o alvo na linha de invocação, tanto no registro de plano quanto nos registros aplicados e no registro de rollback
- O registro `rollback` do `apply` e do `batch --transaction` publica sempre `target_resolved` como array, por `set_targets_array`; todo outro verbo mantém a forma string-para-um de `set_targets`


## Consequências
- Uma transação que exclui um arquivo deixa `f1.txt.bak.<carimbo>` ao lado do buraco, que é o caminho de volta que o `delete` standalone já prometia
- Uma reversão que falha nomeia as cópias que manteve, então o agente lê os caminhos de recuperação no registro em vez de listar o diretório
- Quem consome o registro de rollback trata uma forma só e deixa de ramificar pela contagem
- Um agente que lê `target_source` aprende de onde o alvo realmente veio, e nenhum envelope credita uma linha que quem chamou nunca escreveu
- O schema publicado do `batch` ganha dois campos de array, e o do `apply` não muda porque a forma que ele já aceitava continua aceita


## Alternativas
- Remover a cópia da exclusão no sucesso por uniformidade: rejeitada, porque o arquivo não existiria em lugar nenhum e a execução destruiria dado para satisfazer uma regra escrita para reescritas
- Manter todo snapshot depois do commit: rejeitada, porque uma reescrita deixaria resíduo que `--no-backup` e a ADR-0038 proíbem
- Publicar só o envelope de erro quando a reversão falha: rejeitada, porque quem chama não consegue saber quais cópias sobreviveram, e estado escondido é a falha que este projeto pune
- Fazer todo verbo publicar array: rejeitada, porque cinco testes existentes e vários consumidores leem o campo como string, e a mudança quebraria o contrato de quarenta verbos para consertar um evento
- Carimbar `argv` para um patch que nomeia o próprio arquivo: rejeitada, porque a linha de invocação não carregava caminho nenhum e quem chama não consegue verificar uma origem que nunca existiu


## Gatilho para revisitar
- Uma operação do batch que destrói um arquivo e expõe o mesmo snapshot por campo próprio do registro dela
- Um consumidor que precise do caminho da cópia do arquivo excluído sem ler `backups_preserved`
- Um `move` cuja remoção da origem precise do mesmo tratamento da exclusão
