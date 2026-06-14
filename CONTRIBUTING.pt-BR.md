[Read in English](CONTRIBUTING.md)


# Contribuindo com o atomwrite


## Bem-vindo
- Obrigado por considerar uma contribuição para o atomwrite
- Toda contribuição importa: código, testes, docs, bug reports, ideias de feature
- Este guia ajuda você a começar rapidamente


## Início Rápido
- Faça fork do repositório no GitHub
- Clone seu fork localmente
- Crie uma feature branch a partir de `main`
- Faça suas mudanças
- Rode a suite de testes
- Abra um pull request


## Setup de Desenvolvimento
### Pré-requisitos
- Rust 1.88 ou posterior (edition 2024) — MSRV bumped em v0.1.7
- Git

### Build
```bash
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo build
```

### Rodar Testes
```bash
cargo test
```

### Rodar Linter
```bash
cargo clippy -- -D warnings
```

### Verificar Formatação
```bash
cargo fmt -- --check
```


## Branching
- Crie branches a partir de `main`
- Use nomes descritivos: `feat/batch-parallel`, `fix/checksum-race`, `docs/readme-update`
- Mantenha branches curtas e focadas em uma única mudança


## Convenção de Commit
- Use imperativo no presente: "add batch support", não "added batch support"
- Mantenha a primeira linha abaixo de 72 caracteres
- Referencie números de issues quando aplicável: `fix search exit code (#42)`
- Uma mudança lógica por commit


## Processo de PR
- Preencha o template de PR com uma descrição clara
- Link issues relacionadas
- Garanta que todos os checks de CI passam antes de pedir review
- Mantenha PRs focados: uma feature ou fix por PR
- Responda ao feedback de review prontamente
- Squash commits quando pedido pelos maintainers


## Testes
- Escreva testes para toda nova feature e bug fix
- Coloque testes unitários no mesmo arquivo do código sob `#[cfg(test)]`
- Coloque testes de integração no diretório `tests/`
- Use `assert_cmd` e `predicates` para testes de integração de CLI
- Use `insta` para testes de snapshot de saída NDJSON
- Use `proptest` para testes property-based onde aplicável
- Mire em pelo menos 80% de cobertura para código novo
- Rode a suite completa antes de submeter: `cargo test` (461 testes em v0.1.15)


## Documentação
- Atualize o README ao adicionar ou mudar comandos
- Atualize AGENTS.md ao modificar o contrato de saída ou códigos de saída
- Atualize CHANGELOG.md (inglês) e CHANGELOG.pt-BR.md (português) para qualquer mudança visível ao usuário
- Para decisões arquiteturais não-triviais, adicione um ADR em `docs/decisions/` seguindo o formato Michael Nygard (Status, Context, Decision, Consequences, Alternatives, Trigger to revisit). Veja `docs/decisions/README.md` para o índice
- Para novos envelopes de saída NDJSON, adicione um JSON Schema em `docs/schemas/` (versionado por release)
- Adicione doc comments em todas as funções e tipos públicos
- Mantenha exemplos de código nos docs testados e atualizados


## Architecture Decision Records (ADRs)
- atomwrite usa ADRs em `docs/decisions/` para documentar escolhas de design não-triviais
- 7 ADRs foram adicionados em v0.1.12 (0019-0025), todos seguindo o formato Michael Nygard
- Cada nova decisão arquitetural deve adicionar um novo arquivo ADR e atualizar `docs/decisions/README.md`
- ADRs NÃO são atualizados uma vez escritos — ao invés disso, sobrescreva com um novo ADR


## Adicionando um Novo Subcomando
- Adicione um módulo em `src/commands/seu_subcomando.rs`
- Registre o subcomando em `src/commands/mod.rs`
- Defina a struct de argumentos em `src/cli_args.rs` com derives de `clap`
- Adicione o braço de dispatch em `src/lib.rs`
- Adicione uma entrada no enum `Commands` em `src/cli.rs`
- Adicione um JSON Schema correspondente em `docs/schemas/`
- Adicione o subcomando aos inventários de README e llms.txt
- Escreva pelo menos 3 testes de integração em `tests/cli_seu_subcomando.rs`
- Atualize llms-full.txt para referenciar o novo subcomando na categoria correta
- v0.1.12 tem 28 subcomandos; a contagem deve ficar em sincronia entre todos os docs


## Reportar Bugs
- Abra uma issue no GitHub com o label `bug`
- Inclua: versão do atomwrite, SO, versão do Rust, passos para reproduzir, comportamento esperado vs atual
- Inclua a saída NDJSON completa do erro quando aplicável
- Casos de reprodução mínimos são muito apreciados


## Pedir Features
- Abra uma issue no GitHub com o label `enhancement`
- Descreva o problema que a feature resolve
- Descreva o comportamento esperado
- Considere como ela se encaixa no contrato de saída NDJSON


## Processo de Release
- Maintainers lidam com releases
- Versão segue Semantic Versioning 2.0.0
- Changelog atualizado antes de cada release (EN e PT-BR)
- Tags seguem o formato `vX.Y.Z`
- Publicado no crates.io após CI passar
- v0.1.12 foi publicada em 2026-06-07 com commit 6af0d76


## Reconhecimento
- Contribuidores são reconhecidos no changelog e notas de release
- Contribuições significativas são reconhecidas no repositório


## Quality Gates
- Rode `cargo fmt --check` antes de commitar
- Rode `cargo clippy --all-targets -- -D warnings` para checagens de lint
- Rode `cargo test` para a suite completa (461 testes em v0.1.15)
- Rode `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` para checagens de documentação
- Rode `cargo audit` para advisories de segurança
- Rode `cargo deny check` para política de licenças e dependências (veja `deny.toml`)
- Rode `cargo check --all-features` contra o MSRV (Rust 1.88) para compatibilidade de toolchain
- Rode `cargo package --no-verify --list` e `cargo publish --dry-run --allow-dirty` para validar artefatos de release

## Validação Cross-Platform (adicionado em v0.1.4)
- Instale targets Windows: `rustup target add x86_64-pc-windows-gnu` e `i686-pc-windows-gnu`
- Em Linux, instale mingw: `mingw64-gcc` (Fedora) ou `mingw-w64` (Ubuntu)
- Rode o gate de cross-compile: `cargo test --test cross_compile_check -- --ignored`
- O gate falha em qualquer regressão `E0433`, `E0308` ou `E0507` em blocos `#[cfg(windows)]`
- Necessário para qualquer mudança que toque `src/atomic.rs`, `src/platform.rs`, `src/signal.rs` ou outro código Windows-only

## Gates Específicos da v0.1.12
- Se você adicionar um novo subcomando, atualize a contagem em TODOS os: `README.md`, `README.pt-BR.md`, `llms.txt`, `llms.pt-BR.txt`, `llms-full.txt`, `docs/AGENTS.md`, `docs/AGENTS.pt-BR.md`, `docs/MIGRATION.md`, `docs/MIGRATION.pt-BR.md`, `CHANGELOG.md`, `CHANGELOG.pt-BR.md`, `skill/atomwrite-en/SKILL.md`, `skill/atomwrite-pt/SKILL.md`
- Se você adicionar uma nova variante de erro, atualize os códigos de saída em: `README.md`, `README.pt-BR.md`, `llms-full.txt`, `docs/AGENTS.md`, `docs/AGENTS.pt-BR.md`, `skill/atomwrite-en/SKILL.md`, `skill/atomwrite-pt/SKILL.md`, `locales/en.toml`, `locales/pt-BR.toml`
- A fonte única de verdade para a contagem de subcomandos é o binário: `atomwrite --help | rg "^  [a-z]" | wc -l` (atualmente 29 em v0.1.12 = 28 user-facing + `help`)


## Perguntas
- Abra uma GitHub Discussion para perguntas gerais
- Abra uma issue para bugs e pedidos de feature
- Seja respeitoso e construtivo em todas as interações
