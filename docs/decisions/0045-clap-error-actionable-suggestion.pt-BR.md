# ADR-0045: Sugestões acionáveis para erros de parsing do clap

- **Status**: Aceito
- **Data**: 2026-06-21
- **Contexto**: `ARGUMENT_PARSE_ERROR` (exit 2) era o único código de erro sem campo `suggestion` context-aware no envelope JSON de erro. Quando o clap rejeitava argumentos envolvendo `--old`/`--new` (ex.: conteúdo com hífens iniciais passado via expansão shell), o campo suggestion era `null` ou continha o tip genérico do clap (`to pass '-' as a value, use '-- -'`). Este tip é incorreto para o modo multi-par de edit (o separador `--` termina o parsing de flags, quebrando pares `--old`/`--new` subsequentes) e não menciona `--old-file`/`--new-file` — a alternativa segura que lê conteúdo de arquivos dentro do processo atomwrite, contornando expansão shell. Em pipelines de agentes LLM, a cascata era: exit 2 mascarado pelo pipe jaq → loops de retry do agente com workarounds ineficazes → agente recorre a `write` truncante → perda catastrófica de dados (documentado no ADR-0041). As flags `--old-file`/`--new-file` (ADR-0044, v0.1.23) resolvem a causa raiz mas não eram descobríveis no momento do erro.

- **Decisão**: Substituir a chamada direta a `extract_clap_tip()` por `enrich_clap_suggestion()` no caminho de tratamento de erros do clap (main.rs:67). A nova função aplica uma heurística: se a mensagem de erro menciona `--old`, `--new`, `--after-match`, `--before-match`, `--between`, ou contém padrões indicando falha de parsing de valor com hífen (`wasn't expected`, `unexpected argument`, tip mencionando `'--'`), a sugestão é enriquecida com orientação acionável apontando para `--old-file`/`--new-file`. O tip original do clap é preservado como sufixo entre parênteses. Adicionalmente, o help text de `--old` e `--new` em `EditArgs` agora inclui hint: "for content >1KB or with special chars, prefer --old-file/--new-file".

- **Consequências**:
  - **+** Agentes recebem orientação acionável no PRIMEIRO erro, eliminando loops de retry
  - **+** Consistência com todos os outros 25 códigos de erro que já têm `suggestion` context-aware
  - **+** Elimina a cadeia cascata para perda de dados documentada no ADR-0041
  - **+** Zero mudança de schema — campo `suggestion` já existe em `error-output.schema.json`
  - **+** Help text proporciona descobribilidade antes mesmo de um erro ocorrer
  - **-** (aceitável) Falsos positivos possíveis se um subcomando futuro introduzir flag chamada `--old` ou `--new`. Probabilidade extremamente baixa dado o vocabulário existente de comandos
  - **-** (aceitável) A heurística casa com conteúdo textual da mensagem de erro, que pode mudar entre versões do clap. Mitigação: testes de regressão verificam o conteúdo da suggestion

- **Alternativas consideradas**:
  1. **Modificar clap upstream para suportar injeção de sugestões customizadas.** Rejeitado: clap v4 não tem API para sugestões por campo de erro; o tipo `Error` é opaco após geração.
  2. **Abordagem apenas documentação (atualizar AGENTS.md).** Rejeitado: agentes não leem documentação no momento do erro; consomem o campo `suggestion` programaticamente.
  3. **Sempre injetar a sugestão --old-file independente do contexto do erro.** Rejeitado: produziria sugestões irrelevantes para erros em subcomandos que não têm `--old`/`--new` (ex.: `search`, `hash`).

- **Gatilho para revisitar**: Se clap v5 introduzir callback `suggestion_fn` ou API de customização de erros por argumento.
