# ADR-0044: --old-file/--new-file para o comando edit

- Status: Aceito
- Data: 2026-06-19
- Contexto: `edit --old`/`--new` aceitam conteúdo apenas por argumentos da CLI. A expansão de shell `$(cat file)` injeta o conteúdo no argv antes do `execve(2)`. O ARG_MAX do kernel Linux é de 2.097.152 bytes para argv+envp combinados, e o limite efetivo para um único argumento é de ~131 KB. Conteúdo acima desse limite causa E2BIG (errno 7): o processo atomwrite nunca inicia e o agente recebe exit 126 do shell, sem envelope JSON. O contorno `edit --multi` com NDJSON no stdin existe, mas exige uma codificação JSON propensa a erros. Os agentes usam `$(cat)` por simplicidade e esbarram no E2BIG em silêncio com arquivos grandes. A documentação não menciona o ARG_MAX nem sugere `--multi` como alternativa.

- Decisão: Adicionar `--old-file <PATH>` e `--new-file <PATH>` ao `EditArgs` como alternativas a `--old` e `--new`. Os arquivos são lidos dentro do processo atomwrite, contornando a expansão de shell e o ARG_MAX. Usar `conflicts_with` do clap para impedir a mistura de `--old` com `--old-file` (prevenção de Silent Argument Discard conforme rules-rust-cli-stdin-stdout-silent-discard). Validar os caminhos contra o jail do workspace. Adicionar o campo `source: "arg"|"file"` ao `PairResult` para rastreabilidade. Adicionar validação em runtime no `resolve_edit_pairs()` para rejeitar a mistura cruzada de `--old` com `--new-file` ou de `--old-file` com `--new`, com exit 65 (`INVALID_INPUT`) e a mensagem 'cannot mix --old with --new-file or --old-file with --new; use both from the same source'. Adicionar `strip_file_trailing_newline()` para remover exatamente uma quebra de linha final (`\n` ou `\r\n`) do conteúdo do arquivo, por paridade com o comportamento do argv: arquivos criados com `echo` incluem uma quebra de linha final que valores de argv nunca têm.

- Consequências:
  - + Elimina o limite do ARG_MAX para os agentes: conteúdo de qualquer tamanho pode ser usado.
  - + Zero problemas de expansão de shell ($, crases, aspas etc.).
  - + `conflicts_with` impede o descarte silencioso de `--old` quando `--old-file` está presente.
  - + `PairResult.source` oferece rastreabilidade da origem do conteúdo.
  - + A guarda de mistura cruzada pega o caso que o `conflicts_with` do clap não consegue: `--old` pareado com `--new-file` (ou vice-versa).
  - + A remoção da quebra de linha final garante que `--old-file old.txt` case com o mesmo conteúdo de `--old "text"`, mesmo quando o `old.txt` foi criado por `echo "text" > old.txt`.
  - - (aceitável) Os agentes precisam gravar arquivos temporários antes do edit (fluxo em duas etapas).
  - - (aceitável) Não é possível misturar `--old` e `--old-file` na mesma invocação (troca deliberada em favor da segurança).

- Alternativas consideradas:
  1. Ler o conteúdo do stdin com um separador de protocolo. Rejeitado: conflita com `--multi` e `--between`, que já consomem o stdin.
  2. Aumentar o ARG_MAX via sysctl. Rejeitado: exige root, não é portável e não resolve a questão fundamental de design.
  3. **Apenas documentar o `--multi` como contorno.** Rejeitado: a codificação JSON é propensa a erros para os agentes, e o `--old-file` é mais simples.

- Gatilho para revisitar: Se a multiplexação de stdin se tornar viável (por exemplo, via pipes nomeados ou passagem de fd).
