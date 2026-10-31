# ADR-0023: A recuperação do WAL do G114 é consultiva, não automática

- Status: Aceito
- Data: 2026-06-07
- Contexto: Quando um processo é morto entre `Started` e `Committed`, o sidecar do WAL fica com uma entrada `Started` pendente. A correção ingênua é reexecutar ou reverter a escrita automaticamente na próxima partida. Escolhemos expor `recover_orphan_journals(dir)` como função pública que devolve um relatório sem tocar no sistema de arquivos.
- Decisão: A recuperação é consultiva. `recover_orphan_journals(dir)` lê todos os arquivos `.atomwrite.journal.<basename>.atomwrite.journal.json` de um diretório e devolve `OrphanJournalReport { entries: Vec<JournalEntry>, target, op_id, started_at_unix, pid, age_secs }`. Quem chama (um hook local, uma pessoa desenvolvedora, um futuro CLI `atomwrite wal-recover`) decide o que fazer.
- Consequências:
  - + Risco zero de uma recuperação automática "prestativa" tomar uma decisão errada (por exemplo, reexecutar uma escrita que o usuário já corrigiu manualmente).
  - + Quem usa a biblioteca pode ligar o relatório à própria interface, log ou métricas.
  - + A função de recuperação é pura e fácil de testar.
  - - Ainda não existe um CLI `atomwrite wal-recover`, então os usuários precisam automatizar a recuperação por conta própria (adiado para a v0.1.13).
  - - Os journals órfãos se acumulam se ninguém executar a recuperação.
- Alternativas consideradas:
  1. Reexecutar automaticamente na próxima partida do atomwrite. Rejeitado: perigoso, o usuário pode ter corrigido o arquivo manualmente.
  2. Apagar automaticamente os journals órfãos com mais de 7 dias. Rejeitado: perda silenciosa de dados, o modo consultivo é mais seguro.
  3. Rejeitar todas as chamadas `atomic_write` se existir um órfão. Rejeitado: agressivo demais, há casos de uso legítimos em que o usuário quer ignorar os órfãos.
- Gatilho para revisitar: Quando o CLI `atomwrite wal-recover` for implementado (v0.1.13), poderemos adicionar uma flag `--cleanup` que apaga os órfãos com confirmação do usuário.
