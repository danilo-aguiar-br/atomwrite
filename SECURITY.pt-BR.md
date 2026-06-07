[Read in English](SECURITY.md)


# Política de Segurança


## Versões Suportadas
- Apenas a release mais recente recebe atualizações de segurança
- Atualize para a versão mais recente antes de reportar
- Releases mais antigas recebem patches best-effort para vulnerabilidades críticas

| Versão  | Suportada          |
|---------|--------------------|
| 0.1.12  | Sim                |
| 0.1.11  | Best-effort        |
| 0.1.10  | Best-effort        |
| 0.1.9   | Best-effort        |
| 0.1.8   | Best-effort        |
| 0.1.7   | Best-effort        |
| 0.1.6   | Best-effort        |
| 0.1.5   | Best-effort        |
| 0.1.4   | Best-effort        |
| 0.1.3   | Fim de vida        |
| 0.1.2   | Fim de vida        |
| 0.1.1   | Fim de vida        |
| 0.1.0   | Fim de vida        |
| < 0.1.0 | Não lançada        |


## Reportando uma Vulnerabilidade
- Reporte vulnerabilidades de segurança de forma privada via GitHub Security Advisories
- Navegue até a aba Security do repositório e selecione "Report a vulnerability"
- NÃO abra uma issue pública para vulnerabilidades de segurança
- Inclua: versão do atomwrite, SO, versão do Rust, descrição da vulnerabilidade, passos para reproduzir, impacto potencial, prova de conceito se disponível


## SLA de Resposta
- Reconhecimento dentro de 48 horas após o envio do reporte
- Avaliação inicial e classificação de severidade dentro de 5 dias úteis
- Atualizações de status pelo menos a cada 7 dias até a resolução


## SLA de Correção
- Severidade crítica: patch dentro de 7 dias
- Severidade alta: patch dentro de 14 dias
- Severidade média: patch dentro de 30 dias
- Severidade baixa: patch na próxima release agendada


## Política de Divulgação
- Divulgação coordenada: correção primeiro, divulgação depois
- O reporter é creditado a menos que solicite anonimato
- A divulgação pública ocorre após a correção ser publicada no crates.io
- A divulgação inclui: CVE (se aplicável), versões afetadas, versão corrigida, descrição, mitigação


## Política de Atualização de Segurança
- Patches de segurança são lançados como releases pontuais (ex.: 0.1.13, 0.1.14)
- Anúncios são postados via GitHub Security Advisories
- Usuários devem se inscrever em notificações do repositório para atualizações em tempo hábil


## Advisories de Segurança Conhecidas (Resolvidas em v0.1.12)

### RUSTSEC-2026-0009 em `time 0.3.45` (transitivo via `tracing-appender`)
- **Status**: Resolvida em v0.1.7 (2026-06-05)
- **Problema**: DoS via stack exhaustion em parsing de time
- **Correção**: Upgrade de `time` para 0.3.47 com `DEPTH_LIMIT=32`. A correção requer Rust 1.88
- **Ação tomada**: MSRV bumped de 1.85 para 1.88 em v0.1.7. A entrada `ignore` no `deny.toml` e a flag `cargo audit --ignore` foram ambas removidas. A advisory não se aplica mais
- **Referência**: Entrada v0.1.7 no `CHANGELOG.pt-BR.md`

### Sem advisories ativas em v0.1.12
- `cargo audit` reporta 0 vulnerabilidades em 379 crates
- `cargo deny check` reporta 4/4 OK (advisories, bans, licenses, sources)
- Todas as dependências transitivas com notas de segurança foram atualizadas ou substituídas


## Postura de Segurança de Dependências (v0.1.12)
- **Segurança de memória**: 0 blocos de código unsafe em `src/` (negado via `#![deny(unsafe_code)]` na raiz da lib)
- **BLAKE3**: Usado apenas para checksums, não para segurança criptográfica
- **tree-sitter-language-pack**: Parsers são baixados no primeiro uso a partir das releases oficiais do `tree-sitter` no GitHub via a feature `download`. Os parsers baixados são carregados dinamicamente mas não executados como código
- **deny.toml**: Inclui `MPL-2.0`, `CDLA-Permissive-2.0`, `CC0-1.0` no allowlist. Tem 10 skip entries para a coexistência inevitável de `getrandom` 0.2/0.3, `rustix` 0.x/1.x, e `windows-sys` 0.52/0.59 nas árvores de dependência
- **MSRV**: Rust 1.88 stable


## Hall da Fama
- Pesquisadores de segurança que reportam vulnerabilidades válidas são reconhecidos aqui
- Reporte para ser listado (ou solicitar anonimato)
- Para solicitar listagem, abra uma GitHub Security Advisory com o reporte e inclua sua atribuição preferida


## Melhores Práticas para Usuários
- Use `--workspace` para restringir operações à raiz do projeto
- Evite executar atomwrite como root
- Valide `--expect-checksum` em ambientes multi-agente
- Revise a saída de erro NDJSON para os campos `retryable` e `suggestion`
- Mantenha o atomwrite atualizado para a versão mais recente
- Audite o manifesto `batch` antes da execução em produção
- Inscreva-se em notificações do repositório para atualizações de segurança em tempo hábil
- Use `--strict-atomic` apenas quando entender o trade-off (proíbe cross-device copy-fallback, exit 91 em EXDEV)
- Trate journals órfãos (sidecars `.atomwrite.journal.*` de um crash anterior) com suspeita: inspecione o arquivo alvo E o conteúdo do journal antes de deletar o journal
- Quando a verificação G72 syntax check estiver habilitada (`--syntax-check`), NÃO faça pipe de conteúdo sensível pelo stdin em sistemas compartilhados: o envelope de erro pode ecoar a localização da fonte e contexto ao redor
