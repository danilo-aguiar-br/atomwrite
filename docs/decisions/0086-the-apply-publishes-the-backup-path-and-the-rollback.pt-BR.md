# ADR-0086 — o apply publica o caminho do backup e o rollback


## Status
- Aceita (2026-11-03)
- Estende a ADR-0081, que nomeou o veredito do `apply`, e a ADR-0085, que levou o mesmo veredito aos verbos que reescrevem conteúdo
- Estendida pela ADR-0087, que faz o snapshot da transação seguir a política de backup e nomeia os alvos revertidos no registro de rollback


## Contexto
- O `apply` lia `write_strategy` e `backup_skipped` do `WriteResult` e descartava `backup_path`, embora o mesmo valor chegasse na mesma variável
- A v0.1.24 registrou o mesmo defeito no `move` como incidente: o `.bak` chegava ao disco e o evento nunca dizia onde ele estava
- Um patch que nomeia vários arquivos prepara todos e escreve em ordem, e os registros `applied` só saem depois do último commit, então uma falha no meio deixava o stream sem registro do que foi escrito ou desfeito
- O `rollback_committed` restaurava os arquivos e não relatava nada, e a restauração que falhava só chegava ao `tracing::error!`, então o chamador lia a falha de commit como se o disco estivesse no estado de antes
- O rollback rodava com `AtomicWriteOptions::default()`, que tem `backup: true` e `keep_backup: false`, então cada arquivo restaurado pagava uma cópia completa que era apagada em seguida
- O `docs/schemas/apply-result.schema.json` descrevia só o registro `applied`, embora o verbo também escreva um registro `plan` sob `--dry-run`
- Quinze campos opcionais do veredito de escrita declaravam `["string", "null"]` enquanto o binário os omite em vez de emitir `null`


## Decisão
- O `ApplyResult` publica `backup_path`, lido do `WriteResult`, e omite a chave quando cópia nenhuma foi feita
- Um `apply` que falha depois de escrever pelo menos um arquivo restaura os arquivos escritos e publica um registro `rollback` com `operation` igual a `apply`, a quantidade de arquivos restaurados e o tempo da restauração
- O registro sai por `publish_failure`, então carrega `ok: false` e o próprio `code` e `exit` da falha que forçou o rollback, e o `main` não escreve segundo envelope
- A restauração que falha nomeia o arquivo em `failed_paths` e transforma o erro devolvido em `TransactionRollbackFailed`, o mesmo desfecho que o `batch` usa, porque o estado do disco deixou de ser conhecido
- A execução que falha antes de escrever qualquer coisa não publica registro de rollback, porque nada foi desfeito e o envelope de erro já diz que a chamada falhou
- O rollback escreve sem backup, porque uma cópia do conteúdo que a execução produziu seria criada e apagada na mesma chamada
- A restauração passa o digest do conteúdo que encontra no disco, então o escritor atômico compara bytes em vez de inodes: a execução segura o lock do inode que o próprio commit substituiu, e a comparação de inode recusaria toda restauração com `StateDrift`
- O `batch --transaction` recebe o mesmo digest, porque o rollback dele batia na mesma recusa e deixava a árvore parcialmente revertida
- O schema do `apply` passa a ser a união de `plan`, `applied` e `rollback`, no arquivo `apply-result.schema.json`, que não precisa de nome novo
- Todo campo opcional do veredito de escrita declara `string` puro, e o `extend` do schemars muda o tipo sem acrescentar o campo a `required`
- Vetor que o binário omite quando vazio carrega `serde(default)`, então o schema publicado deixa de exigir chave que a execução não escreve; `failed_paths` e os `submatches` de um casamento de busca seguem essa regra


## Consequências
- Um agente que recebe falha do `apply` aprende em um registro que os arquivos voltaram, quantos voltaram e quais não voltaram
- O `.bak` de um patch é descobrível sem listar o diretório
- Um consumidor que valida o stream contra o schema publicado aceita o plano do dry-run e o registro de rollback
- O contrato publicado deixa de prometer `null` num campo que o binário nunca emite


## Alternativas
- Publicar os registros `applied` antes do rollback: rejeitada, porque esses registros descrevem um conteúdo que não existe mais depois do rollback, e os contadores do registro de rollback já dizem quantos voltaram
- Reusar o `RollbackEvent` do `batch`: rejeitada, porque aquele registro conta os arquivos que uma transação removeu e o rollback do `apply` restaura arquivos e falha por arquivo, então `failed_paths` seria campo que o `batch` nunca preencheria
- Emitir registro de rollback com zero arquivos restaurados: rejeitada, porque o envelope de erro já relata a falha e um registro sem nada desfeito é ruído
- Tirar cópia do conteúdo patcheado durante o rollback: rejeitada, porque o `keep_backup` é falso nesse caminho, então a cópia seria escrita e apagada na mesma chamada


## Gatilho para revisitar
- Um `--format full` que aceite mais de um alvo, o que deixaria o rollback criar e remover arquivos
- Um caminho de commit que falhe depois de escrita parcial, que o escritor atômico não tem hoje
- Um evento que precise carregar mais de um alvo resolvido na linha terminal
