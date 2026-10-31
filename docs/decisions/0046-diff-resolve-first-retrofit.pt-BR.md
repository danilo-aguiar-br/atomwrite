# ADR-0046: adaptação do diff à convenção de resolver primeiro

- Status: Aceito
- Data: 2026-06-21
- Contexto: O subcomando `diff` foi implementado antes de o ADR-0027 (convenção de resolver primeiro) ser estabelecido na v0.1.18. Quando o ADR-0027 foi adotado, os seguintes comandos foram adaptados: write, edit, copy, apply, move, rollback, set, del, case, replace. O comando `diff` ficou fora dessa lista porque foi considerado somente leitura e não mutador. No entanto, a convenção de resolver primeiro se aplica a TODOS os comandos que fazem I/O de arquivos, não apenas escritas, porque: (1) caminhos relativos precisam ser resolvidos contra `--workspace` por consistência da API, (2) caminhos que escapam do jail do workspace precisam ser rejeitados com `WORKSPACE_JAIL` (exit 126) independentemente da intenção de leitura ou escrita, e (3) os agentes esperam semântica de caminho uniforme em todos os subcomandos. O bug se manifestava como `FILE_NOT_FOUND` (exit 4) ao chamar `diff a.txt b.txt` com `--workspace /path/to/dir`, mesmo com os dois arquivos existindo dentro do workspace.

- Decisão: Adicionar `global.resolve_workspace()` + `path_safety::validate_path()` para `file_a` e `file_b` no topo do `cmd_diff()`, antes de qualquer chamada a `read_file_string()`. É o mesmo padrão de 3 linhas usado por todos os outros comandos desde a v0.1.18.

- Consequências:
  - + O `diff` agora resolve caminhos relativos contra `--workspace`, de forma consistente com todos os outros subcomandos.
  - + Caminhos que escapam do jail do workspace são rejeitados com `WORKSPACE_JAIL` (exit 126) em vez de um confuso `FILE_NOT_FOUND`.
  - + Os agentes não precisam mais de lógica especial de prefixo de caminho para o `diff`.
  - + Conformidade total com o ADR-0027 e o ADR-0030.
  - - (nenhuma) A mudança é puramente aditiva e retrocompatível: caminhos absolutos dentro do workspace continuam funcionando.

- Alternativas consideradas:
  1. Deixar o diff como está e documentar a exceção. Rejeitado: a inconsistência é uma fonte recorrente de erros dos agentes, e a correção é trivial (3 linhas).
  2. Apenas resolver, mas pular a validação do jail para comandos somente leitura. Rejeitado: a validação do jail previne ataques de escape de caminho baseados em symlink mesmo no modo somente leitura.

- Gatilho para revisitar: Se um novo subcomando somente leitura for adicionado, aplicar o mesmo padrão de resolver primeiro desde o primeiro commit (lição aprendida com este gap).
