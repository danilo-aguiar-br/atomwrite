[Read in English](INTEGRATIONS.md)


# Integrações

- atomwrite é uma CLI que fala NDJSON no stdout: qualquer agente que consegue rodar um comando shell pode chamá-la como subprocesso


## Agentes Compatíveis (v0.1.37)
- Superfície atual: 47 subcomandos (SSOT via `atomwrite agent-surface` / `CANONICAL_SUBCOMMAND_COUNT`), todos listados com exemplo testado em [Os 47 Comandos](#os-47-comandos)
- Histórico (na v0.1.35): 41 subcomandos após residuais A-* (write grande `--ack-overwrite`, `watch_summary`, markers de merge, delete `--plan` e não `--confirm`/`--yes`)
- Integração de agente (pin `^0.1.37`): descubra tools com `agent-surface` / `commands`, estagie patches grandes ou frágeis via `scratch init` + `--old-file`/`--new-file` sob `.atomwrite/scratch/`, parseie o exit code primeiro, depois trate toda linha de erro como `ok: false` (always-emit mesmo sob `--filter`)
- Prefira os globais `--select`/`--fields`, `--filter`, `--limit` a jq para projeção de campos
- atomwrite requer apenas acesso a shell para funcionar
- Saída NDJSON (um objeto JSON por linha) é parseável por qualquer parser JSON sem adaptador customizado
- Sem plugins, extensões ou SDKs necessários
- Status de validação: só os agentes marcados como validados na [Tabela Resumida](#tabela-resumida) foram executados, todo outro agente está marcado como não validado
- Ajuste sem editar TOML à mão: `config list` mostra cada chave com o valor efetivo e a origem, `config set CHAVE VALOR --scope global|workspace` grava uma chave de forma atômica (`--scope` é obrigatório, não há padrão), `config unset` a remove
- Um evento terminal por execução: quando um resumo falha ele carrega `ok: false`, `code` e `exit` nele mesmo
- Leia `partial` e `files_failed` em todo resumo que os publica (`search`, `transform`, `replace`, `scope`, `sparse`, `semantic-search` e `case`), e espere `MATCH_FAILED` (exit 65) quando um par do `edit-loop` não casa
- `count`, `list`, `hash` e `copy` falham na primeira leitura com erro, e as duas famílias estão fixadas no ADR-0077 (`docs/decisions/0077-answer-family-and-walk-honesty.md`)
- Shape global agent-native do stdout (v0.1.36): `--select`/`--fields`, `--filter`, `--limit` (sem `-n` global), erros sempre emitidos
- `--timeout-secs` global tem padrão 120 (passe `0` para desligar, exit 124 no prazo)
- Multi-apply fuzzy é one-pass no conteúdo original, nunca reescaneia texto inserido, aplica uma única vez por padrão e trata um pattern embutido no replacement como um apply único
- Cascata fuzzy padrão para agentes: `--fuzzy auto|aggressive`, e `--fuzzy off` = exact-only
- Em sucesso de `edit --replace-all` parseie `match_count`, em indent flexível parseie `indent_adjusted`
- Leia `best_candidate` em falhas de match (exit 65) em vez de reler o arquivo inteiro
- Use `agent-surface` para o manifesto de tools, a integração é por subprocesso, CLI com saída NDJSON
- Prefira `recipe list|run` para pipelines search-replace-verify de vários passos (exclui `*.bak.*`, inclusive no passo hash)
- O método de backup é `reflink_or_copy`, nunca hardlink do arquivo vivo
- Exit codes: leia a tabela com `atomwrite agent-surface --format json | jaq '.exit_codes'` (41 entradas, 33 números distintos) e ramifique por `exit` e `error_class`
- Install slim: `cargo install --path . --locked --force --no-default-features --features core`
- No build slim os verbos AST `outline`, `query`, `scope`, `transform` e `sparse outline` falham fechados com exit 78 e uma dica de rebuild (lido de `src/commands/*_stub.rs`, o build slim não foi executado para este documento)
- O build padrão não tem `watch`: o verbo `watch` sai 78 até você compilar com `--features watch`
- Pin de agentes em `^0.1.37`
- Status das plataformas: Linux é exercitado de ponta a ponta, Windows é checado com `cargo check --target x86_64-pc-windows-gnu` a partir de um host Linux, macOS não é exercitado em um Mac real (veja `CHANGELOG.pt-BR.md`, 0.1.36)
- Histórico: a v0.1.15 corrigiu o G117 (`edit` multi-par com paridade fuzzy, `pair_results`, `failed_pair_index`, `--partial` opt-in) e o G118 (`write` resolve o alvo contra o workspace, então append e checksum são seguros com CWD divergente)
- Histórico: a v0.1.16 adicionou o G119 L1 WalPolicy e L4 HeuristicsEngine, a v0.1.17 ligou o heal de startup G119 L3, a v0.1.18 adicionou a propagação resolve-first universal do G118, testes cross-flag do G120 L3 e casos de borda do G117


## Tabela Resumida

| Agente | Validação | Evidência |
|--------|-----------|-----------|
| Claude Code | Validado em 2026-10-19 | todo exemplo deste guia foi executado pela sua ferramenta de shell |
| Agentes Customizados | Validado em 2026-10-19 | o snippet Python de subprocesso abaixo foi executado |
| Cursor | Não validado | nenhuma execução registrada |
| Windsurf | Não validado | nenhuma execução registrada |
| Codex CLI | Não validado | nenhuma execução registrada |
| Aider | Não validado | nenhuma execução registrada |
| Continue | Não validado | nenhuma execução registrada |
| Cline | Não validado | nenhuma execução registrada |
| Roo Code | Não validado | nenhuma execução registrada |
| Amazon Q Developer | Não validado | nenhuma execução registrada |
| GitHub Copilot CLI | Não validado | nenhuma execução registrada |


## Os 47 Comandos
- Defina `WS` uma vez como o caminho absoluto da raiz do workspace antes de rodar um exemplo: `WS="$PWD"`
- Todo exemplo rodou com exit 0 em 2026-10-19 contra `atomwrite 0.1.37` num workspace descartável, na ordem abaixo, exceto `watch` (veja a entrada dele)
- Os exemplos assumem os arquivos `src/a.rs` (com uma chamada `unwrap()`), `src/b.rs` (com a palavra `beta`), `src/c.rs` (com `user_id`), `src/tmp.rs`, `config.toml` (com `package.name` e `features.x`), `notes.txt`, `tmp.txt`, `base.txt`, `ours.txt`, `theirs.txt`, `img.png` e `rules.yaml`
- `read` — lê um arquivo com checksum, tamanho e linhas, com opção de só início, fim ou faixa. Exemplo: `atomwrite --workspace "$WS" read --head 2 src/a.rs`
- `write` — cria ou sobrescreve um arquivo de forma atômica a partir do stdin. Exemplo: `printf 'x\n' | atomwrite --workspace "$WS" write notes/new.txt`
- `edit` — edita um arquivo por texto exato ou aproximado, número de linha ou marcador. Exemplo: `atomwrite --workspace "$WS" edit src/b.rs --old beta --new gamma`
- `delete` — apaga arquivos ou árvores com backup, ou só mostra o plano com `--plan`. Exemplo: `atomwrite --workspace "$WS" delete --plan tmp.txt`
- `copy` — copia um arquivo ou árvore com verificação de checksum. Exemplo: `atomwrite --workspace "$WS" copy src/a.rs src/a_copy.rs`
- `move` — move ou renomeia um arquivo de forma atômica. Exemplo: `atomwrite --workspace "$WS" move src/tmp.rs src/tmp2.rs`
- `apply` — aplica um patch (diff unificado, SEARCH/REPLACE, arquivo inteiro) lido do stdin. Exemplo: `printf 'new body\n' | atomwrite --workspace "$WS" apply --format full notes.txt`
- `search` — busca conteúdo de arquivos em paralelo com o motor do ripgrep. Exemplo: `atomwrite --workspace "$WS" search alpha src/`
- `replace` — troca texto em vários arquivos com escrita atômica e casamento aproximado. Exemplo: `atomwrite --workspace "$WS" replace --dry-run alpha omega src/`
- `transform` — reescreve código por padrão de AST (ast-grep). Exemplo: `atomwrite --workspace "$WS" transform --dry-run -p '$E.unwrap()' -r '$E?' -l rust src/`
- `hash` — mostra o checksum BLAKE3 de arquivos. Exemplo: `atomwrite --workspace "$WS" hash src/a.rs`
- `count` — conta linhas e arquivos por extensão ou por tamanho. Exemplo: `atomwrite --workspace "$WS" count --by-extension src/`
- `diff` — compara dois arquivos (unificado, estatística ou mudanças). Exemplo: `atomwrite --workspace "$WS" diff src/a.rs src/b.rs --unified`
- `list` — lista uma árvore de diretórios com metadados. Exemplo: `atomwrite --workspace "$WS" list --long --depth 1 src/`
- `extract` — extrai campos de NDJSON lido do stdin. Exemplo: `printf '{"a":1}\n' | atomwrite extract --field a`
- `extract-artifact` — sinal limitado dos cabeçalhos de PDF, OOXML e imagem, nunca os bytes brutos. Exemplo: `atomwrite --workspace "$WS" extract-artifact img.png`
- `scope` — escopo gramatical: age sobre comentários, strings, funções e outras categorias de código. Exemplo: `atomwrite --workspace "$WS" scope src/ --language rust --query comments --delete --dry-run`
- `regex` — gera uma regex a partir de exemplos. Exemplo: `atomwrite regex abc-12 xyz-99 id-7 --digits`
- `calc` — cálculo e conversão de unidades. Exemplo: `atomwrite calc "2 hours to seconds"`
- `completions` — gera ou instala completions de shell (bash, zsh, fish, elvish, powershell). Exemplo: `atomwrite completions bash > /dev/null`
- `backup` — cria backups com data e hora e checksum BLAKE3. Exemplo: `atomwrite --workspace "$WS" backup src/a.rs --retention 3`
- `rollback` — restaura um arquivo a partir de um backup anterior (rode `backup` antes). Exemplo: `atomwrite --workspace "$WS" rollback src/a.rs --latest --verify`
- `batch` — executa operações NDJSON do stdin, com opção de transação única. Exemplo: `printf '{"op":"write","target":"batch.txt","content":"x"}\n' | atomwrite --workspace "$WS" batch --transaction`
- `edit-loop` — aplica N pares antigo/novo numa escrita só, tudo ou nada (rode antes o exemplo do `edit`). Exemplo: `printf '[{"old":"gamma","new":"delta"}]\n' | atomwrite --workspace "$WS" edit-loop src/b.rs`
- `prune-backups` — remove arquivos `.bak` antigos por idade ou quantidade. Exemplo: `atomwrite --workspace "$WS" prune-backups --max-age-secs 86400 --dry-run .`
- `wal-stats` — mostra o estado dos journals WAL (só leitura). Exemplo: `atomwrite --workspace "$WS" wal-stats`
- `wal-heal` — remove journals WAL terminais órfãos mais antigos que um limite, e também remove um journal `Started` cujo PID dono sumiu, publicado como `dead_owner_removed`. Exemplo: `atomwrite --workspace "$WS" wal-heal --threshold-secs 3600 .`
- `verify` — confere um arquivo contra um hash BLAKE3 esperado (exit 0 se igual, 81 se diferente). Exemplo: `atomwrite --workspace "$WS" verify src/a.rs "$(atomwrite --workspace "$WS" hash src/a.rs | jaq -r .checksum)"`
- `set` — grava um valor num caminho com pontos em arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" set config.toml package.name demo2`
- `get` — lê um valor num caminho com pontos em arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" get config.toml package.name`
- `del` — remove uma chave de um arquivo TOML ou JSON. Exemplo: `atomwrite --workspace "$WS" del config.toml features.x`
- `case` — renomeia identificadores entre estilos de caixa em arquivos e diretórios, e o diretório é expandido pelo walker. Exemplo: `atomwrite --workspace "$WS" case --subvert user_id UserId --to pascal --dry-run src/c.rs`
- `query` — percorre a AST do tree-sitter e emite os nós. Exemplo: `atomwrite --workspace "$WS" query --kinds src/a.rs`
- `outline` — emite a estrutura de alto nível de um arquivo de código (um arquivo, nunca um diretório). Exemplo: `atomwrite --workspace "$WS" outline src/a.rs`
- `semantic-merge` — merge de três vias por linha com marcadores de conflito, não é AST. Exemplo: `atomwrite --workspace "$WS" semantic-merge --base base.txt --ours ours.txt --theirs theirs.txt --output merged.txt --fail-on-conflict`
- `sparse` — lista, leitura e outline com orçamento para árvores grandes. Exemplo: `atomwrite --workspace "$WS" sparse list src/ --max-files 10 --max-bytes 1048576`
- `recipe` — lista e roda receitas embutidas de vários passos. Exemplo: `atomwrite --workspace "$WS" recipe run --name search-replace-verify --pattern alpha --replacement omega --path src --dry-run`
- `stat` — só metadados, alias de `read --stat`. Exemplo: `atomwrite --workspace "$WS" stat src/a.rs`
- `agent-surface` — manifesto de ferramentas para agentes, a fonte única dos subcomandos. Exemplo: `atomwrite agent-surface --format json`
- `watch` — transmite eventos do sistema de arquivos com debounce, termina com `watch_summary`, exige um build com `--features watch`. Exemplo: `atomwrite --workspace "$WS" watch src/ --max-events 1 --idle-exit-ms 500` (o build padrão sai 78 com uma dica de rebuild, e esse é o único exit medido aqui)
- `codemod` — aplica uma lista YAML de regras de reescrita por AST numa árvore. Exemplo: `atomwrite --workspace "$WS" codemod --rules rules.yaml --dry-run src/`
- `semantic-search` — ranking offline por Jaccard de arquivos para uma consulta. Exemplo: `atomwrite --workspace "$WS" semantic-search "unwrap value" src/ --k 5 --min-score 0.01`
- `doctor` — diagnóstico de prontidão do host. Exemplo: `atomwrite doctor`
- `scratch` — cria o diretório de staging `.atomwrite/scratch` dentro do jail do workspace (o caminho é obrigatório). Exemplo: `atomwrite --workspace "$WS" scratch init .atomwrite/scratch`
- `locale` — mostra, define ou limpa o idioma da interface. Exemplo: `atomwrite locale`
- `commands` — imprime a árvore completa de comandos em JSON. Exemplo: `atomwrite commands --include-globals`
- `config` — lê e grava chaves de ajuste persistentes (37 chaves, escopos global e workspace). Exemplo: `atomwrite --workspace "$WS" config get fuzzy.threshold`


## Claude Code
- Instalação: `cargo install atomwrite`
- Status: validado em 2026-10-19, os exemplos deste guia rodaram pela ferramenta de shell do Claude Code
- Saída NDJSON é parseada linha a linha sem adaptadores
- Use `--workspace` para casar com a raiz do projeto
- Combine com `--expect-checksum` para edições concorrentes seguras
- Adicione comandos do atomwrite ao CLAUDE.md para descoberta automática
- Use `set/get/del/case/query/outline` para configuração estruturada e análise AST
```bash
# Exemplo de entrada em CLAUDE.md
# Use atomwrite para todas as operações de arquivo
echo "conteúdo" | atomwrite --workspace . write src/file.rs
atomwrite --workspace . read src/file.rs
atomwrite --workspace . search 'pattern' src/
# caminhar o AST de um arquivo Rust
atomwrite --workspace . query src/main.rs --kinds
```


## Cursor
- Instalação: `cargo install atomwrite`
- Status: não validado
- Use `--dry-run` para prévia antes de operações destrutivas
```bash
atomwrite --workspace . search 'TODO' src/ --include '*.rs'
atomwrite --workspace . replace --dry-run 'old_api' 'new_api' src/
```


## Windsurf
- Instalação: `cargo install atomwrite`
- Status: não validado
- Operações em lote minimizam o número de chamadas
```bash
atomwrite --workspace . batch < manifest.ndjson
```


## Codex CLI
- Instalação: `cargo install atomwrite`
- Status: não validado
- atomwrite fica dentro do diretório dado por `--workspace`
- Checksums permitem verificação de estado entre passos de execução
- Use `case` para refatorar identificadores em vários arquivos
```bash
atomwrite --workspace . read src/main.rs
atomwrite --workspace . hash src/main.rs
atomwrite --workspace . case src/main.rs --subvert user_id UserId --to pascal
```


## Aider
- Instalação: `cargo install atomwrite`
- Status: não validado
- Use `edit` para mudanças cirúrgicas em vez de reescrever o arquivo inteiro
- Use `outline` num arquivo para obter um mapa da estrutura dele antes das edições
```bash
atomwrite --workspace . edit src/lib.rs --old "código antigo" --new "código atualizado"
atomwrite --workspace . outline src/lib.rs  # veja a estrutura primeiro
```


## Continue
- Instalação: `cargo install atomwrite`
- Status: não validado
```bash
atomwrite --workspace . search 'deprecated' src/ --include '*.rs'
```


## Cline
- Instalação: `cargo install atomwrite`
- Status: não validado
- atomwrite grava de forma atômica, o que um redirecionamento de shell não faz
```bash
echo "novo conteúdo" | atomwrite --workspace . write src/config.rs
atomwrite --workspace . diff src/old.rs src/new.rs
```


## Roo Code
- Instalação: `cargo install atomwrite`
- Status: não validado
- Erros estruturados com os campos `retryable` e `suggestion` guiam a recuperação automática
```bash
atomwrite --workspace . copy src/template.rs src/new.rs
```


## Amazon Q Developer
- Instalação: `cargo install atomwrite`
- Status: não validado
```bash
atomwrite --workspace . list src/ --depth 3
atomwrite --workspace . count src/ --by-extension
```


## GitHub Copilot CLI
- Instalação: `cargo install atomwrite`
- Status: não validado
- Os comandos do atomwrite se autodocumentam via `--help` em cada subcomando
- Use `get` para ler metadados do pacote sem escrever um parser
```bash
atomwrite calc "100 MB to bytes"
atomwrite regex "192.168.1.1" "10.0.0.1" "172.16.0.1"
atomwrite --workspace . get Cargo.toml package.version
```


## Agentes Customizados
- Instalação: `cargo install atomwrite`
- Status: validado em 2026-10-19 com o snippet abaixo
- Invoque via `std::process::Command`, `subprocess.run()` ou equivalente
- Parseie o stdout linha a linha como objetos JSON
- Verifique os exit codes para classificar o erro (a tabela completa está em `agent-surface --format json`)
- Use o campo `retryable` nas respostas de erro para lógica de retry automática
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
