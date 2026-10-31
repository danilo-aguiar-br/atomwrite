# ADR-0045: Sugestões acionáveis para erros de parse do clap

- Status: Aceito
- Data: 2026-06-21
- Contexto: `ARGUMENT_PARSE_ERROR` (exit 2) era o único código de erro sem um campo `suggestion` sensível ao contexto no envelope de erro JSON. Quando o clap rejeitava argumentos envolvendo `--old`/`--new` (por exemplo, conteúdo com hífens iniciais passado por expansão de shell), o campo de sugestão ficava `null` ou continha a dica genérica do clap (`to pass '-' as a value, use '-- -'`). Essa dica está incorreta para o modo edit multi-par (o separador `--` encerra todo o parse de flags, quebrando os pares `--old`/`--new` seguintes) e não menciona `--old-file`/`--new-file`, a alternativa segura que lê o conteúdo de arquivos dentro do processo atomwrite, contornando totalmente a expansão de shell. Em pipelines de agentes LLM, a falha em cascata era: exit 2 mascarado pelo pipe com jaq → loops de nova tentativa do agente com contornos ineficazes → o agente recorre a um `write` truncante → perda catastrófica de dados (documentada no ADR-0041). As flags `--old-file`/`--new-file` (ADR-0044, v0.1.23) resolvem a causa raiz, mas não eram descobríveis no momento do erro.

- Decisão: Substituir a chamada direta a `extract_clap_tip()` por `enrich_clap_suggestion()` no caminho de tratamento de erros do clap (main.rs:67). A nova função aplica uma heurística: se a mensagem de erro menciona `--old`, `--new`, `--after-match`, `--before-match`, `--between`, ou contém padrões que indicam uma falha de parse de valor com hífen (`wasn't expected`, `unexpected argument`, dica mencionando `'--'`), a sugestão é enriquecida com uma orientação acionável apontando para `--old-file`/`--new-file`. A dica original do clap é preservada como um sufixo entre parênteses. Além disso, o texto de help de `--old` e `--new` no `EditArgs` agora inclui uma dica: "for content >1KB or with special chars, prefer --old-file/--new-file".

- Consequências:
  - + Os agentes recebem orientação acionável já no PRIMEIRO erro, eliminando loops de nova tentativa.
  - + Consistência com os outros 25 códigos de erro, que já têm `suggestion` sensível ao contexto.
  - + Elimina a cadeia de falha em cascata até a perda de dados documentada no ADR-0041.
  - + Nenhuma mudança de schema, pois o campo `suggestion` já existe no `error-output.schema.json`.
  - + O texto do help oferece descobribilidade mesmo antes de um erro ocorrer.
  - - (aceitável) Falsos positivos são possíveis se um subcomando futuro introduzir uma flag chamada `--old` ou `--new`. A probabilidade é extremamente baixa dado o vocabulário de comandos existente.
  - - (aceitável) A heurística casa com o conteúdo textual da mensagem de erro, que pode mudar entre versões do clap. Mitigação: testes de regressão verificam o conteúdo da sugestão.

- Alternativas consideradas:
  1. Modificar o clap upstream para suportar injeção de sugestão customizada. Rejeitado: o clap v4 não tem API para sugestões de erro por campo, e o tipo `Error` é opaco depois de gerado.
  2. Abordagem apenas de documentação (atualizar o AGENTS.md). Rejeitado: os agentes não leem a documentação no momento do erro, e consomem o campo `suggestion` de forma programática.
  3. Sempre injetar a sugestão de --old-file, independentemente do contexto do erro. Rejeitado: produziria sugestões irrelevantes para erros em subcomandos que não têm `--old`/`--new` (por exemplo, `search`, `hash`).

- Gatilho para revisitar: Se o clap v5 introduzir um callback `suggestion_fn` ou uma API de personalização de erro por argumento.
