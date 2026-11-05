# ADR-0094 — o delete recursivo é dono da árvore nomeada inteira


## Status
- Aceita em 2026-11-03
- Emenda a ADR-0083, que é dona da política compartilhada de caminhada, e a ADR-0092 e a ADR-0093, que são donas da cópia do delete


## Contexto
- O `delete --recursive` montava o walker por `build_scoped_walker`, que é `WalkerFlavor::RespectGlobalFlags`, então a caminhada escondia toda entrada com ponto e todo caminho listado por arquivo de ignore
- A passagem de remoção de diretório usava um walker diferente, engolia todo erro de `remove_dir` e o verbo publicava um `InvalidInput` sintético com exit 65 depois de já ter apagado os arquivos visíveis
- O chamador que recebeu exit 65 encontrou o diretório ainda no disco com `.git`, `kept.log` e entradas ocultas dentro, e teve de apagá-lo com `rm -rf`
- O helper compartilhado tocava só `hidden`, `git_ignore` e `follow_links`, então `.ignore`, o gitignore global e o `info/exclude` continuavam escondendo entradas mesmo com `--no-gitignore`
- A coleta aceitava só arquivo e link, então um fifo, socket ou dispositivo dentro da árvore sobrevivia e mantinha o pai no disco
- O hash e o backup corriam por caminho, então um fifo podia travar o open e um link podia copiar o conteúdo do alvo
- Uma cópia de backup escrita ao lado de um arquivo dentro da árvore sobrevivia à remoção e mantinha a raiz no disco


## Decisão
- O `delete --recursive <dir>` sem filtro de estreitamento é dono da árvore nomeada inteira, e remove entrada oculta, ignorada, link, fifo, socket e dispositivo
- O `WalkerFlavor::DeleteTree` desliga toda fonte de ignore, o filtro de ocultos, a busca de ignore no pai e o seguimento de link, e fica em um único sistema de arquivos
- A travessia nunca segue link, inclusive quando o link é nomeado como raiz, que é desvinculado como entrada e nunca como alvo
- Fifo, socket e dispositivo são desvinculados sem serem abertos e não publicam `checksum_before`, porque não há conteúdo para somar
- O backup pedido para link ou para entrada especial é recusado por evento com `backup_skipped` nomeando o motivo
- O backup pedido com raiz de diretório é recusado antes do primeiro unlink, porque a cópia nasceria dentro da árvore e sobreviveria à remoção
- A limpeza de diretório usa o mesmo walker `DeleteTree` e remove diretório vazio do mais fundo para o mais raso, tolerando `NotFound` e registrando todo outro erro com o caminho
- O run sem estreitamento publica falha de I/O real com o caminho sobrevivente em `errors[]` quando um diretório sobrevive, e nunca o exit 65 sintético
- O run estreitado por `--include`, `--exclude`, `--older-than` ou `--limit` sai 0 e publica `dirs_left`, `entries_left` e um hint que nomeia os filtros
- `--plan` e `--dry-run` não publicam campo de sobra, porque nunca removeram nada
- Raízes sobrepostas colapsam na raiz externa e raiz repetida é deduplicada
- Entrada que some entre a coleta e a remoção conta como pulada, então um run repetido chega ao mesmo estado final
- `--hidden`, `--no-gitignore` e `--follow-symlinks` são aceitos como no-op no `delete` e emitem uma linha de diagnóstico
- O `copy -r` mantém a promessa e deixa de ler `.ignore`, o gitignore global e o `info/exclude`


## Consequências
- O consumidor que tratava exit 65 de run filtrado como falha agora recebe exit 0 com `dirs_left` e `entries_left`
- O consumidor que esperava `INVALID_INPUT` quando um diretório sobrevivia sem filtro agora recebe o código de I/O medido, como 74 ou 13, com o caminho sobrevivente em `errors[]`
- O chamador que contava com `--follow-symlinks` para apagar o alvo por link agora remove o link sozinho
- Apagar `.git` dentro da raiz nomeada passa a ser possível, que é o efeito direto de ser dono da árvore
- Ponto de montagem dentro da árvore não é atravessado e vira falha nomeada, o que difere do `rm -rf`
- Fifo, socket ou dispositivo nomeado como raiz continua recusado por `validate_path` com exit 85, porque a raiz não é entrada dentro de árvore
- A árvore é coletada antes da remoção, então a memória cresce com o número de entradas
- Windows e macOS não foram executados nesta decisão e ficam declarados como não validados


## Alternativas
- Criar `--force` ou uma segunda flag para incluir entradas ocultas: rejeitada, porque um delete que esconde parte da árvore nomeada responde outra pergunta, e flag que conserta o verbo esconde o defeito
- Manter `remove_dir_all` na remoção final: rejeitada, porque destruiria o que `--exclude` deixou de fora sem contador nenhum ver
- Manter o exit 65 sintético para diretório sobrevivente: rejeitada, porque nomeia erro de argumento depois de remoção parcial e esconde o erro de I/O medido
- Criar o backup fora da árvore: rejeitada por ora, porque nenhum verbo aceita destino de backup e a cópia precisaria de contrato novo
- Seguir link para remover o alvo: rejeitada, porque a árvore nomeada é dona das entradas dentro dela e nunca do que as entradas apontam


## Gatilho para revisitar
- Um pedido de destino de backup, que tornaria `--backup` utilizável com raiz de diretório
- Uma plataforma onde `same_file_system` não é suportado, que precisaria de outra guarda de montagem
