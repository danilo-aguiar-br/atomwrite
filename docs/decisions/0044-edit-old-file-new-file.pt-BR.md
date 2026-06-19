# 0044-edit-old-file-new-file (PT-BR) — Tradução

_Tradução automática do ADR original em inglês. Em caso de divergência, a versão EN prevalece._

# ADR-0044: --old-file/--new-file para o comando edit

- **Status**: Aceito
- **Data**: 2026-06-19
- **Contexto**: `edit --old`/`--new` aceitam conteúdo apenas via argumentos CLI. A expansão do shell `$(cat file)` injeta conteúdo no argv antes do `execve(2)`. O ARG_MAX do kernel Linux é 2.097.152 bytes para argv+envp combinados; o limite efetivo para um único argumento é ~131 KB. Conteúdo acima desse limite causa E2BIG (errno 7) — o processo do atomwrite nunca inicia e o agente recebe exit 126 do shell sem envelope JSON. O workaround `edit --multi` com NDJSON via stdin existe mas requer codificação JSON propensa a erros. Agentes usam `$(cat)` por simplicidade e atingem E2BIG silenciosamente em arquivos grandes. A documentação não menciona ARG_MAX nem sugere `--multi` como alternativa.

- **Decisão**: Adicionar `--old-file <PATH>` e `--new-file <PATH>` ao `EditArgs` como alternativas a `--old` e `--new`. Os arquivos são lidos dentro do processo do atomwrite, contornando a expansão do shell e o ARG_MAX. Usar `conflicts_with` do clap para impedir mistura de `--old` com `--old-file` (prevenção de Silent Argument Discard conforme rules-rust-cli-stdin-stdout-silent-discard). Validar caminhos contra o jail do workspace. Adicionar campo `source: "arg"|"file"` ao `PairResult` para rastreabilidade. Adicionar validação em runtime no `resolve_edit_pairs()` para rejeitar mistura cruzada de `--old` com `--new-file` ou `--old-file` com `--new` — exit 65 (`INVALID_INPUT`) com mensagem 'cannot mix --old with --new-file or --old-file with --new; use both from the same source'. Adicionar `strip_file_trailing_newline()` para remover exatamente um newline final (`\n` ou `\r\n`) do conteúdo do arquivo para paridade com o comportamento do argv — arquivos criados por `echo` incluem um newline final que valores de argv nunca possuem.

- **Consequências**:
  - **+** Elimina o limite ARG_MAX para agentes — conteúdo de qualquer tamanho pode ser usado.
  - **+** Zero problemas de expansão do shell ($, crases, aspas, etc.).
  - **+** `conflicts_with` previne descarte silencioso de `--old` quando `--old-file` está presente.
  - **+** `PairResult.source` fornece rastreabilidade da origem do conteúdo.
  - **+** A guarda de mistura cruzada captura o caso que o `conflicts_with` do clap não consegue: `--old` pareado com `--new-file` (ou vice-versa).
  - **+** A remoção do newline final garante que `--old-file old.txt` case o mesmo conteúdo que `--old "text"` mesmo quando `old.txt` foi criado por `echo "text" > old.txt`.
  - **-** (aceitável) Agentes precisam escrever arquivos temporários antes do edit (workflow de dois passos).
  - **-** (aceitável) Não é possível misturar `--old` e `--old-file` na mesma invocação (tradeoff de segurança deliberado).

- **Alternativas consideradas**:
  1. **Ler conteúdo do stdin com separador de protocolo.** Rejeitado: conflita com `--multi` e `--between` que já consomem stdin.
  2. **Aumentar ARG_MAX via sysctl.** Rejeitado: requer root, não é portável, não resolve o problema fundamental de design.
  3. **Apenas documentar `--multi` como workaround.** Rejeitado: a codificação JSON é propensa a erros para agentes; `--old-file` é mais simples.

- **Gatilho para revisitar**: Se a multiplexação de stdin se tornar viável (ex.: via named pipes ou passagem de fd).


---

_Original em inglês: [`0044-edit-old-file-new-file.md`](0044-edit-old-file-new-file.md)_
