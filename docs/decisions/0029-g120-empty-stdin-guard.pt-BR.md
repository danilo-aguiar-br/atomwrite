# ADR-0029: G120 — Cross-validation explícita de `--append` + `--expect-checksum` + empty stdin

- **Status**: Accepted
- **Date**: 2026-06-13
- **Context**: G120 fechou L1 (rejeitar stdin vazio por padrão), L2 (rejeitar append/prepend com stdin vazio) e L4 (campo `stdin_bytes_read` no envelope `WriteOutput`) na v0.1.15. Faltava L3: cross-validation estruturada. O caso problemático: caller combina `--append --expect-checksum <HASH>` e passa stdin vazio por acidente (pipe upstream quebrou, heredoc malformado). Hoje o checksum é comparado contra o arquivo pré-mutação (passa se HASH bate) e o append é no-op (0 bytes). O resultado: exit 0 silencioso, mas a intenção do caller era claramente errada — o caller passou `--append` esperando ANEXAR algo. Sem L3, o agente não tem como distinguir "intencional no-op" de "upstream quebrou e eu não percebi". G118 fechou path resolution; G120 L1+L2 fechou validação de input; L3 fecha a última peça — validação cross-flag que detecta intenção ambígua.

- **Decision**: Implementar L3 como warning estruturado via `tracing` + flag opt-out `--no-checksum-when-empty` em `write`:
  1. **Detecção**: em `cmd_write`, quando `stdin_bytes_read == 0 && (args.append || args.prepend) && args.expect_checksum.is_some()`, ativar a lógica L3.
  2. **Comportamento padrão**: `tracing::info!` no stderr documenta a combinação cross-flag e prossegue com `verify_checksum` (que ainda valida o estado pré-mutação). O caller que passou `--allow-empty-stdin` declarou intent explícita; L3 apenas loga.
  3. **Opt-out explícito**: `--no-checksum-when-empty` suprime o `verify_checksum` quando stdin está vazio. O `tracing::warn!` registra a decisão.
  4. **Falha mantida**: o L1 (`read_stdin_content` rejeita stdin vazio por padrão) e L2 (`handle_append_prepend` rejeita append+empty) permanecem inalterados. L3 é ADITIVO: só roda quando L1/L2 foram explicitamente bypassados via `--allow-empty-stdin`.
  5. **Telemetria**: o campo `stdin_bytes_read` (L4) já presente permite detecção tardia via grep em logs; L3 adiciona o sinal cross-flag explícito no stderr.

- **Consequences**:
  - **+** L3 captura a última classe de ambiguidade do bug composto do G120: caller que combina `--append --expect-checksum` E stdin vazio recebe warning estruturado no stderr.
  - **+** Opt-out explícito (`--no-checksum-when-empty`) preserva a opção de "no-op intencional" sem surpresa.
  - **+** `tracing::info` no caminho default é observável mas não bloqueante — o caller que não monitora stderr não vê diferença; o que monitora vê o sinal.
  - **+** L3 NÃO introduz novo exit code: o comportamento de exit continua igual. A diferença é visibilidade, não controle.
  - **-** Operador que monitora stderr vai ver mais linhas. Mitigado: usa `tracing::info` (não warn), filtrável por `--verbose` se desejado.
  - **-** L3 só trata append/prepend + checksum + empty. Não cobre `write` puro + empty + checksum (mas L1 já rejeita isso). Não cobre `--prepend + empty` (mas L2 já rejeita). Cobertura completa com L1+L2+L3.

- **Alternatives considered**:
  1. Erro fatal em L3 (`return Err(InvalidInput)`). Rejeitado: L1+L2 já protegem; L3 é complemento observacional, não defesa principal. Forçar exit code quebraria a regra "L1 opt-in cobre 99% dos casos".
  2. Adicionar campo `cross_flag_warning: Option<String>` ao envelope `WriteOutput` em vez de tracing. Considerado mas rejeitado: warnings estruturados em NDJSON poluem o output para agentes que parseiam campos. `tracing` no stderr é a camada correta para sinais não-bloqueantes.
  3. Auto-aplicar `--no-checksum-when-empty` quando empty stdin + append + checksum detectados. Rejeitado: decisão implícita esconde intent; opt-out explícito é mais transparente.
  4. Adicionar `--strict-empty-stdin` que falha em QUALQUER combinação de stdin vazio. Rejeitado: redundante com L1.

- **Trigger to revisit**: Se a telemetria de produção mostrar que agentes ignoram os warnings de L3 consistentemente (audit log de 0 leituras em 30 dias), considerar promover o warning para stderr de primeira classe ou exit code distinto. Se a combinação `--append --expect-checksum` + empty stdin provar-se comum em pipelines legítimos, reverter L3 para opt-out implícito.
