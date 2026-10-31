# ADR-0043: guarda de encolhimento com --expect-checksum

> Status: Superado na v0.1.37 quanto ao alcance da guarda. Uma escrita que encolhe o arquivo em mais de 50% é bloqueada com exit 65 mesmo sem `--expect-checksum` (medido: 1000 -> 1 bytes). `--allow-shrink` é a sobrescrita.

> Histórico (pré-0.1.35): os knobs de produto `ATOMWRITE_*` / env descritos abaixo foram superados. A configuração de runtime é apenas flags de CLI + `config.toml` XDG / `atomwrite set|get`.


- Status: Aceito
- Data: 2026-06-19
- Contexto: `--expect-checksum` valida apenas a concorrência (o hash bate), NÃO a correção do conteúdo. A função `verify_checksum()` devolve `Ok(())` quando os hashes batem, sem inspecionar o tamanho do stdin. O `risk_assessment` (L1, do ADR-0035 v0.1.20) calcula a variação de tamanho, mas só emite um aviso via `eprintln!` e nunca bloqueia a operação. No incidente de 2026-06-15, o `--expect-checksum` passou (o arquivo não havia sido modificado por terceiros), mas a escrita encolheu o arquivo de 122.994 para 16.780 bytes (redução de 86%). O agente acreditava estar protegido pelo `--expect-checksum` porque a documentação diz "USE `--expect-checksum` para locking otimista (detecção de state drift)", uma linguagem que sugere segurança quando só oferece controle de concorrência. Agentes LLM consomem o stdout (NDJSON), NÃO o stderr, então o aviso do L1 era invisível para o agente. O exit code foi 0 ("sucesso") mesmo após destruir 86% do conteúdo. A combinação `--expect-checksum` + exit 0 + `status: "success"` criou uma falsa confiança de que a operação era segura.

- Decisão: Adicionar uma guarda de encolhimento que bloqueia escritas que reduzam o arquivo em mais de 50% quando `--expect-checksum` está ativo. A guarda roda DEPOIS do `verify_checksum()` e ANTES do `atomic_write()`. Quando disparada: devolve exit 65 (`INVALID_INPUT`) com a mensagem `"stdin is {pct}% smaller than target ({original} → {new} bytes); pass --allow-shrink to confirm"`. Adicionar a flag `--allow-shrink` ao `WriteArgs` para sobrescrita explícita quando o truncamento é intencional. Tornar o `risk_assessment` (L1) bloqueante quando `--expect-checksum` está ativo E o encolhimento é detectado: sem `--expect-checksum`, o L1 continua informativo (aviso no stderr), preservando o comportamento atual, e com `--expect-checksum`, o L1 passa a bloquear, com exit 65 quando a variação excede `--risk-threshold`. Justificativa: se o chamador se importou o bastante para passar `--expect-checksum`, ele quer proteção de verdade. Sem `--expect-checksum`, o comportamento fica completamente inalterado (retrocompatível).

- Consequências:
  - + Agentes que usam `--expect-checksum` recebem proteção real contra truncamento acidental: o incidente de 2026-06-15 teria sido bloqueado.
  - + Zero falsos positivos para crescimento (a guarda só bloqueia encolhimento, não crescimento). Gravar um arquivo maior que o original sempre tem sucesso.
  - + `--allow-shrink` oferece sobrescrita explícita para truncamento intencional, com atrito zero nos casos de uso legítimos.
  - + A guarda tem custo zero quando o stdin é maior que o original (uma comparação de inteiros).
  - + A resposta de erro inclui `shrink_blocked: true` e `shrink_pct: N`, para que os agentes entendam POR QUE a operação foi bloqueada e possam tomar ação corretiva.
  - - (aceitável) O truncamento intencional com `--expect-checksum` agora exige `--allow-shrink`. É uma troca deliberada: a guarda protege o caso comum (truncamento acidental) ao custo de uma flag extra no caso raro (truncamento intencional com controle de concorrência).
  - - (aceitável) A guarda só é ativada com `--expect-checksum`. Escritas simples sem `--expect-checksum` NÃO são protegidas por esta guarda: esse caso é coberto pelo GAP-016 (backup por padrão), que oferece um mecanismo de recuperação em vez de uma guarda bloqueante.

- Alternativas consideradas:
  1. **Sempre bloquear o encolhimento, independentemente de `--expect-checksum`.** Rejeitado: quebra a retrocompatibilidade de scripts que truncam arquivos intencionalmente via `write`. O gate de `--expect-checksum` garante que a guarda só seja ativada quando o chamador expressou intenção de operações seguras.
  2. Apenas emitir um aviso (manter o L1 informativo). Rejeitado: agentes não leem o stderr. O incidente de 2026-06-15 prova que guardas apenas informativas são invisíveis para agentes LLM. A guarda precisa ser bloqueante (exit diferente de zero) para ser eficaz.
  3. Usar um limite configurável em vez de 50% fixos. Aceito parcialmente: a flag `--risk-threshold` (do L1 do ADR-0035) já oferece configurabilidade. Os 50% padrão são o piso rígido da guarda de encolhimento, e o `--risk-threshold` pode reduzi-lo ainda mais. Um arquivo que encolhe 51% é quase certamente um erro, enquanto um que encolhe 10% pode ser uma limpeza legítima.
  4. **Adicionar `--expect-checksum-strict` como uma flag separada.** Rejeitado: outra flag aumenta a carga cognitiva. A guarda de encolhimento é ativada automaticamente quando `--expect-checksum` está presente, sem uma nova flag para lembrar. `--allow-shrink` é a válvula de escape, não um novo modo.

- Gatilho para revisitar: Se casos de uso legítimos de encolhimento de mais de 50% com `--expect-checksum` forem comuns (por exemplo, rotação de arquivos de configuração, truncamento de logs), considerar reduzir o limite para 75% ou torná-lo configurável via `ATOMWRITE_SHRINK_THRESHOLD`.
