# 0043-shrink-guard (PT-BR) — Tradução

_Tradução automática do ADR original em inglês. Em caso de divergência, a versão EN prevalece._

# ADR-0043: guarda de shrink com --expect-checksum

- **Status**: Aceito
- **Data**: 2026-06-19
- **Contexto**: O `--expect-checksum` valida APENAS concorrência (match de hash), NÃO correção de conteúdo. A função `verify_checksum()` retorna `Ok(())` quando os hashes conferem sem inspecionar o tamanho do stdin. O `risk_assessment` (L1, do ADR-0035 v0.1.20) calcula o delta de tamanho mas apenas emite um warning via `eprintln!` — nunca bloqueia a operação. No incidente de 2026-06-15, `--expect-checksum` passou (o arquivo não havia sido modificado por terceiros) mas o write reduziu o arquivo de 122.994 para 16.780 bytes (redução de 86%). O agente acreditava estar protegido pelo `--expect-checksum` porque a documentação diz "USAR `--expect-checksum` para locking otimista (detecção de state drift)" — linguagem que sugere segurança quando apenas fornece controle de concorrência. Agentes LLM consomem stdout (NDJSON), NÃO stderr — o warning do L1 era invisível para o agente. O exit code era 0 ("sucesso") mesmo após destruir 86% do conteúdo. A combinação `--expect-checksum` + exit 0 + `status: "success"` criou confiança falsa de que a operação era segura.

- **Decisão**: Adicionar guarda de shrink que bloqueia writes que reduzem o arquivo em mais de 50% quando `--expect-checksum` está ativo. A guarda roda APÓS `verify_checksum()` e ANTES de `atomic_write()`. Quando disparada: retorna exit 65 (`INVALID_INPUT`) com mensagem `"stdin is {pct}% smaller than target ({original} → {new} bytes); pass --allow-shrink to confirm"`. Adicionar flag `--allow-shrink` em `WriteArgs` para override explícito quando truncamento intencional é desejado. Tornar `risk_assessment` (L1) bloqueante quando `--expect-checksum` está ativo E shrink é detectado: sem `--expect-checksum`, L1 permanece informativo (warning no stderr) — comportamento atual preservado; com `--expect-checksum`, L1 se torna bloqueante — exit 65 quando delta excede `--risk-threshold`. Racional: se o chamador se preocupou em passar `--expect-checksum`, ele quer proteção real. Sem `--expect-checksum`, o comportamento é completamente inalterado (compatível retroativamente).

- **Consequências**:
  - **+** Agentes usando `--expect-checksum` recebem proteção REAL contra truncamento acidental — o incidente de 2026-06-15 teria sido bloqueado.
  - **+** Zero falsos positivos para crescimento (a guarda só bloqueia shrink, não crescimento). Escrever um arquivo maior que o original sempre tem sucesso.
  - **+** `--allow-shrink` fornece override explícito para truncamento intencional — zero fricção para casos de uso legítimos.
  - **+** A guarda é zero-cost quando o stdin é maior que o original (uma comparação de inteiros).
  - **+** A resposta de erro inclui `shrink_blocked: true` e `shrink_pct: N` para que agentes entendam POR QUE a operação foi bloqueada e possam tomar ação corretiva.
  - **-** (aceitável) Truncamento intencional com `--expect-checksum` agora requer `--allow-shrink`. Este é um tradeoff deliberado: a guarda protege o caso comum (truncamento acidental) ao custo de uma flag extra para o caso raro (truncamento intencional com controle de concorrência).
  - **-** (aceitável) A guarda só ativa com `--expect-checksum`. Writes sem `--expect-checksum` NÃO são protegidos por esta guarda — esse caso é coberto pelo GAP-016 (backup-by-default) que fornece um mecanismo de recuperação em vez de uma guarda bloqueante.

- **Alternativas consideradas**:
  1. **Sempre bloquear shrink independentemente de `--expect-checksum`.** Rejeitado: quebra compatibilidade retroativa para scripts que truncam arquivos intencionalmente via `write`. O portão `--expect-checksum` garante que a guarda só ativa quando o chamador expressou intenção de operações seguras.
  2. **Apenas emitir warning (manter L1 informativo).** Rejeitado: agentes não leem stderr. O incidente de 2026-06-15 prova que guardas apenas informativas são invisíveis para agentes LLM. A guarda deve ser bloqueante (exit não-zero) para ser eficaz.
  3. **Usar threshold configurável em vez de 50% fixo.** Aceito como parcial: a flag `--risk-threshold` (do ADR-0035 L1) já fornece configurabilidade. O default de 50% é o piso rígido da guarda de shrink; `--risk-threshold` pode reduzi-lo ainda mais. Um arquivo que encolhe 51% é quase certamente um erro; um arquivo que encolhe 10% pode ser limpeza legítima.
  4. **Adicionar `--expect-checksum-strict` como flag separada.** Rejeitado: adicionar outra flag aumenta a carga cognitiva. A guarda de shrink ativa automaticamente quando `--expect-checksum` está presente — nenhuma nova flag para lembrar. `--allow-shrink` é a escotilha de escape, não um novo modo.

- **Gatilho para revisitar**: Se casos de uso legítimos para shrink >50% com `--expect-checksum` forem comuns (ex.: rotação de arquivos de config, truncamento de logs), considerar reduzir o threshold para 75% ou torná-lo configurável via `ATOMWRITE_SHRINK_THRESHOLD`.


---

_Original em inglês: [`0043-shrink-guard.md`](0043-shrink-guard.md)_
