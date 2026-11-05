# ADR-0079 — O lock no inode do alvo

## Status
Aceito (2026-11-03), substitui a exigência de sidecar do ADR-0031

## Contexto
- O lock exclusivo era tomado num arquivo sidecar `.<nome>.atomwrite.lock` criado ao lado do alvo.
- Criar esse nome exige permissão de escrita no diretório, e a escrita in-place nunca a exige: a permissão do próprio arquivo basta.
- Quem era dono do arquivo numa pasta sem escrita apanhava exit 13, e a mensagem nomeava a pasta enquanto o pedido nomeava o arquivo.
- A recusa parecia segurança, então ninguém procurava o defeito: o lock protegia um nome em vez do objeto que o verbo muta.
- A reentrância era chaveada pelo caminho do sidecar, então dois nomes que chegavam ao mesmo arquivo por links diferentes não compartilhavam lock nenhum.

## Decisão
### 1. O sujeito do lock é o inode
- `acquire_exclusive` abre o alvo em leitura e escrita e trava esse handle com `std::fs::File::try_lock`, que é `flock` no Unix e `LockFileEx` no Windows.
- O inode travado é o inode escrito, então a escrita in-place segura o lock no objeto exato que muta.
- Alvo que ainda não existe não tem inode a travar, e o sidecar cobre esse caso, que é a escrita de arquivo novo.
- Alvo que não abre para escrita cai no sidecar e deixa a escrita falhar depois nos próprios termos dela.

### 2. O sidecar vira compatibilidade, não correção
- O sidecar continua sendo tomado depois do inode, em melhor esforço, para a versão antiga do atomwrite, que só conhece o sidecar, seguir mutuamente excluída com esta.
- Sidecar que não pode ser criado deixa de abortar a execução que o lock do inode já cobre, e a falha vai para o log em nível debug.
- O sidecar fica para a janela de migração de duas versões e sai da prova de correção.
- Handle de sidecar que não responde mais ao próprio caminho é retomado, porque quem soltou o lock deslinkou o arquivo, e lock em inode inalcançável não exclui ninguém.
- A retomada é limitada, e diretório que este processo não pode escrever continua deixando a corrida só com o lock do inode.

### 3. A reentrância é chaveada pelo caminho do alvo
- A tabela por thread passa a usar o caminho resolvido do alvo em vez do caminho do sidecar, então o encaixe aninhado no mesmo objeto é grátis.
- Outra thread do mesmo processo pergunta ao sistema e espera, igual a outro processo.

### 4. A troca revalida o inode
- `LockGuard::target_unchanged` compara o handle travado com o caminho por `same_file::Handle`, que já é dependência.
- `atomic_write_based` chama isso imediatamente antes do despacho de estratégia, e caminho que agora responde a outro inode falha com `STATE_DRIFT` (exit 82) antes de escrever qualquer coisa.
- A recusa descarta o journal, porque nada foi trocado e não há o que recuperar.
- A checagem é pulada quando `expected_base` é `Some`, porque o verbo que leu o arquivo sob este mesmo lock já provou o conteúdo que vai substituir.
- `acquire_exclusive` compara o inode travado com o caminho por `target_is_same_inode` logo depois de o lock cair.
- Inode travado que não responde mais ao caminho é troca que aconteceu entre o open e o lock, então a aquisição recomeça por um open novo do caminho.
- O recomeço é limitado por `LOCK_STALE_INODE_MAX_RESTARTS`, e a última tentativa fica com o handle que tem em vez de girar sem fim.

### 5. Diretório sem escrita seleciona a estratégia in-place
- A estratégia era `Rename` para qualquer arquivo comum, e o rename precisa de um temporário ao lado do alvo.
- `parent_is_writable` pergunta `access(2)` por `W_OK`, e diretório sem escrita seleciona `InPlace`, que é a única estratégia que roda ali.
- O resultado é que arquivo do próprio usuário é escrito mesmo com a pasta sem escrita, e o código de saída deixa de ser 13 nesse caso.

### 6. Os verbos que leem antes de escrever passam a base
- `apply`, `set`, `del` e `edit` entregam o digest BLAKE3 do conteúdo que leram para `atomic_write_based`.
- Esse digest é a rede em filesystem onde o lock consultivo não existe.

## Consequências
- A escrita em pasta que o processo não pode escrever passa pelo caminho in-place, e o sidecar não é criado ali.
- Dois escritores no mesmo arquivo continuam serializados, e atualização perdida vira recusa em vez de silêncio.
- O Windows segue não validado: travar o handle do alvo pode barrar o rename por violação de compartilhamento, e isso precisa de prova em máquina real.
- NFS mantém a limitação antiga, porque `flock` é consultivo ali e pode ser ignorado; o digest da base é a rede.
- `ARCHITECTURE.md`, `docs/CROSS_PLATFORM.md`, `CHANGELOG.md` e o gêmeo `.pt-BR` registram o modelo novo e os dois limites acima.
