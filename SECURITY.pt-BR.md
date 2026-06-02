[Read in English](SECURITY.md)


# Política de Segurança


## Versões Suportadas
- Apenas a release mais recente recebe atualizações de segurança
- Atualize para a versão mais recente antes de reportar

| Versão  | Suportada           |
|---------|---------------------|
| 0.1.2   | Sim                 |
| 0.1.1   | Sim                 |
| 0.1.0   | Não (atualize)      |
| < 0.1.0 | Não (pré-release)   |


## Reportando uma Vulnerabilidade
- Reporte vulnerabilidades de segurança de forma privada via GitHub Security Advisories
- Navegue até a aba Security do repositório e selecione "Report a vulnerability"
- NÃO abra uma issue pública para vulnerabilidades de segurança
- Inclua: versão do atomwrite, SO, descrição da vulnerabilidade, passos para reproduzir, impacto potencial


## SLA de Resposta
- Confirmação de recebimento em até 48 horas após submissão do reporte
- Avaliação inicial e classificação de severidade em até 5 dias úteis
- Atualizações de status pelo menos a cada 7 dias até a resolução


## SLA de Correção
- Severidade crítica: patch em até 7 dias
- Severidade alta: patch em até 14 dias
- Severidade média: patch em até 30 dias
- Severidade baixa: patch na próxima release agendada


## Política de Divulgação
- Divulgação coordenada: corrigir primeiro, divulgar depois
- O reporter recebe crédito, a menos que solicite anonimato
- A divulgação pública ocorre após a correção ser publicada no crates.io
- A divulgação inclui: CVE (se aplicável), versões afetadas, versão corrigida, descrição, mitigação


## Política de Atualização de Segurança
- Patches de segurança são lançados como point releases (ex: 0.1.2)
- Anúncios são publicados via GitHub Security Advisories
- Usuários devem se inscrever nas notificações do repositório para atualizações oportunas

## Advisories de Segurança Conhecidas (v0.1.2)
- **RUSTSEC-2026-0009** em `time 0.3.45` (transitivo via `tracing-appender`): DoS via exaustão de pilha no parsing de tempo. A correção requer `time >= 0.3.47` que precisa de Rust 1.88. Nossa MSRV é 1.85, e atomwrite usa `time` apenas via `tracing-appender` para timestamps de log — não explorável via entrada do usuário. Reconhecido em `deny.toml` e rastreado para bump de MSRV em v0.2.0.


## Hall da Fama
- Pesquisadores de segurança que reportam vulnerabilidades válidas são reconhecidos aqui
- Reporte para ser listado (ou solicite anonimato)


## Boas Práticas
- Use `--workspace` para restringir operações à raiz do projeto
- Evite executar atomwrite como root
- Valide `--expect-checksum` em ambientes multi-agente
- Revise a saída de erro NDJSON para os campos `retryable` e `suggestion`
- Mantenha o atomwrite atualizado para a versão mais recente
- Audite o manifesto `batch` antes da execução em produção
