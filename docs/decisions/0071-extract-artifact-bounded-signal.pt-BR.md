# ADR-0071 — `extract-artifact`: sinal limitado, nunca bytes brutos (E5)

## Status
Aceito (2026-10-15)

## Contexto
- Um agente precisa do sinal de um PDF, de um arquivo Office ou de uma imagem: contagem de páginas, dimensões, cabeçalho de planilha, títulos de slides, texto do documento. Ele nunca precisa dos bytes brutos.
- Pôr bytes brutos no canal de dados NDJSON quebra o contrato (um objeto JSON por linha) e o orçamento de bytes.
- Os limites do extract eram literais no código do comando.

## Decisão
- Criar o subcomando `extract-artifact` CAMINHO. Ele lê o contêiner, extrai o sinal e emite um registro NDJSON limitado de `type: "artifact"` por artefato. O stream bruto nunca é repassado.
- A cobertura é estreita de propósito: contêineres cujo decoder é Rust puro, sem toolchain C e sem pacote de sistema. São `pdf`, `docx`, `xlsx`, `pptx`, e sondagem só de cabeçalho para imagens `png`, `jpeg`, `gif`, `bmp`, `webp`, `tiff` e `ico`. A detecção de tipo lê os magic bytes e nunca a extensão do arquivo.
- O registro é limitado. Toda parte lida é limitada por `--max-filesize`, e um contêiner forjado não estoura o payload porque planilhas e slides têm teto.
- Os quatro limites são chaves de config (ADR-0058): `extract.max_sample_rows` (até 100000), `extract.max_text_chars` (até 10000000), `extract.max_sheets` e `extract.max_slides` (até 10000). As flags `--max-sample-rows` e `--max-text-chars` ainda vencem a configuração.
- O comando é limitado por CPU num único artefato, então o pool compartilhado do rayon não é criado (ADR-0064).
- Dependências: `zip` (9.0.5, backend `deflate-flate2`), `flate2` (backend Rust), `quick-xml` (0.42), `imagesize` e `lopdf`.

## Consequências
- O `extract-artifact` é um dos 47 verbos. A contagem foi de 45 para 46 com ele e para 47 com o `config` (CHANGELOG, Corrections). Ele precisa do seu schema entre os 55 nomes do `--print-schema` (ADR-0067).
- O `zip` subiu para 9.0.5 e o `quick-xml` para 0.42 para fechar advisories nos leitores de OOXML e de artefato (RUSTSEC-2026-0367, -0368, -0369, -0194 e -0195).
- Qualquer coisa fora dos formatos listados não é lida, por desenho.

## Alternativas
- Transmitir os bytes brutos e deixar o agente decodificá-los: rejeitada, quebra o contrato NDJSON e o orçamento.
- Embrulhar um decoder que exige toolchain C ou pacote de sistema: rejeitada, o verbo continua Rust puro.

## Evidência
- `CHANGELOG.md:16,57,111,315-316` e `gaps.md:69-70` (E5).
- `src/commands/extract_artifact/mod.rs:3-17` (justificativa e cobertura), `:35-46` (limites padrão), `:48-64` (`ExtractArtifactArgs`), `:179` (a flag vence o valor de ajuste), `src/commands/extract_artifact/office.inc.rs:4`, `image.inc.rs:19`, `pdf.inc.rs`, `Cargo.toml:173-195` (dependências), `src/cli.rs:237`.
- `tests/cli_v0137_artefato.rs` (17 testes) e `tests/cli_v0137_config_verb.rs` (`extract_keys_bound_the_artifact_and_a_flag_still_wins`).
- Binário 0.1.37: um cabeçalho PNG sintético de 2x3 devolveu `{"type":"artifact","format":"png","width":2,"height":3}` e exit 0.

## Gatilho para revisar
- Um formato fora da lista ser necessário e ter um decoder em Rust puro.
- Um decoder da árvore receber um advisory.
