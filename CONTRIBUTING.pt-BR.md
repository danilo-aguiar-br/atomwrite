[Read in English](CONTRIBUTING.md)


# Contribuindo com o atomwrite


## Bem-vindo
- Obrigado por considerar uma contribuição ao atomwrite
- Toda contribuição importa: código, testes, docs, relatos de bugs, ideias de features
- Este guia ajuda você a começar rapidamente


## Início Rápido
- Faça um fork do repositório no GitHub
- Clone seu fork localmente
- Crie uma branch de feature a partir da `main`
- Faça suas mudanças
- Execute a suite de testes
- Abra um pull request


## Setup de Desenvolvimento
### Pré-requisitos
- Rust 1.85 ou superior (edition 2024)
- Git

### Build
```bash
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo build
```

### Executar Testes
```bash
cargo test
```

### Executar Linter
```bash
cargo clippy -- -D warnings
```

### Verificar Formatação
```bash
cargo fmt -- --check
```


## Branches
- Crie branches a partir da `main`
- Use nomes descritivos: `feat/batch-parallel`, `fix/checksum-race`, `docs/readme-update`
- Mantenha branches curtas e focadas em uma única mudança


## Convenção de Commits
- Use presente no modo imperativo: "add batch support", não "added batch support"
- Mantenha a primeira linha abaixo de 72 caracteres
- Referencie números de issue quando aplicável: `fix search exit code (#42)`
- Uma mudança lógica por commit


## Processo de PR
- Preencha o template de PR com uma descrição clara
- Vincule issues relacionadas
- Garanta que todos os checks de CI passam antes de solicitar review
- Mantenha PRs focados: uma feature ou fix por PR
- Responda ao feedback de review prontamente
- Faça squash de commits quando solicitado pelos mantenedores


## Testes
- Escreva testes para toda nova feature e bug fix
- Coloque testes unitários no mesmo arquivo do código sob `#[cfg(test)]`
- Coloque testes de integração no diretório `tests/`
- Use `assert_cmd` e `predicates` para testes de integração CLI
- Use `insta` para testes de snapshot da saída NDJSON
- Use `proptest` para testes baseados em propriedade quando aplicável
- Mire em pelo menos 80% de cobertura para código novo
- Execute a suite completa antes de submeter: `cargo test`


## Documentação
- Atualize o README ao adicionar ou alterar comandos
- Atualize AGENTS.md ao modificar o contrato de saída ou códigos de saída
- Adicione doc comments a todas as funções e tipos públicos
- Mantenha exemplos de código na documentação testados e atualizados


## Reportar Bugs
- Abra uma issue no GitHub com o label `bug`
- Inclua: versão do atomwrite, SO, versão do Rust, passos para reproduzir, comportamento esperado vs real
- Inclua a saída NDJSON completa do erro quando aplicável
- Casos de reprodução mínimos são muito apreciados


## Solicitar Features
- Abra uma issue no GitHub com o label `enhancement`
- Descreva o problema que a feature resolve
- Descreva o comportamento esperado
- Considere como se encaixa no contrato de saída NDJSON


## Processo de Release
- Mantenedores cuidam dos releases
- Versionamento segue Semantic Versioning 2.0.0
- Changelog atualizado antes de cada release
- Tags seguem o formato `vX.Y.Z`
- Publicado no crates.io após CI passar


## Reconhecimento
- Contribuidores são reconhecidos no changelog e notas de release
- Contribuições significativas são destacadas no repositório


## Portões de Qualidade
- Execute `cargo fmt --check` antes de commitar
- Execute `cargo clippy --all-targets -- -D warnings` para verificação de lint
- Execute `cargo test` para a suíte completa de testes
- Execute `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` para verificação de documentação
- Execute `cargo audit` para alertas de segurança
- Execute `cargo deny check` para política de licenças e dependências (veja `deny.toml`)
- Execute `cargo check --all-features` contra o MSRV (Rust 1.85) para compatibilidade de toolchain
- Execute `cargo package --no-verify --list` e `cargo publish --dry-run --allow-dirty` para validar artefatos de release

## Validação Cross-Platform (adicionado na v0.1.4)
- Instale targets Windows: `rustup target add x86_64-pc-windows-gnu` e `i686-pc-windows-gnu`
- No Linux, instale mingw: `mingw64-gcc` (Fedora) ou `mingw-w64` (Ubuntu)
- Execute o gate de cross-compile: `cargo test --test cross_compile_check -- --ignored`
- O gate falha em qualquer regressão de `E0433`, `E0308`, ou `E0507` em blocos `#[cfg(windows)]`
- Obrigatório para qualquer mudança que toque `src/atomic.rs`, `src/platform.rs`, `src/signal.rs`, ou outro código Windows-only
- O gate é uma defesa contra a regressão do GAP 14: `cargo install atomwrite` estava quebrado no Windows 10/11 na v0.1.3 porque três erros de compilação Windows-only não foram capturados pelo CI Linux-only


## Dúvidas
- Abra uma GitHub Discussion para perguntas gerais
- Abra uma issue para bugs e solicitações de features
- Seja respeitoso e construtivo em todas as interações
