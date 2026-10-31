[Read in English](PRIVACY.md)


# Política de Privacidade — atomwrite


- Última atualização: 2026-10-31 (atomwrite 0.1.37)


## Resumo
- `atomwrite` é uma CLI local
- Não tem telemetria, analytics nem phone-home: nada é coletado, então não existe chave de opt-in para ligar (um opt-in explícito seria exigido antes de qualquer telemetria ser adicionada)
- Nunca envia o conteúdo dos seus arquivos
- O único tráfego de rede que o produto inicia sozinho é o download único dos parsers do tree-sitter, descrito em [Acesso à Rede](#acesso-à-rede)


## O Que Roda Localmente
- Todas as leituras, escritas, buscas, backups e sidecars WAL executam na máquina que invoca o binário
- A configuração é somente flags CLI + diretórios XDG + `.atomwrite.toml`, sem knobs de produto `ATOMWRITE_*` em runtime (política desde a v0.1.35, linha atual `0.1.37`)
- O código do produto não chama `env::var`, e um teste guarda isso
- As crates de SO `sys-locale` e `directories` leem `LANG` (e `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` no Unix), `HOME` e `XDG_*` por você, veja [Leituras de Ambiente por Dependências](#leituras-de-ambiente-por-dependências)
- O `doctor` não lê nenhuma variável de ambiente: a detecção do host usa só sondas locais de arquivo
- Diretórios de staging do agente sob `.atomwrite/scratch/` são arquivos locais do workspace, nunca enviados
- `atomwrite config set` e `config unset` gravam apenas o `config.toml` XDG (modo `0600` no Unix) ou o `.atomwrite.toml` do workspace, de forma atômica, e `config list` e `config get` apenas os leem
- Os verbos `watch`, `semantic-search` e `semantic-merge` operam somente em dados do filesystem local


## Acesso à Rede
- Os verbos centrais de leitura, escrita, edição, busca, troca e todos os outros não precisam de rede
- Os recursos de AST precisam dos parsers do tree-sitter, e o build não embute nenhum: `outline`, `query` e `--syntax-check` baixam o pacote de parsers na primeira vez que uma linguagem é usada e o cache local não tem cópia
- O download vem de `github.com/xberg-io/tree-sitter-language-pack` (assets de release), pelo cliente HTTP `ureq` da crate `tree-sitter-language-pack` 1.22.1 (`ureq`, `rustls` e `ring` estão no binário padrão)
- Medido em 2026-10-31 no manifesto da `v1.22.1`: cerca de 22 MiB (23.031.807 bytes para `linux-x86_64`), gravados em `$XDG_CACHE_HOME/tree-sitter-language-pack/v1.22.1/` (`~/.cache/tree-sitter-language-pack/v1.22.1/` por padrão), e os seis parsers que esta máquina já extraiu ocupam 3,9 MiB
- A requisição não leva dados dos seus arquivos, o cliente HTTP acrescenta os próprios cabeçalhos padrão (não inspecionados para este documento), e o host que serve o arquivo vê o endereço IP da sua máquina, como em qualquer download
- As execuções seguintes reutilizam o cache e não fazem requisição
- Como evitar o download: copie um diretório de cache já preenchido de uma máquina conectada para o mesmo caminho, compile sem `ast` (`cargo install atomwrite --no-default-features --features core`), ou não use `outline`, `query` e `--syntax-check`
- `transform` usa as gramáticas compiladas no `ast-grep-language` e não usa este download (medido na ADR-0057)
- `cargo install atomwrite` baixa crates do crates.io, e esse tráfego é entre o seu host e o crates.io


## Leituras de Ambiente por Dependências
- Código de terceiros lê o ambiente do processo em três lugares, listados na ADR-0057 para que nenhum leitor os encontre de surpresa
- `sys-locale` lê `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` e `LANG` no Unix para detectar o idioma do SO para a interface
- `directories` lê `HOME` e `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME` no Linux para posicionar os caminhos de config, estado e dados
- `tree-sitter-language-pack` lê `HOME` e `XDG_CACHE_HOME` para o cache de parsers, e `ureq` lê `ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY` e `NO_PROXY` (com variantes minúsculas) quando baixa um parser
- Nenhum desses valores é enviado a lugar algum pelo atomwrite


## Arquivos que o atomwrite Grava
- Config: `config.toml` sob `$XDG_CONFIG_HOME/atomwrite/` (`~/.config/atomwrite/` no Linux), modo `0600` no Unix, gravado só por `config set` e `config unset`
- Config do workspace: `.atomwrite.toml` na raiz do workspace, gravado só por `config set --scope workspace`
- Preferência de locale: o arquivo de uma linha `locale` no mesmo diretório, modo `0600` quando a plataforma permite, gravado só por `locale --set`
- Logs: somente quando `log.file` é `true` (padrão `false`), linhas JSON sob `$XDG_STATE_HOME/atomwrite/logs/atomwrite.<data>.jsonl`, rotacionadas por dia por padrão (`log.rotation`) e limitadas a 7 arquivos
- As linhas de log guardam eventos de diagnóstico e podem incluir os caminhos de arquivo e o locale da execução, então trate-as como sensíveis quando os caminhos forem
- Journals WAL: `.atomwrite.journal.<alvo>.atomwrite.journal.json` ao lado do arquivo alvo, removidos depois de uma escrita confirmada e pelo `wal-heal`
- Backups: `<arquivo>.bak.<timestamp>` ao lado do arquivo, criados pelos verbos que gravam e removidos depois de uma escrita bem-sucedida por padrão (`delete` e `replace` os mantêm, medido em 2026-10-31, e `prune-backups` remove os antigos)
- Sidecars de lock (`.<alvo>.atomwrite.lock`) e arquivos temporários ao lado do alvo durante uma escrita
- O sidecar de lock mantém o nome literal até `.<alvo>.atomwrite.lock` passar de 255 bytes, e acima disso ele vira um digest curto BLAKE3 no mesmo diretório, então o nome não carrega caminho além do nome do alvo
- Staging: `.atomwrite/scratch/` no workspace, criado por `scratch init`
- Scripts de completion de shell quando você roda `atomwrite completions --install`
- Relatórios de falha: o hook `human-panic` grava um arquivo `report-<uuid>.toml` no diretório temporário do SO quando o programa entra em pânico, com o nome do SO, a versão da crate, a mensagem do pânico e um backtrace, e ele pode incluir caminhos que aparecem neles
- O relatório de falha é só um arquivo local e nunca é transmitido, você decide se o anexa a uma issue
- O cache de parsers descrito em [Acesso à Rede](#acesso-à-rede)


## O Que Nunca É Coletado
- Nenhum analytics, serviço de relatório de falhas ou beacon de uso está embutido no binário
- A saída NDJSON pode conter caminhos de arquivo e trechos de conteúdo que você (ou um agente) escolheu processar, então trate o stdout como sensível se esses caminhos guardarem segredos


## Dados Pessoais e Locale
- O locale detectado ou escolhido é tratado como dado potencialmente pessoal
- Minimização: só uma tag de idioma (`en` ou `pt-BR`) é guardada, como uma única linha no arquivo `locale`
- Retenção: o arquivo fica no seu disco até você removê-lo, e o atomwrite nunca o copia para outro lugar
- Exclusão: rode `atomwrite locale --clear`, ou apague o arquivo no `preference_path` que `atomwrite locale` imprime
- O locale do SO é lido uma vez por execução e não sai do host
- Como nada é enviado para fora do host, o atomwrite não coleta dados pessoais no sentido da GDPR (UE) ou da LGPD (Brasil), e isto é uma descrição do projeto, não aconselhamento jurídico


## Dependências de Terceiros
- Crates upstream são regidas pelas próprias licenças (veja `deny.toml` / `cargo deny`)
- Se você habilitar tooling que chama serviços externos (wrappers não relacionados, hosts de agentes), esses sistemas têm políticas de privacidade separadas


## Contato
- Mantenedor: veja `Cargo.toml` `authors` / repositório listado nos metadados do pacote
