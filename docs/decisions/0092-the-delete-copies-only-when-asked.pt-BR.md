# ADR-0092 — o delete só copia quando a chamada pede


## Status
- Aceita (2026-11-03)
- Registra o padrão que o código e o teste da 0.1.38 já fixaram
- Esclarece a ADR-0038 para o `delete` e para as operações do `batch`
- Emendada pela ADR-0093: a chave escrita agora alcança o `delete` e o `batch`


## Contexto
- O `delete` passava `silent_default = args.backup_opts.backup == Some(true)`, então sem `--backup` ele não criava cópia
- `delete_default_leaves_no_bak_file` e `delete_keep_backup_emits_warning_in_envelope` já fixavam esse contrato nos testes
- O help compartilhado continuava dizendo que a cópia vinha ligada por padrão, e os guias, o `llms-full` e a skill repetiam
- `--keep-backup` sozinho não criava cópia no `delete` standalone, mas o `delete` do `batch` contava a flag como pedido e criava uma
- `[defaults].backup` nunca chegava ao `delete`, porque o verbo passa o próprio default silencioso


## Decisão
- O `delete` cria cópia só quando a chamada pede, por `--backup` ou pela chave escrita `[defaults].backup = true` (ADR-0093); `--no-backup` é aceito e vence
- `--keep-backup` só preserva uma cópia que existe, e o aviso diz que `--backup` é a flag que cria uma
- O `delete` do `batch` segue a mesma regra: `--keep-backup` sozinho não cria cópia
- `[defaults].backup` não governava o `delete` nem as operações do `batch` nesta rodada; a ADR-0093 supera esta linha, e a chave ausente mantém o comportamento registrado aqui
- O help de `--backup`, o help de `--keep-backup`, o `about` do `delete`, o comentário no `delete.rs`, os guias e a skill carregam a regra real


## Consequências
- Um agente que lê `delete --help` aprende que nenhuma cópia nasce sem `--backup`
- Uma cópia de `delete`, depois de criada, nunca é removida sozinha, então `--keep-backup` mantém um sentido só entre o verbo standalone e o batch
- A chave `[defaults].backup` deixa de ser uma promessa silenciosa no `delete`, e a exceção fica escrita onde a chave é documentada
- O histórico do `docs/MIGRATION.md` mantém a linha escrita para a 0.1.28, e a correção chega como bullet novo


## Alternativas
- Religar a cópia por padrão: rejeitada, porque apagar é o pedido e uma cópia que o chamador não pediu mantém bytes no disco
- Ler `[defaults].backup` no `delete`: rejeitada nesta rodada e aceita na ADR-0093, que acrescenta o estado ausente em vez de mudar o significado da chave
- Manter o help compartilhado e documentar a exceção só nos guias: rejeitada, porque a primeira linha que o agente lê é o help


## Gatilho para revisitar
- Fechado pela ADR-0093: `backup` agora é `Option<bool>`, então uma chave ausente e um `true` escrito são estados diferentes
- Um pedido para `--keep-backup` criar a cópia no `delete`
