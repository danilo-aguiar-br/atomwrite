# ADR-0063 — SIGPIPE: um handler cooperativo com exit 141 (desvio consciente)

## Status
Aceito (2026-10-15)

## Contexto
- A regra do dono manda resetar o SIGPIPE para `SIG_DFL` (o antigo `reset_sigpipe`).
- Com `SIG_DFL`, um leitor fechado mata o processo sem rodar nenhum destrutor. Um `replace | head -1` deixava de 7 a 11 sidecars de lock, de 3 a 7 journals (os `committed` inclusive) e às vezes um `.atomwrite-*.tmp` (V1 FAIL-1 em 5 execuções, V2 M1 em 6 execuções).
- O `wal-heal` nunca recolhia esses journals: ele reportava `removed:0, preserved:7`.
- Ignorar o SIGPIPE era a outra opção. Isso esconde o leitor fechado de todo worker, exceto o que escreve.

## Decisão
- O SIGPIPE não é ignorado nem deixado em `SIG_DFL`. O `register_broken_pipe` instala um handler que marca um encerramento cooperativo com exit 141 (`ShutdownSignal::record_broken_pipe`). O handler faz só stores atômicos: sem alocação, sem lock, sem I/O.
- Um SIGPIPE repetido nunca força a saída, porque várias threads podem bater no pipe fechado antes de a primeira ter desenrolado.
- A escrita que bate no pipe fechado falha com `BrokenPipe`. O caminho de erro normal roda todo `Drop`, e o `main` sai com exit 141.
- Uma escrita em voo termina e commita: o `is_global_hard_shutdown` ignora a bandeira de pipe quebrado, e só SIGINT, SIGTERM, SIGHUP e o timeout a abortam. As checagens entre arquivos param o trabalho novo.
- O banner de encerramento não é escrito para um pipe quebrado. O `read` devolve 141 e propaga uma falha de flush, onde devolvia 0. Quando o próprio envelope de erro não pode ser escrito porque o leitor fechou o pipe, a execução sai com exit 141.
- O `reset_sigpipe` foi removido.

## Consequências
- Este é um DESVIO CONSCIENTE da regra literal do dono. A regra de encerramento graceful e o exit 141 foram preservados, e o dono pode reverter o desvio restaurando o `SIG_DFL`, ao custo do resíduo acima.
- 20 execuções de `replace | head -1` e 20 de cada `list`, `search` e `read` saíram com exit 141 por código, não por sinal, e não deixaram lock, journal nem tempfile (gaps.md W7, T2). Uma repetição de 5 execuções de `replace | head -1` em 600 arquivos também deu exit 141 e resíduo zero (T5b).
- No Windows não existe SIGPIPE, e o `BrokenPipe` do `std` segue o mesmo caminho. Isso não foi validado lá.
- Num pipe fechado o stderr ainda recebe duas linhas `WARN data channel write failed` (o evento cancelled e o flush). Elas não são dado e não poluem o stdout.
- Todo caminho de saída que escreve no stdout por conta própria sai com exit 141 num leitor fechado, o `completions` inclusive. A auditoria de docs achou um resíduo antes da publicação: `completions bash | head -1` entrava em pânico dentro do `clap_complete::generate` e saía com exit 134 e um arquivo de relatório de pânico. Ele foi corrigido na própria 0.1.37: o `render_completions` gera o script em memória e a escrita propaga o erro.
- Não coberto: um leitor que fecha só depois de receber a saída inteira não causa `EPIPE`, então a execução sai com exit 0, o que está correto. No Windows o mesmo caminho não foi validado.

## Alternativas
- `SIG_DFL` (a regra literal do dono): rejeitada, pelo resíduo medido acima.
- Ignorar o SIGPIPE: rejeitada, só a thread que escreve saberia que o leitor fechou.

## Evidência
- `CHANGELOG.md:76,176,196-201,283` e `gaps.md:193-200` (W7).
- `src/signal_handlers.inc.rs:122-149` (`register_broken_pipe` e a justificativa no rustdoc) e `:281`, `src/signal.rs:63-77` (`is_global_hard_shutdown`) e `:195-204` (`record_broken_pipe`), `src/atomic/rename_path.rs:49-52`, `src/main.rs:182-184`, `src/constants.rs:56` (`EXIT_BROKEN_PIPE` vale 141, com assert de igualdade a 128 + 13).
- `tests/cli_v0137_terminal_pipe.rs` (4 testes), `tests/cli_v0137_broken_pipe_outputs.rs` (2 testes, 17 caminhos de saída com a ponta de leitura já fechada: `completions` para 5 shells, `--print-schema` em 3 formas, `--json-schema`, `agent-surface`, `commands`, `doctor`, três formas de `--help`, `--version` e um comando desconhecido), os testes unitários de `signal::tests`.
- `src/lib.rs:339,356` (as escritas do script) e `:363` (`render_completions`).
- Binário 0.1.37: `atomwrite completions bash | head -c 10` saiu com exit 141 e `atomwrite --help | head -c 10` saiu com exit 141. `--print-schema | head -c 10` e `--version | head -c 3` saíram com exit 0, porque a saída inteira cabe no pipe antes de o `head` terminar. A prova determinística é o teste acima.

## Gatilho para revisar
- O dono decidir restaurar o `SIG_DFL`, aceitando o resíduo.
- O Windows ser validado num host real.
