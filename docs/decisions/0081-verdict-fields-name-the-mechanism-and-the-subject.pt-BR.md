# ADR-0081 — o veredito nomeia o mecanismo e o sujeito


## Status
- Aceita (2026-11-03)
- Emenda a ADR-0078, que fecha todo ramo terminal e por isso obriga um campo novo a chegar junto com o schema regravado


## Contexto
- `write` carregava `write_strategy` dentro do resultado da escrita e o descartava em `src/commands/write.rs`, então o chamador deduzia a estratégia pelo inode ou por `platform.rename_method`
- `platform.rename_method` responde `renameat2`, `rename` ou `MoveFileEx`, que nomeia a chamada de sistema e não separa escrita no lugar de troca
- `apply` terminava `commit_target` com `.map(|_| ())` em `src/commands/apply.rs`, então o `backup_skipped` que a camada atômica preenchia para uma cópia recusada nunca chegava ao evento
- Medido numa pasta sob `setfacl -m u::r-x`: `apply --backup` saiu 0 com `ok:true` e sem `backup_skipped` no JSON, enquanto o mesmo caso no `write` publicava o motivo
- Um arquivo novo nessa pasta saiu 13 e nomeou `.novo.txt.atomwrite.lock`, um sidecar que a execução cria e apaga, então a nova tentativa que a mensagem convida não alcança nada
- `src/lock.rs` classificava a falha com `from_io_at(&lock_path, error)` porque o sidecar era a única alça de um arquivo novo


## Decisão
- `write` publica `write_strategy` como campo obrigatório, e `apply` publica o mesmo campo mais um `backup_skipped` opcional
- Os dois vêm do resultado de escrita que a camada atômica já devolve, e nenhum cálculo novo entra
- A falha de lock é classificada contra o alvo, porque o alvo é o arquivo que o chamador pediu e o sidecar é o mecanismo
- O caminho do sidecar e o erro de I/O ficam numa linha `tracing::warn!` no stderr, que é onde diagnóstico pertence
- O código de saída não muda, porque `from_io_at` continua escolhendo a variante pelo `io::ErrorKind`


## Consequências
- O chamador prova a estratégia por um campo, e uma escrita no lugar fica distinguível de uma troca sem medir o inode
- `apply` deixa de esconder a cópia recusada, então o stdout sozinho carrega o veredito inteiro
- O caminho de erro nomeia um arquivo que existe, e o sidecar continua rastreável no log
- `docs/schemas/write-output.schema.json` e `docs/schemas/apply-result.schema.json` são regravados do binário, porque a ADR-0078 fecha o ramo
- `batch`, `copy`, `move`, `delete`, `rollback`, `semantic-merge` e `config` também descartam esses campos, e ficam para outra decisão


## Alternativas
- Publicar `write_strategy` só no `write` e deixar o `apply` calado: rejeitada, porque a causa é a mesma e a divergência entre irmãos é o que esta rodada fecha
- Manter o caminho do sidecar em `path` e acrescentar o alvo como segunda chave: rejeitada, porque `path` é o sujeito por contrato e dois sujeitos num erro é uma ambiguidade nova
- Deduzir a estratégia de `platform.rename_method`: rejeitada, porque ele nomeia a chamada de sistema e responde `renameat2` para uma escrita no lugar


## Gatilho de revisão
- Um verbo que escreve e precisa de campo de estratégia próprio, o que tornaria isto um carimbo no writer em vez de um campo por verbo
- Uma plataforma cujo caminho no lugar não possa ser nomeado, o que tornaria os três valores de estratégia insuficientes
