# ADR-0020: Caminho e formato do arquivo sidecar do WAL

> Status: Superado na v0.1.37 quanto à vida do sidecar. O journal que foi gravado é removido quando a escrita termina (medido: `write --wal-policy always` não deixa arquivo de journal).

- Status: Aceito
- Data: 2026-06-07
- Contexto: O G114 (Auditoria de Recuperação de Crash) precisa de um write-ahead log para que um processo morto no meio de um `atomic_write` deixe rastros recuperáveis. Já usamos `.<target>.atomwrite.lock` para o `flock(2)` consultivo. Era preciso uma convenção de sidecar separada, que não colida com o sidecar do lock.
- Decisão: O sidecar do WAL é `.atomwrite.journal.<basename>.atomwrite.journal.json`. Cada `atomic_write` abre/cria esse arquivo e acrescenta uma linha JSONL por fase. O arquivo fica no mesmo diretório do alvo. Em caso de sucesso, a linha final `Committed` é escrita e o arquivo NÃO é apagado (barato, humanos podem usar `rm` e as próximas operações sobrescrevem o mesmo caminho). Se o processo morrer entre `Started` e `Committed`, o sidecar vira um "journal órfão".
- Consequências:
  - + O caminho é determinístico e único por alvo, então as ferramentas de recuperação varrem um diretório e acham os órfãos por glob.
  - + O formato JSONL (um objeto por linha) é fácil de processar com `jaq` e `rg`, sem precisar de uma biblioteca JSON completa na hora da recuperação.
  - + A recuperação (`recover_orphan_journals(dir)`) é consultiva: lê e relata os órfãos, mas nunca altera o sistema de arquivos. Quem chama decide se reexecuta, aborta ou limpa.
  - - Não limpamos automaticamente os journals confirmados, então um journal sem uso pode se acumular. Um futuro subcomando `atomwrite wal-gc` pode limpar entradas com mais de 7 dias (adiado para a v0.1.13).
  - - Um processo patológico que cai 1000 vezes seguidas deixa 1000 linhas no sidecar. Limitamos a 1000 linhas por arquivo (adiado para a v0.1.13).
- Alternativas consideradas:
  1. Um único `/var/log/atomwrite.wal` global. Rejeitado: não é portável, tem problemas de permissão e não é por alvo.
  2. WAL do SQLite. Rejeitado: exige a dependência de sistema `libsqlite3-sys` e complica a compilação cruzada.
  3. Embutir em xattr. Rejeitado: nem todo sistema de arquivos suporta xattr, e FAT32 e /tmp em overlayfs retornam `EOPNOTSUPP`.
- Gatilho para revisitar: Se o tamanho do sidecar do WAL virar problema em scripts locais, adicionar `--wal-rotate-size N` para rotacionar o sidecar.
