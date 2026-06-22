# ADR-0046: Retrofit resolve-first no diff

- **Status**: Aceito
- **Date**: 2026-06-21
- **Context**: O subcomando `diff` foi implementado antes do ADR-0027 (convenção resolve-first) ser estabelecida na v0.1.18. Quando o ADR-0027 foi adotado, os seguintes comandos foram retrofitados: write, edit, copy, apply, move, rollback, set, del, case, replace. O `diff` foi omitido dessa lista porque era considerado read-only e não mutante. Porém, a convenção resolve-first se aplica a TODOS os comandos que fazem I/O de arquivo — não apenas escritas — porque: (1) paths relativos devem resolver contra `--workspace` para consistência de API, (2) paths escapando o jail do workspace devem ser rejeitados com `WORKSPACE_JAIL` (exit 126) independente da intenção de leitura/escrita, e (3) agentes esperam semântica uniforme de paths em todos os subcomandos. O bug se manifestava como `FILE_NOT_FOUND` (exit 4) ao chamar `diff a.txt b.txt` com `--workspace /path/to/dir` mesmo quando ambos os arquivos existiam dentro do workspace.

- **Decision**: Adicionar `global.resolve_workspace()` + `path_safety::validate_path()` para ambos `file_a` e `file_b` no topo de `cmd_diff()`, antes de qualquer chamada a `read_file_string()`. O mesmo padrão de 3 linhas usado por todos os outros comandos desde a v0.1.18.

- **Consequences**:
  - **+** `diff` agora resolve paths relativos contra `--workspace`, consistente com todos os outros subcomandos.
  - **+** Paths escapando o jail do workspace são rejeitados com `WORKSPACE_JAIL` (exit 126) em vez de `FILE_NOT_FOUND` confuso.
  - **+** Agentes não precisam mais de lógica especial de prefixação de path para `diff`.
  - **+** Conformidade total com ADR-0027 e ADR-0030.
  - **-** (nenhuma) A mudança é puramente aditiva e retrocompatível: paths absolutos dentro do workspace continuam funcionando.

- **Alternatives considered**:
  1. **Deixar diff como está e documentar a exceção.** Rejeitado: inconsistência é fonte recorrente de erros de agentes e o fix é trivial (3 linhas).
  2. **Apenas resolver mas pular validação de jail para comandos read-only.** Rejeitado: validação de jail previne ataques de escape de path via symlink mesmo em modo read-only.

- **Trigger to revisit**: Se um novo subcomando read-only for adicionado, aplicar o mesmo padrão resolve-first desde o primeiro commit (lição aprendida deste gap).
