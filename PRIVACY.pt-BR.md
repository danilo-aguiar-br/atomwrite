# Política de Privacidade — atomwrite

[Read in English](PRIVACY.md)

**Última atualização:** 2026-07-19

## Resumo

`atomwrite` é uma CLI local. Ele **não** faz phone-home, não coleta telemetria e não envia conteúdos de arquivos a terceiros por padrão.

## O que roda localmente

- Todas as leituras, escritas, buscas, backups e sidecars WAL executam na máquina que invoca o binário.
- Configuração é somente flags CLI + diretórios XDG + `.atomwrite.toml` (sem knobs de produto `ATOMWRITE_*` em runtime na v0.1.35). O env do host ainda pode fornecer sinais de SO como `NO_COLOR`; o locale do SO é lido uma vez via `sys-locale` apenas para idioma de sugestões de UI.
- Features opcionais (AST, watch, semantic) ainda operam somente em dados do filesystem local.
- **Preferência de locale (somente local):** ao rodar `atomwrite locale --set <tag>`, o atomwrite grava um arquivo de preferência de uma linha sob o diretório de config XDG (ex.: `~/.config/atomwrite/locale` no Linux) com modo `0600` quando a plataforma permite. Esse arquivo armazena apenas uma tag BCP 47 (`en` ou `pt-BR`). Nunca é enviado. O locale do SO não sai do host.

## O que o atomwrite em si nunca coleta

- Nenhum analytics, SaaS de crash-reporting ou beacon de uso está embutido no binário.
- Nenhum cliente de rede é necessário para as operações core de read/write/edit/search/replace.
- A saída NDJSON pode conter **caminhos de arquivo e trechos de conteúdo** que você (ou um agente) escolheu processar — trate stdout como sensível se esses caminhos guardarem segredos.

## Dados que você pode produzir

- Arquivos de backup (`*.bak.*`), journals WAL e temporários no workspace ou no diretório temp do SO.
- Scripts de completion de shell ao usar `atomwrite completions --install`.
- Scripts de instalação podem baixar artefatos de release do GitHub Releases quando você os executa — esse tráfego é entre o seu host e o GitHub, não um endpoint de telemetria do atomwrite.

## Dependências de terceiros

- Crates upstream são regidos pelas próprias licenças (veja `deny.toml` / `cargo deny`).
- Se você habilitar tooling que chama serviços externos (wrappers não relacionados, hosts de agentes), esses sistemas têm políticas de privacidade separadas.

## Contato

Maintainer: veja `Cargo.toml` `authors` / repositório listado nos metadados do pacote.


## Privacidade de configuração na v0.1.35

- Configuração de produto é somente flags CLI + diretórios XDG + `.atomwrite.toml`.
- Nenhuma telemetria de produto é coletada ou transmitida.
- O `doctor` pode ler env do host para diagnósticos locais (indicadores de CI); isso não é um canal remoto de phone-home.
