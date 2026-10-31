# ADR-0042: backup por padrão para comandos que alteram conteúdo

> Histórico (pré-0.1.35): os knobs de produto `ATOMWRITE_*` / env descritos abaixo foram superados. A configuração de runtime é apenas flags de CLI + `config.toml` XDG / `atomwrite set|get`.


- Status: Aceito
- Data: 2026-06-19
- Contexto: `AtomicWriteOptions::default()` tinha `backup: false`. O comando `write` (e outros 8 comandos que alteram conteúdo) NÃO criava backups por padrão. No incidente de 2026-06-15, um agente usou `atomwrite write` em vez de `atomwrite edit --before-match`, destruindo 122.994 bytes do `gaps.md` sem backup para recuperar. O ADR-0035 (v0.1.20) adicionou 6 camadas de defesa em profundidade (L1-L6), mas TODAS eram opt-in. O campo `keep_backup: false` significava que, mesmo quando `--backup` era passado explicitamente, o backup era apagado em silêncio após uma escrita bem-sucedida via `delete_backup_quietly`. O resultado: o caminho de execução padrão (sem flags) preservava ZERO cópias recuperáveis. A convenção do Unix (cp, mv, dd) não cria backups por padrão, mas essa convenção é inadequada para uma ferramenta cujo público principal são agentes LLM que cometem erros semânticos de comando (confundindo `write` com `edit`).

- Decisão: Mudar o padrão de `backup` de `false` para `true` em 9 structs de argumentos que alteram conteúdo: `WriteArgs`, `EditArgs`, `EditLoopArgs`, `ReplaceArgs`, `TransformArgs`, `ApplyArgs`, `SetArgs`, `DelArgs`, `CaseArgs`. Mudar `AtomicWriteOptions::default().backup` de `false` para `true`. Adicionar a flag `--no-backup` às 9 structs para opt-out explícito. Adicionar a variável de ambiente `ATOMWRITE_BACKUP=0` para opt-out global. Adicionar o helper `resolve_backup()` em `src/commands/mod.rs`, que implementa a cadeia de precedência: `--no-backup` (CLI) > `ATOMWRITE_BACKUP=0` (env) > padrão `true`. Manter `keep_backup: false` inalterado: o backup serve como rede de segurança temporária, não como armazenamento permanente. O backup é criado ANTES da escrita atômica e apagado automaticamente após o sucesso. 4 structs que não alteram conteúdo (`DeleteArgs`, `MoveArgs`, `CopyArgs`, `RollbackArgs`) mantêm `backup: false`, porque não sobrescrevem o conteúdo de arquivos via stdin.

- Consequências:
  - + Rede de segurança automática para TODAS as alterações de conteúdo: agentes que confundem `write` com `edit` podem recuperar via `rollback --latest`.
  - + ~1ms de overhead em SSD (`fs::copy` + `fs::remove_file` no caminho de sucesso). Overhead zero quando o alvo não existe (arquivo novo, sem necessidade de backup).
  - + `--no-backup` oferece opt-out explícito para pipelines críticos em desempenho.
  - + `ATOMWRITE_BACKUP=0` oferece opt-out global para ambientes automatizados que gerenciam a própria estratégia de backup.
  - + Alinha o atomwrite ao princípio "seguro por padrão": a proteção é automática, não opt-in.
  - - (aceitável) A flag `--backup` se torna redundante, já que o backup agora é o padrão. A flag é preservada por compatibilidade retroativa e explicitude.
  - - (aceitável) Testes existentes que testavam `--require-backup` sem `--backup` agora precisam de `--no-backup` para disparar a guarda, porque `--require-backup` verifica se `backup` é true, e o novo padrão o torna sempre true.

- Alternativas consideradas:
  1. **Manter o padrão `false` com documentação melhor.** Rejeitado: documentação não protege agentes LLM que cometem erros semânticos. O incidente de 2026-06-15 prova que a segurança opt-in é insuficiente: o agente tinha acesso à documentação e mesmo assim usou `write` em vez de `edit`.
  2. **Mudar apenas o `WriteArgs`.** Rejeitado: os 9 comandos que alteram conteúdo compartilham o mesmo perfil de risco. Um agente pode destruir dados via `edit --range 1:9999` ou `replace` com substituição vazia tão facilmente quanto via `write`. Cobertura parcial cria uma falsa sensação de segurança.
  3. **Mudar `keep_backup` para `true` (backups permanentes).** Rejeitado: backups permanentes acumulam uso de disco sem limite. O mecanismo `--retention N` já existe para quem quer backups persistentes. O backup temporário (criado e apagado no mesmo caminho de syscall) oferece recuperação de crash sem inchar o disco.

- Gatilho para revisitar: Se o overhead de ~1ms causar regressão mensurável em operações em lote (>10.000 arquivos), adicionar um modo `--fast` que desabilita o backup. Se a adoção de `ATOMWRITE_BACKUP=0` em scripts locais passar de 50% das invocações, reconsiderar se o padrão deveria ser sensível ao ambiente.
