# ADR-0033: v0.1.19 — Consolidação do desvio de documentação dos códigos de saída

> Status: Superado na v0.1.37 quanto aos itens 1 e 4. Uma divergência de `--verify-checksum` no `read` devolve `CHECKSUM_VERIFY_FAILED` (exit 81), e não `STATE_DRIFT` (82), e o `BROKEN_PIPE` (141) vem de um handler cooperativo de SIGPIPE, e não da disposição padrão restaurada.

> Histórico (pré-0.1.35): os knobs de produto `ATOMWRITE_*` / env descritos abaixo foram superados. A configuração de runtime é apenas flags de CLI + `config.toml` XDG / `atomwrite set|get`.


- Status: Aceito
- Data: 2026-06-14
- Contexto: Os testes da Fase D em 2026-06-14 rodaram 7 sondagens concretas no nível do binário contra o release v0.1.18 e revelaram 7 pontos em que a documentação publicada (SKILL.md EN+PT, `error-output.schema.json`, README, CHANGELOG) divergia do comportamento real do binário. Cada desvio é pequeno isoladamente, mas juntos criam um ambiente em que agentes e gates locais não conseguem interpretar os códigos de saída com confiança. Os desvios são:
  1. `STATE_DRIFT` (82) vs `CHECKSUM_VERIFY_FAILED` (81) — a documentação dizia que `--verify-checksum` devolve `CHECKSUM_VERIFY_FAILED`, mas o binário devolve `STATE_DRIFT`. O código 81 é reservado para a divergência de BLAKE3 do conteúdo do arquivo no caminho do `read`. O código 82 é a falha de locking otimista, que inclui a divergência de `--expect-checksum` em writes/edits e a divergência de `--verify-checksum` em leituras.
  2. `SYNTAX_ERROR` vs `SYNTAX_ERROR_DETECTED` — a documentação da v0.1.12 chamava o código de `SYNTAX_ERROR`, e o binário da v0.1.18 emite `SYNTAX_ERROR_DETECTED`. A renomeação aconteceu no lançamento do tree-sitter (G72) da v0.1.12, mas a documentação não foi atualizada.
  3. `ORPHAN_JOURNAL` (93) é consultivo, NÃO é detectado automaticamente — a documentação sugeria que qualquer sidecar obsoleto é detectado a cada invocação. O gate real é `ATOMWRITE_WAL=1` OU `--strict-atomic`, e o `write` padrão não grava sidecar e, portanto, não pode detectar órfãos.
  4. `BROKEN_PIPE` (141) exige propagação real de SIGPIPE — a documentação sugeria que um simples pipe com `head -1` o dispara. O comportamento real é que a restauração do SIGPIPE da v0.1.4+ devolve a disposição padrão, então o sinal só é levantado quando o consumidor a jusante fecha ativamente o pipe no meio do fluxo.
  5. Leituras de arquivo binário devolvem exit 0 com metadados `kind=binary`, NÃO exit 65 — a heurística `BINARY_FILE` da v0.1.4 foi alterada para emitir um envelope estruturado e exit 0, e o caminho do código 65 agora só dispara para `read` sem `--format raw` E com a heurística binária contornada.
  6. Posicional ausente devolve `ARGUMENT_PARSE_ERROR` (exit 2), NÃO `INVALID_INPUT` (65) — erros de argumento no nível do clap são reportados como exit 2. O código 65 é reservado para validação de conteúdo em runtime (por exemplo TOML malformado, regex inválida, stdin vazio por padrão).
  7. `--workspace` ausente assume o CWD, NÃO é um erro — a documentação sugeria que `--workspace` é obrigatório, mas o comportamento real ancora caminhos relativos no CWD e só dispara `WORKSPACE_JAIL` (126) quando um caminho absoluto resolve para fora do jail efetivo.

- Decisão: Aceitar o comportamento do binário como canônico. Consolidar a documentação na v0.1.19 para coincidir com o binário. Especificamente:
  1. **STATE_DRIFT absorve CHECKSUM_VERIFY_FAILED para `--verify-checksum`** — atualizar a tabela `Exit Codes` para registrar a absorção, atualizar a `Error Code List` para marcar `CHECKSUM_VERIFY_FAILED` (81) como histórico e adicionar uma nota de desvio nos dois arquivos SKILL.
  2. **Renomear `SYNTAX_ERROR` para `SYNTAX_ERROR_DETECTED`** nas seções `Exit Codes` e `Error Code List`. O nome histórico é preservado apenas em prosa, para facilitar a busca com grep.
  3. Documentar o gate do ORPHAN_JOURNAL — adicionar texto explícito nos dois arquivos SKILL dizendo que o caminho consultivo exige `ATOMWRITE_WAL=1` ou `--strict-atomic`. O `WalPolicy::Auto` atual (v0.1.16 G119) significa que o sidecar é pulado em writes triviais, então invocações padrão nunca veem esse código.
  4. Documentar o requisito de propagação do BROKEN_PIPE — o contrato é "exit 141 é levantado quando o SIGPIPE é entregue", e não "exit 141 é levantado quando a saída é curta". A nota sobre a restauração do SIGPIPE da v0.1.4+ pertence aqui.
  5. Documentar o envelope de leitura binária — `kind=binary` é o sinal canônico, e o código 65 é o caminho secundário, de caso de borda.
  6. Documentar a divisão clap vs runtime — exit 2 é do clap, exit 65 é do runtime. A SKILL já os separa, e a seção de desvios reforça a distinção.
  7. Documentar o fallback para o CWD — `--workspace` é documentado como uma flag com padrão no CWD, não como um argumento obrigatório. A semântica do `WORKSPACE_JAIL` está atrelada ao jail efetivo (o CWD quando `--workspace` é omitido).
- Consequências:
  - + Os 7 desvios têm uma nota de uma linha na seção de desvios da v0.1.19 dos dois arquivos SKILL. Agentes e gates locais podem buscar a seção de desvios com grep quando um código de saída não bate com a tabela legada.
  - + A entrada da v0.1.19 no CHANGELOG documenta a consolidação em um único item, indexado pela data dos testes da Fase D, 2026-06-14.
  - + O ADR-0033 registra o raciocínio para que mantenedores futuros não redescubram os desvios.
  - + Nenhuma mudança no binário é necessária, a documentação passa a coincidir com o binário, e não o contrário.
  - - Operadores que automatizaram contra o nome de código legado `SYNTAX_ERROR` verão uma divergência em runtime. Mitigado: a nota de desvio da SKILL é encontrável com grep e o schema (`error-output.schema.json`) já usa `SYNTAX_ERROR_DETECTED`.
  - - O código `CHECKSUM_VERIFY_FAILED` (81) agora é histórico, e chamadores que o casavam precisam migrar para `STATE_DRIFT` (82). Nenhum foi observado na suíte de testes deste repositório.

- Alternativas consideradas:
  1. Mudar o binário para coincidir com a documentação (renomear `SYNTAX_ERROR_DETECTED` de volta para `SYNTAX_ERROR`, separar `STATE_DRIFT` e `CHECKSUM_VERIFY_FAILED` etc.). Rejeitado: quebraria o contrato do G72 da v0.1.12 que os testes já fixam, forçaria um salto de versão maior, e o novo comportamento é estritamente mais correto (por exemplo, o `STATE_DRIFT` já cobre a semântica de locking que o `CHECKSUM_VERIFY_FAILED` tentava expressar).
  2. Adicionar um opt-in `--legacy-exit-codes` que restaura os nomes da v0.1.12. Rejeitado: YAGNI. As notas de desvio bastam para a migração, e uma flag de runtime traria um custo de suporte permanente.
  3. Documentar os desvios em um arquivo separado `docs/drifts/v0-1-18.md` e linkar a partir do CHANGELOG. Rejeitado: os arquivos SKILL são a principal superfície de consulta dos agentes, e um arquivo separado é menos encontrável.
  4. Pular a nota do fallback para o CWD porque a redação `--workspace REQUIRED` na seção de flags globais já a implica. Rejeitado: a correção do GAP 13 da v0.1.4 tornou o envelope de erro sensível ao contexto, e o contrato do fallback para o CWD é a base dessa correção, então merece uma nota dedicada.

- Gatilho para revisitar: Se um release v0.2.0 introduzir um novo código de saída, copiar a seção de desvios para um capítulo de migração da v0.2.0 e converter as notas de desvio no próprio local em links históricos arquivados. Se um teste do binário falhar por causa de um código de saída não documentado, o novo desvio ganha seu próprio item aqui.
