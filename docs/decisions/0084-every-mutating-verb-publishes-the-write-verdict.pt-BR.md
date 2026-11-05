# ADR-0084 — todo verbo que muta publica o veredito de escrita


## Status
- Aceita (2026-11-03)
- Estende a ADR-0081, que fez `write` e `apply` nomearem a estratégia, para o resto da superfície
- Estendida pela ADR-0085, que trouxe os nove verbos que reescrevem conteúdo para a mesma forma


## Contexto
- `atomic_write` devolve um `WriteResult` com `write_strategy` e `backup_skipped`, e `src/atomic/write.rs` é o único lugar que sabe qual caminho a escrita tomou de verdade
- `copy`, `move`, `delete`, `rollback`, `semantic-merge`, `config set`, `config unset` e as operações do `batch` o chamavam e descartavam o resultado, então o evento relatava `bytes` e `checksum` enquanto o mecanismo ficava invisível
- Um chamador só deduzia o mecanismo pelo nome da chamada de sistema ou pelo inode, e nenhum dos dois viaja no JSON
- O `batch` roda as operações num closure paralelo, então campo emprestado amarraria o worker ao tempo de vida do chamador
- O `config` responde seis ações, e quatro delas não leem nem escrevem, então campo obrigatório faria uma leitura reivindicar mecanismo que nunca usou


## Decisão
- `copy`, `move`, `delete`, `rollback` e `semantic-merge` publicam `write_strategy` como campo obrigatório, porque cada um escreve ou desfaz o vínculo do alvo que relata
- `delete` publica o valor `none`, porque remove a entrada de diretório e não toma caminho de escrita atômica
- `move` publica a estratégia da chamada que tomou: `rename` no mesmo sistema de arquivos, e o valor do `WriteResult` na queda entre dispositivos, que `cross_device` e `atomic` já separam
- `config set` e `config unset` publicam o campo só quando escreveram, e a loja `locale` publica `rename` para o arquivo de preferência e `none` para a remoção dele
- `batch` publica o veredito por operação, com `none` para `hash` e `delete` e o valor do `WriteResult` para `write`, `replace`, `edit` e `copy`
- Uma operação de `batch --dry-run` também publica `none`, porque o evento relata um plano e a execução não usou caminho de escrita nenhum
- O campo do `batch` é `&'static str`, então o closure paralelo continua `Send` e o valor continua o conjunto fechado que o motor produz
- `backup_skipped` é opcional em todo lugar e é omitido no JSON quando ausente, então nenhum evento carrega `null` que o leitor não distinga de cópia pulada


## Consequências
- O consumidor lê o mecanismo no evento que relata a escrita, e o inode e a chamada de sistema deixam de ser a única evidência
- Os sete verbos respondem a mesma forma, então um registro de um é comparado com o de outro
- Análises que validam o evento contra schema fechado precisam permitir os dois nomes, e o guia de migração diz isso
- `write_strategy` numa ação de leitura do `config` fica ausente em vez de reivindicar `none`, o que mantém leitura e remoção distinguíveis
- Os oito schemas publicados são regravados do binário, então arquivo e evento não divergem


## Alternativas
- Exigir `write_strategy` no `config` em toda ação: rejeitada, porque `path`, `list`, `get` e `doctor` não escrevem e `none` soaria como se tivessem escrito
- Publicar texto livre montado por verbo: rejeitada, porque o valor é conjunto fechado e campo de texto convida variante que o motor não produz
- Deixar o `batch` sem o campo e envolver o resultado da operação: rejeitada, porque relataria o veredito um nível longe da operação que o produziu


## Gatilho para revisitar
- Um verbo que tome caminho de escrita que o `WriteResult` não descreve, o que exigiria um quarto valor de estratégia
- Uma operação do `batch` que escreva de modo que o enum de desfecho não nomeie, o que tornaria `none` errado
