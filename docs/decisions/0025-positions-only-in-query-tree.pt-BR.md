# ADR-0025: --positions é opt-in e limitado aos modos --query/--tree

- Status: Aceito
- Data: 2026-06-07
- Contexto: O `query` da v14 tem quatro modos: `--kinds` (conta kinds), `--query <KIND>` (emite os nós que casam), `--tree` (despeja todos os nós) e `--positions` (adiciona `start_line`, `start_column`, `byte_offset` a cada nó). O usuário pode querer posições em qualquer modo, mas emiti-las em `--kinds` seria sem sentido (kinds são contagens, não posições).
- Decisão: `--positions` vale nos modos `--query <KIND>` e `--tree`. Ele é ignorado silenciosamente em `--kinds` (com um aviso no stderr em builds `cfg(debug_assertions)`). Cada linha NDJSON de um nó que casou ganha um campo `positions` quando `--positions` está ligado.
- Consequências:
  - + A saída NDJSON é concisa por padrão.
  - + Usuários avançados obtêm intervalos de bytes para processamento posterior (por exemplo, extrair o corpo de uma função para outro arquivo).
  - + O formato de saída é estável: o campo `positions` está presente quando ativado e ausente caso contrário.
  - - O comportamento "ignorado silenciosamente em --kinds" surpreende, por isso adicionamos um aviso de debug.
  - - Quem quer posições na saída de `--kinds` precisa trocar para `--query <KIND> --positions` e filtrar.
- Alternativas consideradas:
  1. Sempre emitir posições. Rejeitado: incha a saída nos 95% dos casos em que o usuário só quer os nomes dos kinds.
  2. Separar `--positions` e `--kinds --positions` em duas flags. Rejeitado: explosão combinatória de flags.
  3. Fazer `--positions` funcionar em todo lugar, até em `--kinds`. Rejeitado: a saída de `--kinds` são tuplas `{kind, count}`, e posições não fazem sentido ali.
- Gatilho para revisitar: Se os usuários pedirem "um histograma de funções por arquivo", podemos adicionar um modo dedicado `--function-locations`.
