[Read in English](CONTRIBUTING.md)


# Contribuindo com o atomwrite

> Contribua com a CLI atômica em que agentes confiam para edições duráveis


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
- Instale Rust 1.88 ou posterior (edition 2024; MSRV declarado em `Cargo.toml`)
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

### Definição de pronto local
- Este produto CLI não usa workflows de GitHub Actions do produto como gate de release
- Valide localmente antes de cada PR e release
- Rode testes de biblioteca e integração: `cargo test --lib --tests`
- Rode clippy com warnings negados: `cargo clippy --all-targets -- -D warnings`
- Verifique cross-compile Windows ao tocar código de plataforma: `cargo check --target x86_64-pc-windows-gnu`
- Instale a partir do path após mudanças substantivas: `cargo install --path . --force`
- Suite opcional de contrato residual de agente: `cargo test --test cli_e2e_v0135`
- Suite opcional de contrato de agente v0.1.36: `cargo test --test cli_v0136_gaps`
- SSOT opcional de gaps: `bash scripts/check-gaps-ssot.sh`
- Superfície slim opcional: `cargo test --no-default-features --features core`
- Checagem opcional de formatação: `cargo fmt -- --check`
- Lint opcional de docs: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- Checagens opcionais de supply-chain: `cargo audit` e `cargo deny check`

### Pin da release atual
- A versão pública atual é `0.1.36`
- Fixe consumidores e agentes em `^0.1.36`
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


## Documentação
- Mantenha o inglês como idioma canônico da documentação pública
- Espelhe cada mudança de doc pública no arquivo `.pt-BR` correspondente na mesma entrega
- Abra docs públicos com link cruzado para o idioma oposto
- Atualize `README.md` e `README.pt-BR.md` quando comandos ou orientação de instalação mudarem
- Atualize `docs/AGENTS.md` e `docs/AGENTS.pt-BR.md` quando o contrato de agente mudar
- Atualize `CHANGELOG.md` e `CHANGELOG.pt-BR.md` para toda mudança visível ao usuário
- Siga Keep a Changelog e Semantic Versioning
- Atualize skills em `skills/atomwrite-en/` e `skills/atomwrite-pt/` quando contratos operacionais mudarem
- Adicione ou atualize JSON schemas em `docs/schemas/` para novos envelopes NDJSON
- Adicione um ADR em `docs/decisions/` para escolhas de design não triviais
- Atualize `llms.txt`, `llms.pt-BR.txt` e `llms-full.txt` quando a documentação principal mudar
- Quando a superfície da CLI mudar: atualize `CANONICAL_SUBCOMMAND_COUNT`, inventários de **45** em README/AGENTS/`llms*`, `docs/schemas/README.md` e rode `scripts/check-gaps-ssot.sh`
- Quando gaps de contrato de agente fecharem: atualize status em `gaps.md`, ADR, HOW_TO_USE, COOKBOOK, MIGRATION, TESTING e as duas skills na mesma entrega
- Nota: `gaps.md` são notas locais de auditoria e não fazem parte da documentação pública de empacotamento
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
- Valide com a DoD local; este produto não depende de GitHub Actions do produto como gate de release
- Checagens sugeridas pré-publicação:
  - `cargo test --lib --tests`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo check --target x86_64-pc-windows-gnu`
  - `cargo package --list` / dry-run de publish conforme necessário
  - `cargo install --path . --force`
- Publique no crates.io somente após a DoD local estar verde
- A linha de release atual é `0.1.36`; consumidores devem fixar `^0.1.36`


## Reconhecimento
- Contribuidores são creditados em entradas de changelog e notas de release quando apropriado
- Contribuições significativas podem ser reconhecidas na documentação do repositório
- Pesquisadores de segurança são creditados conforme [SECURITY.pt-BR.md](SECURITY.pt-BR.md)


## Perguntas
- Abra uma GitHub Discussion para perguntas gerais quando disponível
- Abra uma issue para bugs concretos e pedidos de feature
- Contate o maintainer em daniloaguiarbr@proton.me para perguntas privadas de contribuição
- Seja respeitoso e construtivo em toda interação
