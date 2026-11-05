# ADR-0085 — a família do edit publica o veredito de escrita


## Status
- Aceita (2026-11-03)
- Estende a ADR-0084, que fez os verbos de cópia e de remoção nomearem o mecanismo, para os verbos que reescrevem conteúdo
- Estendida pela ADR-0086, que leva o mesmo veredito ao `backup_path` e faz o rollback do `apply` publicar o que restaurou


## Contexto
- `atomic_write_based` devolve o mesmo `WriteResult` que `atomic_write` devolve, e os nove verbos desta rodada o chamavam e consumiam só o braço de erro ou o checksum
- `edit`, `replace`, `transform`, `scope`, `case`, `set`, `del`, `edit-loop` e `locale` publicam um evento por arquivo, e todo evento ficava calado sobre o caminho que a escrita tomou
- `scope` e `transform` têm ramo somente leitura que constrói o mesmo struct sem escrever nada, e `case`, `transform` e `scope` também respondem prévia, então um campo obrigatório sem valor de ausência deixaria uma prévia reivindicar escrita que nunca fez
- O `del` retorna cedo quando a chave nunca existiu, e o irmão `config unset` já omite o campo nesse caso
- O `batch delete` copiava a cópia de uma cópia, porque a regra de não copiar cópia morava em `src/commands/delete.rs` em vez de um módulo que os dois verbos dividem


## Decisão
- `edit`, `replace`, `transform`, `scope`, `case`, `set`, `del`, `edit-loop` e `locale` publicam `write_strategy` e `backup_skipped`, então o consumidor lê o mecanismo no evento que o relata
- `write_strategy` é obrigatório em todo evento que relata escrita, e `backup_skipped` fica opcional e é omitido no JSON quando ausente
- A prévia e o ramo somente leitura publicam `"none"`, pela mesma regra que a ADR-0084 fixou para o `batch --dry-run`, porque a execução não tomou caminho de escrita
- O `del` de chave ausente omite `write_strategy` em vez de publicar `none`, porque o irmão `config unset` omite e veredito para chave que nunca existiu seria falso
- O `locale --clear` publica `"none"`, porque remove o arquivo de preferência e não grava conteúdo, e o relatório simples do `locale` omite os dois campos porque não escreve nada
- O `edit-loop` publica o veredito do único arquivo que reescreve, porque escreve uma vez por invocação e não uma vez por par
- O `batch delete` recusa copiar uma cópia, pelo predicado compartilhado `backup_names::is_backup_path`, e os dois verbos leem a sentença `BACKUP_OF_BACKUP_SKIPPED` desse módulo para o texto não divergir
- O alvo nomeado como cópia de backup continua sendo removido, porque a recusa é sobre a cópia e não sobre a remoção


## Consequências
- Os nove verbos respondem a mesma forma que os sete verbos da ADR-0084 respondem, então um registro de um é comparado com o de outro
- A prévia e a escrita deixam de parecer iguais para o consumidor, porque a prévia publica `none`
- Um consumidor que falha com campo desconhecido precisa permitir os dois nomes, e o guia de migração diz isso
- Os nove schemas são regravados do binário por `cargo xtask schemas`, então o arquivo e o evento não podem divergir
- O `delete` e o `batch delete` dividem uma sentença só, então a mesma árvore deixa de responder diferente conforme o verbo


## Alternativas
- Extrair um `WriteVerdict` único com `#[serde(flatten)]`: rejeitada, porque o livro do serde mede que `flatten` exige struct com campos nomeados ou mapa e não convive com `deny_unknown_fields`, e a economia seria de duas linhas por struct
- Publicar `none` no `del` de chave ausente: rejeitada, porque o irmão `config unset` omite e dois irmãos carregariam significados diferentes para o mesmo campo
- Exigir `write_strategy` no relatório simples do `locale`: rejeitada, porque esse relatório não escreve nada e `none` leria como se tivesse escrito
- Fazer `backup`, `prune-backups` e `wal-heal` publicarem o veredito: rejeitada nesta rodada, porque eles nunca chamam o escritor atômico e não têm `WriteResult` para descartar


## Gatilho para revisitar
- Um verbo que reescreve conteúdo por caminho que o `WriteResult` não descreve, o que exigiria um quarto valor de estratégia
- Uma prévia que passe a escrever, o que tornaria `none` errado
- Uma operação de `batch` que copie a cópia de uma cópia sob condição que o predicado compartilhado não cobre
