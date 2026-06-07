[Read in English](INTEGRATIONS.md)


# Integrações

> atomwrite funciona com todo agente LLM que pode executar comandos shell


## Agentes Compatíveis (v0.1.12)
- atomwrite requer apenas acesso a `bash` para funcionar
- Qualquer agente que pode executar um comando shell pode usar atomwrite
- Saída NDJSON é parseável por todo LLM principal sem adaptadores customizados
- Sem plugins, extensões ou SDKs necessários
- **28 subcomandos** a partir da v0.1.12 (6 novos: set, get, del, case, query, outline)
- A partir da v0.1.12, atomwrite roda em Windows 10/11, Linux e macOS com contrato NDJSON idêntico
- A release v0.1.12 adicionou 5 novas variantes de erro e 445 testes em 43 suites


## Tabela Resumida

| Agente | Acesso a Shell | Parsing NDJSON | Esforço de Integração |
|--------|----------------|----------------|-----------------------|
| Claude Code | Nativo | Nativo | Zero config |
| Cursor | Nativo | Nativo | Zero config |
| Windsurf | Nativo | Nativo | Zero config |
| Codex CLI | Nativo | Nativo | Zero config |
| Aider | Nativo | Nativo | Zero config |
| Continue | Nativo | Nativo | Zero config |
| Cline | Nativo | Nativo | Zero config |
| Roo Code | Nativo | Nativo | Zero config |
| Amazon Q Developer | Nativo | Nativo | Zero config |
| GitHub Copilot CLI | Nativo | Nativo | Zero config |
| Agentes Customizados | Via subprocess | Via parser JSON | Um `cargo install` |


## Claude Code
- Instalação: `cargo install atomwrite`
- Claude Code executa comandos bash nativamente
- Saída NDJSON é parseada diretamente sem adaptadores
- Use `--workspace` para casar com a raiz do projeto
- Combine com `--expect-checksum` para edições concorrentes seguras
- Adicione comandos do atomwrite ao CLAUDE.md para descoberta automática
- v0.1.12: use `set/get/del/case/query/outline` para configuração estruturada e análise AST
```bash
# Exemplo de entrada em CLAUDE.md
# Use atomwrite para todas as operações de arquivo
echo "content" | atomwrite --workspace . write src/file.rs
atomwrite --workspace . read src/file.rs
atomwrite --workspace . search 'pattern' src/
# v0.1.12: caminhar o AST de um arquivo Rust
atomwrite --workspace . query src/main.rs --kinds
```


## Cursor
- Instalação: `cargo install atomwrite`
- Cursor executa comandos de terminal via seu shell embutido
- Respostas NDJSON integram diretamente com fluxos de tool-use
- Use `--dry-run` para preview antes de operações destrutivas
```bash
atomwrite --workspace . search 'TODO' src/ --include '*.rs'
atomwrite --workspace . replace 'old_api' 'new_api' src/
```


## Windsurf
- Instalação: `cargo install atomwrite`
- Windsurf roda comandos shell via sua integração de terminal
- Saída estruturada reduz consumo de tokens comparado com CLIs raw
- Operações em lote minimizam o número de tool calls
```bash
cat manifest.ndjson | atomwrite --workspace . batch
```


## Codex CLI
- Instalação: `cargo install atomwrite`
- Codex CLI executa comandos em um shell sandboxed
- atomwrite respeita fronteiras de sandbox via `--workspace`
- Checksums habilitam verificação de estado entre passos de execução
- v0.1.12: use `case` para refatorar identificadores em múltiplos arquivos
```bash
atomwrite --workspace . read src/main.rs
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . case src/ --subvert user_id UserId --to pascal
```


## Aider
- Instalação: `cargo install atomwrite`
- Aider roda comandos shell para workflows de edição de código
- atomwrite fornece garantias atômicas que built-ins do shell não têm
- Use `edit` para mudanças cirúrgicas em vez de reescritas completas de arquivo
- v0.1.12: use `outline` para dar ao Aider um mapa do codebase antes das edições
```bash
atomwrite --workspace . edit src/lib.rs --old "código antigo" --new "código atualizado"
atomwrite --workspace . outline src/  # veja a estrutura primeiro
```


## Continue
- Instalação: `cargo install atomwrite`
- Continue executa comandos de terminal como parte de seu loop de agente
- Saída NDJSON é legível por máquina sem pós-processamento
```bash
atomwrite --workspace . search 'deprecated' src/ --include '*.rs'
```


## Cline
- Instalação: `cargo install atomwrite`
- Cline usa comandos shell para operações de arquivo
- atomwrite substitui pipelines frágeis de sed/awk por operações atômicas
```bash
echo "novo conteudo" | atomwrite --workspace . write src/config.rs
atomwrite --workspace . diff src/old.rs src/new.rs
```


## Roo Code
- Instalação: `cargo install atomwrite`
- Roo Code roda comandos bash em seu ambiente de execução de agente
- Erros estruturados com campos `retryable` e `suggestion` guiam recuperação automática
```bash
atomwrite --workspace . copy src/template.rs src/new.rs
```


## Amazon Q Developer
- Instalação: `cargo install atomwrite`
- Amazon Q executa comandos CLI em seu ambiente de desenvolvimento
- atomwrite fornece comportamento cross-platform consistente
```bash
atomwrite --workspace . list src/ --depth 3
atomwrite --workspace . count src/ --by-extension
```


## GitHub Copilot CLI
- Instalação: `cargo install atomwrite`
- Copilot CLI sugere e executa comandos shell
- Comandos do atomwrite são auto-documentados via `--help` em cada subcomando
- v0.1.12: use `get` para ler metadados de pacote sem escrever um parser
```bash
atomwrite calc "100 MB to bytes"
atomwrite regex "192.168.1.1" "10.0.0.1" "172.16.0.1"
atomwrite --workspace . get Cargo.toml package.version
```


## Agentes Customizados
- Instalação: `cargo install atomwrite`
- Invoque via `std::process::Command`, `subprocess.run()`, ou equivalente
- Parseie stdout linha por linha como objetos JSON
- Verifique códigos de saída para classificação de erro
- Use o campo `retryable` nas respostas de erro para lógica de retry automática
- v0.1.12: trate 5 novos códigos de saída (83, 88, 91, 92, 93) para timeout de lock, erro de sintaxe, EXDEV desabilitado, copy-back BLAKE3 falhou, journal órfão
```python
import subprocess
import json

result = subprocess.run(
    ["atomwrite", "--workspace", ".", "read", "src/main.rs"],
    capture_output=True, text=True
)
for line in result.stdout.strip().split("\n"):
    data = json.loads(line)
    print(data["type"], data.get("path"))
```
