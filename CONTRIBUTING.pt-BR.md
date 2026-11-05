[Read in English](CONTRIBUTING.md)


# Contribuindo com o atomwrite


- Contribua com a CLI atômica em que agentes confiam para edições duráveis


## Bem-vindo
- Obrigado por considerar uma contribuição para o atomwrite
- atomwrite é uma CLI Rust para operações atômicas de arquivo e contratos NDJSON para agentes
- Toda contribuição importa: código, testes, docs, relatos de bugs e ideias de feature
- Leia este guia antes de abrir um pull request
- Siga o [Código de Conduta](CODE_OF_CONDUCT.pt-BR.md) em toda interação


## Início Rápido
- Faça fork do repositório no GitHub
- Clone seu fork localmente
- Crie uma feature branch a partir de `main`
- Faça uma mudança focada
- Execute os comandos da definição de pronto (DoD) local abaixo
- Abra um pull request contra `main`


## Setup de Desenvolvimento
### Pré-requisitos
- Instale Rust 1.90 ou posterior (edition 2024; MSRV declarado em `Cargo.toml` como `rust-version = "1.90"`)
- O `rust-toolchain.toml` local seleciona o canal 1.94 com `rustfmt` e `clippy`, e o MSRV continua 1.90
- Instale Git
- Instale `rustfmt` e `clippy` via a toolchain ativa (`rust-toolchain.toml`)

### Build
```bash
git clone https://github.com/danilo-aguiar-br/atomwrite.git
cd atomwrite
cargo build
# somente core slim
cargo build --release --no-default-features --features core
# superfície completa
cargo build --release --features full
```

### Política de MSRV
- O MSRV é o campo `rust-version` do `Cargo.toml` (1.90 hoje, subido de 1.89 porque o `ast-grep-core` 0.51 e o `tree-sitter` 0.27 exigem, veja a ADR-0075)
- Subir o MSRV é mudança de versão minor, nunca patch, e o CHANGELOG a lista
- Mantenha `README.md`, `docs/INSTALL.md` e `docs/CROSS_PLATFORM.md` em sincronia com o `Cargo.toml` quando ele mudar

### Definição de pronto local
- Todo gate roda localmente na sua máquina, não existe automação remota
- Valide localmente antes de cada PR e release
- Rode testes de biblioteca e integração com as features padrão: `cargo test --lib --tests`
- Rode clippy com warnings negados: `cargo clippy --all-targets -- -D warnings`
- Verifique cross-compile Windows ao tocar código de plataforma: `cargo check --target x86_64-pc-windows-gnu`
- Instale a partir do path após mudanças substantivas: `cargo install --path . --force`
- Suite opcional de contrato residual de agente: `cargo test --test cli_e2e_v0135`
- Suite opcional de contrato de agente v0.1.36: `cargo test --test cli_v0136_gaps`
- Suites opcionais de contrato de agente v0.1.37: `cargo test --test cli_v0137_envelope --test cli_v0137_dominio --test cli_v0137_agent_native --test cli_v0137_artefato --test cli_v0137_etd --test cli_v0137_cancel`
- O status de um gap vive no ADR que o possui; nota local de auditoria nunca é portão de release
- Perfil slim: `cargo test --no-default-features --features core` fica verde em 131 alvos de teste, porque todo teste que exercita `scope`, `transform`, `outline` ou `query` carrega `#[cfg(feature = "ast")]` e é pulado nesse build
- Para provar que o build slim compila, rode `cargo xtask check-features` (roda `cargo check --locked --no-default-features` nos conjuntos `core` e `core,watch` e para na primeira falha), e rode a suíte de testes com as features padrão
- Suites opcionais da v0.1.37 para o verbo `config` e os guardas de código-fonte: `cargo test --test cli_v0137_config_verb --test cli_v0137_hardcode_guard --test cli_v0137_retry_guards --test cli_v0137_let_underscore_guard --test cli_v0137_terminal_event`
- Portões de documentação: `cargo test --test docs_coverage --test docs_consistency --test docs_exit_codes --test docs_schemas_sync`, onde `docs_coverage` exige todo subcomando em cada documento principal e `docs_schemas_sync` mantém os arquivos de schema e o binário em passo
- Checagem de formatação: `cargo fmt -- --check`
- Lint de docs: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- Checagens de supply-chain, rode antes do bump de versão e não depois de escrever a documentação: `cargo audit` e `cargo deny check`

### Pin da release atual
- A linha de release desta árvore é `0.1.38`, que ainda está em preparação, e o crates.io carrega a `0.1.37` até a `0.1.38` ser publicada
- Fixe consumidores e agentes em `^0.1.37` hoje e em `^0.1.38` quando essa versão sair
- Mantenha strings de versão consistentes em `Cargo.toml`, README, CHANGELOG e skills


## Estratégia de Branches
- Crie branches a partir de `main`
- Prefira branches de vida curta e propósito único
- Nomeie branches com intenção: `feat/watch-summary`, `fix/ack-overwrite`, `docs/contributing`
- Evite misturar refactors não relacionados com o trabalho da feature


## Convenção de Commit
- Escreva o assunto no imperativo do presente: `add watch_summary on idle`
- Mantenha a primeira linha abaixo de 72 caracteres
- Referencie issues relacionadas quando aplicável: `fix fuzzy one-pass hang (#123)`
- Prefira uma mudança lógica por commit quando prático
- Prefira prefixos Conventional Commits quando úteis: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`


## Processo de PR
- Descreva o problema, a mudança e como você validou
- Linke issues relacionadas
- Mantenha cada PR focado em uma feature, fix ou mudança de docs
- Atualize a documentação bilíngue no mesmo PR quando o comportamento visível ao usuário mudar
- Rode a DoD local antes de pedir review
- Responda ao feedback de review com prontidão
- Faça squash dos commits quando os maintainers pedirem

### Checklist do pull request
- [ ] `cargo fmt -- --check` passa
- [ ] `cargo clippy --all-targets -- -D warnings` passa
- [ ] `cargo test --lib --tests` passa com as features padrão
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` passa
- [ ] `cargo audit` e `cargo deny check` passam
- [ ] README, CHANGELOG e os dois espelhos `.pt-BR` estão atualizados
- [ ] Os schemas NDJSON e a ADR estão atualizados quando o contrato muda
- [ ] Nenhum número não medido foi adicionado à documentação


## Testes
- Adicione testes para toda nova feature e bug fix
- Coloque testes unitários ao lado do código sob `#[cfg(test)]` quando apropriado
- Coloque testes de integração de CLI em `tests/`
- Prefira `assert_cmd` e `predicates` para asserções de CLI
- Prefira `insta` para contratos de snapshot NDJSON
- Prefira `proptest` para cobertura property-based quando couber
- Rode `cargo test --lib --tests` antes de submeter
- Rode suites direcionadas à superfície que você tocou, por exemplo:
  - `cargo test --test cli_e2e_v0135`
  - `cargo test --test cli_v0130_agent_contract`
  - `cargo test --test cli_v0133_oneshot_fuzzy`
- Veja [docs/TESTING.pt-BR.md](docs/TESTING.pt-BR.md) para categorias e perfis
- Não invente nem cole contagens de testes não medidas na documentação

### Testes herméticos e guardas de código-fonte
- Todo teste que roda o binário passa por `common::hermetic(...)` (`tests/common/mod.rs`), que dá ao filho `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME` e `APPDATA` vazios e fixa `LANG` em `en_US.UTF-8`
- O cache XDG continua o real de propósito, para os parsers AST não serem baixados de novo a cada teste
- Um teste que precisa da própria config redireciona `HOME` e `XDG_*` depois de chamar `hermetic`
- Os guardas de código-fonte quebram o build: `tests/cli_v0137_hardcode_guard.rs` proíbe literais de duração, o infixo `.bak.`, nomes de sidecar e números de unidade de tempo fora de `constants*`, além de `env::var` e `Command::new` em qualquer código de produção, e proíbe qualquer arquivo `.sh` e qualquer diretório `.github`
- `tests/cli_v0137_let_underscore_guard.rs` proíbe `let _ =`, `.unwrap()`, `.expect(`, `unreachable!`, `panic!`, `todo!` e `unimplemented!` em código de produção
- Os dois guardas mantêm as listas de exceção vazias, então código novo deve cumpri-los em vez de adicionar uma exceção


## Documentação
- Mantenha o inglês como idioma canônico da documentação pública
- Espelhe cada mudança de doc pública no arquivo `.pt-BR` correspondente na mesma entrega
- Abra docs públicos com link cruzado para o idioma oposto
- Atualize `README.md` e `README.pt-BR.md` quando comandos ou orientação de instalação mudarem
- Atualize `docs/AGENTS.md` e `docs/AGENTS.pt-BR.md` quando o contrato de agente mudar
- Atualize `CHANGELOG.md` e `CHANGELOG.pt-BR.md` para toda mudança visível ao usuário
- Traduções: o pt-BR é a única tradução, escrita com todos os acentos, e as strings da interface vivem em `locales/en.toml` e `locales/pt-BR.toml` com paridade de chaves garantida por `tests/locale_parity.rs`
- Siga Keep a Changelog e Semantic Versioning
- Atualize skills em `skills/atomwrite-en/` e `skills/atomwrite-pt/` quando contratos operacionais mudarem
- Adicione ou atualize JSON schemas em `docs/schemas/` para novos envelopes NDJSON
- Adicione um ADR em `docs/decisions/` para escolhas de design não triviais
- Atualize `llms.txt`, `llms.pt-BR.txt` e `llms-full.txt` quando a documentação principal mudar
- Quando a superfície da CLI mudar: atualize `CANONICAL_SUBCOMMAND_COUNT`, os inventários de 47 em README/AGENTS/`llms*` e `docs/schemas/README.md`
- Quando gaps de contrato de agente fecharem: atualize o status no ADR que possui o gap, HOW_TO_USE, COOKBOOK, MIGRATION, TESTING e as duas skills na mesma entrega
- Nota: o inventário de gaps é nota local não versionada, então o status corrente vive na série de ADRs e nunca nessa nota
- Exclua notas privadas como `docs_rules/`, `gaps.md` e memória local de agentes das expectativas de empacotamento de release


## Reportar Bugs
- Abra uma issue no GitHub com reprodução clara
- Inclua versão do atomwrite, SO, versão do Rust, comando exato, resultado esperado e resultado atual
- Anexe a saída NDJSON completa do erro quando disponível
- Prefira uma reprodução mínima a um dump grande de workspace
- Reporte issues de segurança em privado via [SECURITY.pt-BR.md](SECURITY.pt-BR.md), não em issues públicas


## Pedir Features
- Abra uma issue no GitHub descrevendo primeiro o problema do usuário
- Explique o comportamento esperado da CLI e o impacto no contrato NDJSON
- Considere códigos de saída, segurança de workspace, garantias de escrita atômica e docs bilíngues
- Prefira features que mantenham o atomwrite como CLI one-shot não interativa


## Processo de Release
- Maintainers são donos das releases
- O versionamento segue Semantic Versioning 2.0.0
- Entradas de changelog seguem Keep a Changelog em EN e pt-BR
- Tags usam o formato `vX.Y.Z`
- Valide com a DoD local, todo gate roda localmente
- Checagens sugeridas pré-publicação:
  - `cargo test --lib --tests`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo check --target x86_64-pc-windows-gnu`
  - `cargo xtask check-features`
  - `cargo audit` e `cargo deny check`
  - `cargo package --list` / dry-run de publish conforme necessário
  - `cargo install --path . --force`
- Publique no crates.io somente após a DoD local estar verde, e o crates.io continua carregando a `0.1.37` até o publish da `0.1.38` acontecer
- A linha de release atual é `0.1.38`; consumidores devem fixar `^0.1.37` hoje e `^0.1.38` quando ela for publicada


## Código de Conduta
- Todo contribuidor segue o [Código de Conduta](CODE_OF_CONDUCT.pt-BR.md)
- Reporte violações como descrito na seção Aplicação desse documento


## Reconhecimento
- Contribuidores são creditados em entradas de changelog e notas de release quando apropriado
- Contribuições significativas podem ser reconhecidas na documentação do repositório
- Pesquisadores de segurança são creditados conforme [SECURITY.pt-BR.md](SECURITY.pt-BR.md)


## Perguntas
- Abra uma GitHub Discussion para perguntas gerais quando disponível
- Abra uma issue para bugs concretos e pedidos de feature
- Contate o maintainer em daniloaguiarbr@proton.me para perguntas privadas de contribuição
- Seja respeitoso e construtivo em toda interação
