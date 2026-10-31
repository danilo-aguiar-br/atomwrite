[Read in English](SECURITY.md)


# Política de Segurança


## Versões Suportadas
- A release mais recente recebe correções de segurança, e releases mais antigas recebem patches best-effort só para vulnerabilidades críticas
- Atualize para a versão mais recente antes de reportar, porque a correção nasce na linha atual
- A tabela lista a linha de release, e o crates.io carrega a 0.1.36 enquanto a 0.1.37 é a versão em preparação, então o pin deste documento diz qual está em teste
- As versões 0.1.15, 0.1.16, 0.1.17, 0.1.21 e 0.1.33 nunca foram publicadas, então não aparecem

| Versão  | Suportada          |
|---------|--------------------|
| 0.1.37  | Sim (em preparação) |
| 0.1.36  | Best-effort        |
| 0.1.35  | Best-effort        |
| 0.1.34  | Best-effort        |
| 0.1.32  | Best-effort        |
| 0.1.31  | Best-effort        |
| 0.1.30  | Best-effort        |
| 0.1.29  | Best-effort        |
| 0.1.28  | Best-effort        |
| 0.1.27  | Best-effort        |
| 0.1.26  | Best-effort        |
| 0.1.25  | Best-effort        |
| 0.1.24  | Best-effort        |
| 0.1.23  | Best-effort        |
| 0.1.22  | Best-effort        |
| 0.1.20  | Best-effort        |
| 0.1.19  | Best-effort        |
| 0.1.18  | Best-effort        |
| 0.1.14  | Best-effort        |
| 0.1.13  | Best-effort        |
| 0.1.12  | Best-effort        |
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
- Reporte vulnerabilidades de segurança de forma privada por e-mail para daniloaguiarbr@proton.me, que é o canal que funciona hoje
- GitHub Security Advisories pela aba Security do repositório é o segundo canal, disponível quando o repositório for publicado
- NÃO abra uma issue pública para vulnerabilidades de segurança
- Inclua: versão do atomwrite, SO, versão do Rust, descrição da vulnerabilidade, passos para reproduzir, impacto potencial, prova de conceito se disponível


## Metas de Resposta
- Estas são metas best-effort de um mantenedor só, e não um SLA contratual: reconhecimento em 48 horas, classificação de severidade em 5 dias úteis e atualização de status a cada 7 dias
- Um reporte que perde uma meta continua sendo tratado, e o repórter é avisado de qual meta foi perdida


## Metas de Correção
- Estas são metas best-effort, e a severidade segue o CVSS 3.1: crítica 9.0-10.0, alta 7.0-8.9, média 4.0-6.9, baixa abaixo de 4.0
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


## Advisories de Segurança Conhecidas

### RUSTSEC-2026-0009 em `time 0.3.45` (transitivo via `tracing-appender`)
- Status: resolvida em v0.1.7 (2026-06-05)
- Problema: DoS via stack exhaustion em parsing de time
- Correção: upgrade de `time` para 0.3.47 com `DEPTH_LIMIT=32`, e a correção requer Rust 1.88
- Ação tomada: MSRV subido de 1.85 para 1.88 em v0.1.7, a entrada `ignore` no `deny.toml` e a flag `cargo audit --ignore` foram ambas removidas, e a advisory não se aplica mais
- Referência: entrada v0.1.7 no `CHANGELOG.pt-BR.md`

### BUG-SEC-001 — Escape de symlink-directory do jail do workspace (Corrigido em v0.1.27)
- Severidade: crítica
- Versões afetadas: v0.1.0 a v0.1.26
- Corrigido em: v0.1.27
- Vetor de ataque: criar um symlink para diretório fora do workspace (`ln -s /tmp $WS/link`), depois usar atomwrite para escrever/ler pelo symlink (`atomwrite write link/escape.txt`)
- Causa raiz: `validate_path` em `path_safety.rs` usava `soft_canonicalize` que resolvia `.` e `..` mas NÃO symlinks em componentes intermediários do caminho
- Impacto: escrita e leitura arbitrária de arquivos fora do jail do workspace (exfiltração de dados, criação arbitrária de arquivos)
- Subcomandos afetados: write, read, edit, edit-loop, apply, set, del, copy, move
- NÃO afetados: search, replace (usam WalkBuilder do crate `ignore` com verificação própria de symlinks)
- Correção: adicionada `canonicalize_existing_prefix` que resolve symlinks via `std::fs::canonicalize` na porção existente do caminho antes da verificação de jail
- Mitigação: atualize para v0.1.27 e não crie symlinks dentro do workspace apontando para fora

### Auditoria da cadeia de suprimentos da v0.1.37 (zip, quick-xml)
- A auditoria da v0.1.36 não foi reexecutada antes daquela release, e rodá-la na árvore da v0.1.37 achou 7 vulnerabilidades, 2 delas altas (7.5) no `quick-xml` 0.37
- As duas altas eram alcançáveis pelo `extract-artifact` lendo XML de um contêiner OOXML: alocação ilimitada de namespaces (`RUSTSEC-2026-0195`) e checagem quadrática de atributos duplicados (`RUSTSEC-2026-0194`), corrigidas ao subir para o `quick-xml` 0.42
- `zip` 9.0.5 fecha `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` e `RUSTSEC-2026-0369` (uma delas crítica, 9.1) no leitor de contêiner OOXML
- `crossbeam-epoch`, `rustls`, `anyhow` e `memmap2` foram atualizados para as versões corrigidas no mesmo passe
- Lição registrada: rode `cargo audit` e `cargo deny check` ANTES do bump de versão, não depois de escrever a documentação

### Estado atual da árvore da v0.1.37
- `cargo audit` no `Cargo.lock` (434 dependências de crates, 1338 advisories no banco local, que não foi baixado de novo nessa execução): exit 0, nenhuma vulnerabilidade e nenhum aviso, medido em 2026-10-19
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok, medido em 2026-10-19
- Os avisos de crates sem manutenção das auditorias antigas, `instant` (`RUSTSEC-2024-0384`) e `unsafe-libyaml` (`RUSTSEC-2024-0449`), sumiram: nem esses dois nem `serde_yaml` ou `serde_yaml_ng` estão no `Cargo.lock`, e `cargo tree --locked --all-features -i <crate>` responde "did not match any packages" para cada um
- As regras YAML do `transform` são lidas com `serde-saphyr` 1.3.0 e o `notify` está na 8.x (ADR-0057 seções 4 e 5)
- Reexecute as duas ferramentas na árvore final antes de toda release, os números acima são a medição de um dia

### Paths de content-source e jail do workspace (política, não vulnerabilidade)
- Paths de content-source de `--old-file` / `--new-file` permanecem sob o jail do workspace (sem allowlist de `/tmp` do SO)
- Agentes DEVEM estagiar corpos de patch via `atomwrite scratch init .atomwrite/scratch` sob `$workspace/.atomwrite/scratch/` e então passar esses paths seguros no jail
- Um `WORKSPACE_JAIL` (exit 126) em old/new sob `/tmp/...` é política intencional (v0.1.36 / ADR-0055), não um buraco de segurança a contornar


## Postura de Segurança de Dependências (v0.1.37)
- Segurança de memória: `unsafe` é negado em todo o crate (`#![deny(unsafe_code)]` em `src/lib.rs` e `src/main.rs`) e liberado só em `src/signal.rs` e `src/platform.rs` (`#![allow(unsafe_code)]` na linha 5 de cada um), onde 8 blocos `unsafe` envolvem o registro do `signal-hook` e chamadas libc (`_exit`, `poll`, `write` no stderr, `fcntl` no macOS, `renameat2` no Linux, setup do console no Windows)
- Os 8 blocos estão em `src/signal.rs:260`, `src/signal_handlers.inc.rs:31,143,171,219` (incluído por `src/signal.rs`) e `src/platform.rs:97,203,275`, contados com `rg 'unsafe\s*\{' src`, que devolve 9 ocorrências porque `src/commands/scope/queries.inc.rs:77` guarda o texto literal de um padrão de consulta, e cada bloco tem um comentário `SAFETY:`
- BLAKE3: usado apenas para checksums, não para segurança criptográfica
- tree-sitter-language-pack 1.22.1: os parsers são baixados no primeiro uso dos assets de release de `github.com/xberg-io/tree-sitter-language-pack` pela feature `download` (cliente HTTP `ureq` com `rustls` e `ring`, todos no binário padrão)
- O cliente confere cada pacote contra o `sha256` listado no manifesto buscado da mesma release (`download.rs` da crate), e um pacote que falha na conferência é rejeitado
- Os parsers baixados são bibliotecas nativas carregadas no processo (`dynamic-loading`), então rodam como código: mantenha o diretório de cache (`$XDG_CACHE_HOME/tree-sitter-language-pack/`) gravável só pelo seu usuário
- Para rodar sem download, preencha esse cache antes ou compile sem `ast`, veja `PRIVACY.pt-BR.md`
- deny.toml: inclui `MPL-2.0`, `CDLA-Permissive-2.0`, `CC0-1.0` no allowlist, e tem 33 entradas `skip`, cada uma com motivo, para a coexistência inevitável de versões duplicadas de crates (`getrandom` 0.2/0.3, `rustix` 0.x/1.x, `windows-sys` 0.52/0.59 e as crates Windows da feature opt-in `watch`)
- MSRV: Rust 1.90 stable (`rust-version` no `Cargo.toml`, subido de 1.88 na v0.1.37, veja a ADR-0057)


## Hall da Fama
- Pesquisadores de segurança que reportam vulnerabilidades válidas são reconhecidos aqui
- Reporte para ser listado (ou solicitar anonimato)
- Para solicitar listagem, abra uma GitHub Security Advisory com o reporte e inclua sua atribuição preferida


## Melhores Práticas para Usuários
- Use `--workspace` para restringir operações à raiz do projeto
- Evite executar atomwrite como root
- Valide `--expect-checksum` em ambientes multi-agente
- Revise a saída de erro NDJSON para `ok: false`, `retryable` e `suggestion` (parse exit-first)
- Mantenha o atomwrite atualizado para a versão mais recente
- Audite o manifesto `batch` antes da execução em produção
- Inscreva-se em notificações do repositório para atualizações de segurança em tempo hábil
- Trate journals órfãos (sidecars `.atomwrite.journal.*` de um crash anterior) com suspeita: inspecione o arquivo alvo E o conteúdo do journal antes de deletar o journal
- Quando `--syntax-check` estiver habilitado, NÃO faça pipe de conteúdo sensível pelo stdin em sistemas compartilhados: o envelope de erro pode ecoar a localização da fonte e contexto ao redor
- Estagie patches de agente com `scratch init` + `--old-file`/`--new-file` sob o workspace, e não espere que `/tmp` do SO fique isento do jail
- Um relatório de falha (`report-<uuid>.toml` no diretório temporário do SO, gravado pelo hook `human-panic` e nunca transmitido) pode guardar caminhos e um backtrace, então revise-o antes de anexar a uma issue
