# 0042-backup-by-default (PT-BR) — Tradução

_Tradução automática do ADR original em inglês. Em caso de divergência, a versão EN prevalece._

# ADR-0042: backup-by-default para comandos que mutam conteúdo

- **Status**: Aceito
- **Data**: 2026-06-19
- **Contexto**: `AtomicWriteOptions::default()` tinha `backup: false`. O comando `write` (e outros 8 comandos que mutam conteúdo) NÃO criavam backups por padrão. No incidente de 2026-06-15, um agente usou `atomwrite write` em vez de `atomwrite edit --before-match`, destruindo 122.994 bytes de `gaps.md` sem backup para recuperação. O ADR-0035 (v0.1.20) adicionou 6 camadas de defesa em profundidade (L1-L6) mas TODAS eram opt-in. O campo `keep_backup: false` significava que mesmo quando `--backup` era passado explicitamente, o backup era deletado silenciosamente após um write bem-sucedido via `delete_backup_quietly`. O resultado: o caminho de execução padrão (sem flags) preservava ZERO cópias recuperáveis. A convenção Unix (cp, mv, dd) não cria backups por padrão, mas essa convenção é inadequada para uma ferramenta cujo público primário são agentes LLM que cometem erros semânticos de comando (confundindo `write` com `edit`).

- **Decisão**: Alterar o default de `backup` de `false` para `true` em 9 structs de argumentos que mutam conteúdo: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`. Alterar `AtomicWriteOptions::default().backup` de `false` para `true`. Adicionar flag `--no-backup` a todas as 9 structs para opt-out explícito. Adicionar variável de ambiente `ATOMWRITE_BACKUP=0` para opt-out global. Adicionar helper `resolve_backup()` em `src/commands/mod.rs` que implementa a cadeia de precedência: `--no-backup` (CLI) > `ATOMWRITE_BACKUP=0` (env) > default `true`. Manter `keep_backup: false` inalterado — o backup serve como rede de segurança temporária, NÃO como armazenamento permanente. O backup é criado ANTES do write atômico e auto-deletado após sucesso. 4 structs não-conteúdo (`DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`) mantêm `backup: false` porque não sobrescrevem conteúdo de arquivo via stdin.

- **Consequências**:
  - **+** Rede de segurança automática para TODAS as mutações de conteúdo — agentes que confundem `write` com `edit` podem recuperar via `rollback --latest`.
  - **+** ~1ms de overhead em SSD (`fs::copy` + `fs::remove_file` no caminho de sucesso). Zero overhead quando o alvo não existe (arquivo novo, backup desnecessário).
  - **+** `--no-backup` fornece opt-out explícito para pipelines com prioridade em performance.
  - **+** `ATOMWRITE_BACKUP=0` fornece opt-out global para ambientes de CI que gerenciam sua própria estratégia de backup.
  - **+** Alinha o atomwrite com o princípio "seguro por padrão" — proteção é automática, não opt-in.
  - **-** (aceitável) A flag `--backup` se torna redundante já que backup agora é o padrão. A flag é preservada para compatibilidade retroativa e explicitude.
  - **-** (aceitável) Testes existentes que testavam `--require-backup` sem `--backup` agora precisam de `--no-backup` para disparar a guarda, porque `--require-backup` verifica se `backup` é true, e o novo default o torna sempre true.

- **Alternativas consideradas**:
  1. **Manter default `false` com melhor documentação.** Rejeitado: documentação não protege agentes LLM que cometem erros semânticos. O incidente de 2026-06-15 prova que segurança opt-in é insuficiente — o agente tinha acesso à documentação e ainda assim usou `write` em vez de `edit`.
  2. **Alterar apenas `WriteArgs`.** Rejeitado: todos os 9 comandos que mutam conteúdo compartilham o mesmo perfil de risco. Um agente pode destruir dados via `edit --range 1:9999` ou `replace` com substituição vazia tão facilmente quanto via `write`. Cobertura parcial cria falsa sensação de segurança.
  3. **Alterar `keep_backup` para `true` (backups permanentes).** Rejeitado: backups permanentes acumulam uso de disco sem limite. O mecanismo `--retention N` já existe para usuários que desejam backups persistentes. O backup temporário (criado e deletado no mesmo caminho de syscall) fornece recuperação de crash sem acúmulo de disco.

- **Gatilho para revisitar**: Se o overhead de ~1ms causar regressão mensurável em operações batch (>10.000 arquivos), adicionar um modo `--fast` que desabilita backup. Se a adoção de `ATOMWRITE_BACKUP=0` em CI exceder 50% das invocações, reconsiderar se o default deveria ser sensível ao ambiente.


---

_Original em inglês: [`0042-backup-by-default.md`](0042-backup-by-default.md)_
