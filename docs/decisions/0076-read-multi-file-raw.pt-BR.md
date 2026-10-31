# ADR-0076 — O `read` aceita vários operandos e expande um nível de glob

## Status
Aceita (2026-10-28)

## Contexto
- O `read` recebia um `PathBuf` só, então o hook do Claude Code não conseguia trocar `cat a b` nem `cat *.rs` por uma chamada do atomwrite, e cada negação de glob custava uma volta extra de contexto
- Medido antes da mudança: `atomwrite read --format raw a.rs b.rs` saía 2 com `unexpected argument 'b.rs' found`, e o `cat` nos mesmos operandos imprimia os dois arquivos
- Medido antes da mudança: `atomwrite read --format raw <dir>` saía 74 e escrevia o envelope NDJSON de erro no STDOUT, então o canal de bytes levava registro JSON onde deveria haver arquivo
- A ordem do glob no shell segue o `LC_COLLATE`: com `LC_ALL=C` a ordem é a dos bytes dos nomes, que é a única ordem que um binário portátil pode prometer
- O `globset 0.4` já está no grafo via `ignore 0.4`, então casar nome contra padrão compilado não acrescenta crate ao build

## Decisão
### 1. Um slot variádico, e não dois
- `ReadArgs.path` virou `ReadArgs.paths: Vec<PathBuf>` com `num_args = 1..` e `required = true`, e `read` sem operando continua falhando no parse
- Operando explícito mantém a posição do argv, então `read a b` e `read b a` diferem igual ao `cat`
- `read` e `stat` são verbos de leitura idempotente e não aparecem em `explicit_target_roots`, então o slot variádico não carrega autoridade ambiente e a regra de ETD é cumprida em vez de contornada
- `stat` reusa `ReadArgs`, então vários alvos saem 65 com mensagem que nomeia o verbo e aponta o `--format raw`

### 2. O binário expande o padrão, e paga com contrato estreito
- `src/commands/read_globs.inc.rs` separa o operando em diretório a listar e componente final a casar, lista UM nível com `std::fs::read_dir` e casa com `globset::GlobBuilder::literal_separator(true)`
- Os nomes são ordenados por `OsStr::as_encoded_bytes`, que é byte no Unix e WTF-8 no Windows, então um comparador ordena os três sistemas
- Nome que começa com `.` é descartado, porque o `*` do shell nunca pega ponto inicial
- Padrão com `**` sai 65, e padrão com curinga no nome de um diretório também, porque recusar é melhor que expandir pela metade: resposta parcial silenciosa entrega menos bytes do que foi pedido, que é justamente o defeito que este verbo existe para remover
- O prefixo de diretório pode ser tão fundo quanto o chamador escrever, porque só o componente final é padrão, e operando que nomeia caminho existente é lido como está em vez de ser expandido
- O diretório que o padrão alcança passa por `path_safety::validate_path`, e cada alvo expandido passa de novo, então padrão não alarga o jail do `--workspace`

### 3. A concatenação streama, e um alvo só fica intocado
- Um alvo mantém o caminho exato de antes da mudança: `read_file_bytes` mais `apply_line_filters`, então `--head`, `--lines`, `--verify-checksum` e a prova byte a byte contra a 0.1.37 continuam valendo
- Vários alvos copiam bloco a bloco para um stdout travado uma vez, sem cabeçalho, sem separador e sem `\n` extra, e o buffer é uma constante `RAW_COPY_BUF_BYTES` para o pico de memória não seguir a soma dos tamanhos
- A cópia avança um `CancelCheck::for_bytes`, então o `--timeout-secs` corta uma concatenação de 196 MB no meio em vez de depois do último arquivo
- `--lines`, `--line`, `--context`, `--head`, `--tail`, `--grep`, `--stat` e `--verify-checksum` saem 65 com vários alvos, porque o `cat` não tem intervalo de linha e aplicar um a cada arquivo em silêncio é pior que recusar

### 4. O canal de bytes nunca carrega envelope de erro
- `is_text_mode` virou `is_byte_stream_mode` e cobre `read --format raw` e `stat --format raw` além de `search --format text`, então falha nesses modos vai para o stderr e o stdout fica vazio
- Arquivo ausente, diretório ou glob sem resultado imprime uma linha no stderr, e os outros alvos ainda chegam ao stdout
- O status é o código semântico da PRIMEIRA falha, escolha do dono contra o 1 chapado do `cat`: ausente sai 4, permissão sai 13 e jail sai 126

## Consequências
- O hook consegue trocar `cat a b` e `cat 'src/*.rs'` por uma chamada do atomwrite, e o guard resolve cada operando antes de reescrever o segmento
- Glob recursivo fica fora por decisão declarada, e não por limitação medida, e a recusa nomeia o limite em vez de escondê-lo
- macOS e Windows não são executados nesta máquina: a ordem por bytes e a listagem de um nível ficam provadas só em Linux
