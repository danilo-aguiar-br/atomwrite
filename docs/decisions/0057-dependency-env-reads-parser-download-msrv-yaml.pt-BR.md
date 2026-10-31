# ADR-0057 — Leituras de ambiente de dependências, download de parsers, MSRV 1.89 e pilha YAML

## Status
Aceito (2026-10-19)

## Contexto
- O código do produto não lê nenhuma variável de ambiente em tempo de execução (G-007). A configuração é feita por flags da CLI mais `config.toml` XDG.
- Três grupos de código de dependências ainda leem o ambiente do processo. Eles estão listados aqui para que ninguém os encontre por surpresa.
- O `cargo audit` reportava RUSTSEC-2024-0449 (`unsafe-libyaml`, sem manutenção) e RUSTSEC-2024-0384 (`instant`, sem manutenção).
- O `tree-sitter-language-pack` é compilado com `download`, que traz `ureq`, `rustls`, `ring` e `zstd-sys`.

## Decisão
### 1. Exceções mínimas de leitura de ambiente (só código de terceiros)
- O `sys-locale` 0.3.2 lê `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` e `LANG` no Unix, para detectar o idioma do SO no i18n automático (`src/locale.rs`).
- O `directories` 6 lê `HOME` e `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME` no Linux, para resolver os caminhos XDG do `.atomwrite.toml` (`src/storage.rs`).
- O `tree-sitter-language-pack` (via `dirs`) lê `HOME` e `XDG_CACHE_HOME` para o cache de parsers, e o `ureq` 3.3 lê `ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY` e `NO_PROXY` (e as versões minúsculas) quando um parser é baixado.
- Tanto o i18n quanto o XDG são exigidos pelas regras do projeto (idioma automático do SO, layout XDG), então as duas primeiras são exceções aceitas.
- Não existe nenhuma variável `ATOMWRITE_*`, e o código do produto nunca chama `std::env::var`.

### 2. Manter a feature `download` do `tree-sitter-language-pack`
- A versão 1.8.1 não compila nenhuma gramática. Todo parser é baixado no primeiro uso ou já está no cache local.
- Medido sem a feature (build debug, cache vazio): `outline` e `query` falharam nas seis linguagens testadas (Rust, TypeScript, Python, Go, Java, Ruby) com `Language '<nome>' not found`.
- Medido com a feature (cache novo e vazio, rede disponível): `outline` e `query` funcionaram nas mesmas seis linguagens.
- `transform --dry-run` se comportou igual nos dois builds, porque usa o `ast-grep-language` (compilado dentro), e não o pack.
- Portanto `download` é a única forma de `outline` e `query` obterem uma gramática (medido). O `--syntax-check` também chama `get_parser` em `src/syntax_check.rs` (lido no código, não executado). A feature fica.
- A CLI nunca chama a API de download (`DownloadManager`, `init`, `download`) por conta própria. O download acontece dentro de `get_language` e `get_parser` no primeiro uso de uma linguagem.
- As variáveis de proxy do `ureq` são a exceção documentada para esse caminho. Um host sem rede pode preencher o diretório de cache antes.

### 3. MSRV 1.89
- O `rust-version` sobe de 1.88 para 1.89, porque o `rust-i18n` 4.2 exige. O toolchain do host é 1.94 (`rust-toolchain.toml`).
- `README.md`, `README.pt-BR.md`, `docs/INSTALL*.md` e `docs/CROSS_PLATFORM*.md` informam 1.89.

### 4. Pilha YAML
- O `rust-i18n` sobe para 4.2 (resolvido 4.2.4). Ele troca o `serde_yaml` pelo `serde-saphyr`.
- As regras YAML do `transform` são lidas com `serde-saphyr` 1.3 no lugar do `serde_yaml_ng`. A versão 1.x é a major que o `rust-i18n-support` já usa, então o grafo mantém uma só cópia. A versão 2.0 adicionaria uma duplicata e não foi escolhida.
- Medido em dez arquivos de regras (válido, vazio, campo ausente, campo desconhecido, multi-documento, malformado, âncoras, lista vazia, linguagem numérica, estilo flow): aceitar ou rejeitar, código de saída e contagens do resumo são idênticos entre os dois crates. Só o texto da mensagem de erro de parse muda.
- Os quatro testes YAML de `tests/cli_v012_audit_regressions.rs` agora fazem parse em `serde_json::Value` com `serde_saphyr`, com as mesmas asserções.
- O `unsafe-libyaml` (RUSTSEC-2024-0449) saiu do `Cargo.lock`. Nenhuma exceção é necessária.

### 5. `notify` 8
- O `notify` (feature opcional `watch`) sobe de 7 para 8.2.0, que elimina o `instant` (RUSTSEC-2024-0384).
- `cargo tree -d --target all` fica igual no conjunto de features padrão (193 entradas antes, 193 depois).
- Com `--features watch` entram nove crates Windows duplicados (`windows-targets` e oito `windows_*`), agora listados na tabela `skip` do `deny.toml` com o motivo.

## Consequências
- O `cargo audit` não reporta nenhum advisory: nem `RUSTSEC-2024-0449` (`unsafe-libyaml`) nem `RUSTSEC-2024-0384` (`instant`) aparecem.
- O binário padrão ainda precisa de rede uma vez por linguagem para `outline` e `query`, e para `--syntax-check` pela leitura do código. É um custo conhecido e documentado.
- Uma nova leitura de ambiente feita por dependência precisa entrar na seção 1 ou ser rejeitada.

## Alternativas
- Remover `download`: rejeitada, desativa `outline`, `query` e a checagem de sintaxe em todas as linguagens.
- `serde-saphyr` 2.0: rejeitada por ora, adiciona uma segunda cópia do crate.
- Manter `notify` 7 com exceção de advisory: rejeitada, a correção existe e afeta só a feature opcional.

## Gatilho para revisar
- O `tree-sitter-language-pack` passar a oferecer gramáticas compiladas como features.
- O `rust-i18n-support` migrar para o `serde-saphyr` 2.
