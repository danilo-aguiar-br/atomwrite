# ADR-0080 — `sparse outline` conta o diário do atomwrite sob `--hidden`

## Status
- Aceita (2026-11-03)
- Emenda a ADR-0077, que levou este verbo à política comum de ocultos e deixou um filtro por nome atrás dele

## Contexto
- `sparse outline` caminha com `WalkerFlavor::RespectGlobalFlags`, então sem `--hidden` um dotfile já sai pela regra que todo verbo de varredura compartilha
- `src/commands/sparse_outline.inc.rs` também descartava todo nome iniciado por `WAL_JOURNAL_PREFIX`, e esse filtro rodava depois do walker sem consultar `global.hidden`
- Medido numa pasta com `a.rs`, `b.txt` e um `.atomwrite.journal.x.atomwrite.journal.json`: `sparse outline` respondeu `files_scanned:2` e `sparse outline --hidden` também respondeu 2
- A mesma pasta respondeu `count --hidden` com 3 arquivos e `sparse list` com 3, então dois verbos discordavam dos outros dois na mesma árvore

## Decisão
- O diário é excluído por ser um dotfile, e por nada além disso
- O filtro por nome é removido, então `--hidden` é o único interruptor que decide se o sidecar do próprio binário aparece na resposta
- Nenhuma condição `!global.hidden` é acrescentada, porque o walker já aplica essa regra e uma segunda cópia seria um segundo lugar para manter em sincronia
- Quem quer o diário removido roda o `wal-heal`, que já apaga journals terminais e é idempotente

## Consequências
- `sparse outline --hidden` responde 3 na pasta medida, e `sparse outline` continua respondendo 2
- Os verbos voltam a concordar na mesma árvore, então um agente que compara as contagens deixa de ver uma diferença silenciosa
- Um diário órfão custa tokens quando o chamador pede explicitamente as entradas ocultas, e esse custo é o pedido dele sendo honrado
- `files_unparsed` não muda, porque o diário tem extensão `.json` e resolve gramática

## Alternativas
- Manter o filtro e condicioná-lo a `!global.hidden`: rejeitada, porque o walker já é dono dessa regra e a duplicata sairia de sincronia
- Manter o filtro incondicional: rejeitada, porque ele faz o `--hidden` mentir, que é a falha que esta ADR fecha
- Excluir por nome tanto o diário quanto o sidecar de lock: rejeitada, porque o sidecar de lock é removido no release e uma lista de nomes cresce a cada sidecar que o binário ganha

## Gatilho de revisão
- Uma segunda categoria de sidecar que sobreviva a uma execução bem-sucedida e não seja oculta por nome
- Um verbo de varredura que precise esconder o diário mesmo sob `--hidden`, o que seria outro contrato e outra ADR
