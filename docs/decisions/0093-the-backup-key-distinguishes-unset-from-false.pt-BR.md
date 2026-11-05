# ADR-0093 — a chave de backup distingue ausente de falso


## Status
- Aceita (2026-11-03)
- Emenda a ADR-0092 e fecha o gatilho que ela registrou


## Contexto
- `DefaultsSection.backup` era um `bool` com padrão `true`, então uma chave ausente e um `false` escrito carregavam o mesmo valor
- O `delete` passava o próprio padrão silencioso e nunca lia a chave, e o `batch` calculava `backup_explicit` só a partir da flag
- A ADR-0092 registrou a exceção e nomeou a condição para revê-la: uma mudança de config que deixasse a chave governar o `delete` sem quebrar o contrato publicado


## Decisão
- `DefaultsSection.backup` é `Option<bool>`, e `None` significa que a chave não está escrita
- A chave ausente mantém o padrão de cada verbo: `resolve_backup` passa `true` para as reescritas, e o `delete` passa `false`
- Um `true` escrito alcança o `delete` e o `batch`, e um `false` escrito desliga a cópia das reescritas
- `--no-backup` continua vencendo a chave, e `--backup` continua forçando a cópia
- `--keep-backup` continua só preservando uma cópia que existe, e o aviso aparece só quando nenhuma cópia foi criada
- `config get` mantém `origin: default` e omite `value` enquanto a chave não está escrita, então a forma publicada não ganha `null`


## Consequências
- Quem escreve `backup = true` ganha cópia no `delete`, e quem nunca escreve a chave mantém o delete sem cópia
- `config unset defaults.backup` volta aos padrões dos verbos
- O `delete` do `batch`, a reescrita do `batch`, o `move` e o `copy` veem a chave escrita do mesmo jeito que veem o `--backup` global
- O schema da chave continua `boolean`, porque o TOML ainda aceita só `true` e `false`
- O aviso do `--keep-backup` é calculado por arquivo, então um alvo que já é backup continua recebendo o conselho mesmo quando a chave pediu a cópia


## Alternativas
- Manter `bool` e detectar presença com uma segunda leitura do TOML: rejeitada, porque duplica a fonte de verdade e `Option` é o idioma
- Fazer o `delete` copiar por padrão: rejeitada, porque apagar é o pedido e a cópia precisa continuar opt-in
- Criar uma segunda chave para o delete: rejeitada, porque a chave existente já nomeia o significado


## Gatilho para revisitar
- Um pedido para o `--keep-backup` criar a cópia no `delete`
- Um formato de config que precise de um terceiro estado além de ausente, verdadeiro e falso
