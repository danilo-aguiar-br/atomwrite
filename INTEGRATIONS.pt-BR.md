[Read in English](INTEGRATIONS.md)


# Integrações

> atomwrite funciona com todo agente LLM que pode executar comandos shell


## Agentes Compatíveis
- atomwrite requer apenas acesso `bash` para funcionar
- Qualquer agente que pode rodar um comando shell pode usar atomwrite
- Saída NDJSON é parseável por todo LLM principal sem adaptadores customizados
- Nenhum plugin, extensão ou SDK necessário
- A partir da v0.1.4, atomwrite roda em Windows 10/11, Linux, e macOS com contrato NDJSON idêntico


## Tabela Resumo

| Agente | Acesso Shell | Parsing NDJSON | Esforço de Integração |
|--------|-------------|----------------|----------------------|
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
| Agentes Customizados | Via subprocess | Via JSON parser | Um `cargo install` |


## Claude Code
- Instalar: `cargo install atomwrite`
- Claude Code executa comandos bash nativamente
- Saída NDJSON é parseada diretamente sem adaptadores
- Use `--workspace` para corresponder a raiz do projeto
- Combine com `--expect-checksum` para edições concorrentes seguras
- Adicione comandos atomwrite ao CLAUDE.md para descoberta automática
```bash
# Exemplo de entrada no CLAUDE.md
# Use atomwrite para todas as operações de arquivo
echo "content" | atomwrite write src/file.rs
atomwrite read src/file.rs
atomwrite search 'pattern' src/
```


## Cursor
- Instalar: `cargo install atomwrite`
- Cursor executa comandos de terminal via seu shell integrado
- Respostas NDJSON integram diretamente com fluxos de tool-use
- Use `--dry-run` para preview antes de operações destrutivas
```bash
atomwrite search 'TODO' src/ --include '*.rs'
atomwrite replace 'old_api' 'new_api' src/
```


## Windsurf
- Instalar: `cargo install atomwrite`
- Windsurf roda comandos shell através de sua integração de terminal
- Saída estruturada reduz consumo de tokens comparado a ferramentas CLI brutas
- Operações em lote minimizam o número de chamadas de ferramenta
```bash
cat manifest.ndjson | atomwrite batch
```


## Codex CLI
- Instalar: `cargo install atomwrite`
- Codex CLI executa comandos em um shell sandboxed
- atomwrite respeita limites do sandbox via `--workspace`
- Checksums habilitam verificação de estado entre etapas de execução
```bash
atomwrite read src/main.rs
atomwrite hash src/main.rs
```


## Aider
- Instalar: `cargo install atomwrite`
- Aider roda comandos shell para fluxos de edição de código
- atomwrite fornece garantias atômicas que built-ins do shell não têm
- Use `edit` para mudanças cirúrgicas em vez de reescritas completas
```bash
atomwrite edit src/lib.rs --old "old code" --new "updated code"
```


## Continue
- Instalar: `cargo install atomwrite`
- Continue executa comandos de terminal como parte de seu loop de agente
- Saída NDJSON é legível por máquina sem pós-processamento
```bash
atomwrite search 'deprecated' src/ --include '*.rs'
```


## Cline
- Instalar: `cargo install atomwrite`
- Cline usa comandos shell para operações de arquivo
- atomwrite substitui pipelines frágeis de sed/awk por operações atômicas
```bash
echo "new content" | atomwrite write src/config.rs
atomwrite diff src/old.rs src/new.rs
```


## Roo Code
- Instalar: `cargo install atomwrite`
- Roo Code roda comandos bash em seu ambiente de execução de agente
- Erros estruturados com campos `retryable` e `suggestion` guiam recuperação automática
```bash
atomwrite copy src/template.rs src/new.rs
```


## Amazon Q Developer
- Instalar: `cargo install atomwrite`
- Amazon Q executa comandos CLI em seu ambiente de desenvolvimento
- atomwrite fornece comportamento cross-platform consistente
```bash
atomwrite list src/ --depth 3
atomwrite count src/ --by-extension
```


## GitHub Copilot CLI
- Instalar: `cargo install atomwrite`
- Copilot CLI sugere e executa comandos shell
- Comandos atomwrite são auto-documentados via `--help` em cada subcomando
```bash
atomwrite calc "100 MB to bytes"
atomwrite regex "192.168.1.1" "10.0.0.1" "172.16.0.1"
```


## Agentes Customizados
- Instalar: `cargo install atomwrite`
- Invocar via `std::process::Command`, `subprocess.run()` ou equivalente
- Parsear stdout linha por linha como objetos JSON
- Verificar códigos de saída para classificação de erros
- Usar o campo `retryable` em respostas de erro para lógica de retry automático
```python
import subprocess
import json

result = subprocess.run(
    ["atomwrite", "read", "src/main.rs"],
    capture_output=True, text=True
)
for line in result.stdout.strip().split("\n"):
    data = json.loads(line)
    print(data["type"], data.get("path"))
```
