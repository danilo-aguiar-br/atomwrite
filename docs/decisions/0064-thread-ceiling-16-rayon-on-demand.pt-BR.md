# ADR-0064 — Teto automático de 16 threads e pool do rayon criado sob demanda

## Status
Aceito (2026-10-15)

## Contexto
- O pool do rayon era criado com uma thread por CPU para todo verbo. Num host de 72 CPUs, um `read` no `-j` padrão usava 44,3 MB e criava 72 threads (gaps.md W10).
- O walker custava cerca de 1,5 MB por thread (31 MB com 1 thread, 38 MB com 8, 142 MB com 72).
- Um `read` ou `verify` grande criava uma thread de pool por CPU pelo caminho do hash.

## Decisão
- A contagem automática de threads é `min(CPUs, MAX_AUTO_THREADS = 16, teto de RAM)`. O walker automático de diretórios é limitado ainda a `WALK_AUTO_MAX_THREADS = 12`, a mesma guarda que a crate `ignore` aplica.
- Um `-j N` explícito vale como pedido, mesmo acima de 16.
- O pool do rayon é criado sob demanda, só para verbo que usa `par_iter`: o `ensure_global_pool()` roda antes do primeiro iterador paralelo. `read`, `stat`, `write`, `move`, `get`, `set`, `del`, `rollback`, `calc`, `regex`, `extract`, `extract-artifact`, `verify`, `query`, `outline`, `scratch`, `locale`, `commands`, `agent-surface` e `completions` não criam nenhum.
- O `hash_bytes` e o hash por mmap pegam o pool limitado antes do primeiro hash em rayon (arquivos a partir de 1 MiB).
- `concurrency.max_threads` (1 a 256, padrão 16) e `concurrency.ram_fraction_percent` (1 a 100) são chaves de config (ADR-0058). Um assert de compilação mantém o `MAX_AUTO_THREADS` abaixo do teto da chave.

## Consequências
- O `read` no `-j` padrão foi de 44,3 MB para 17,9 MB. O `search` no `-j` padrão foi de 142,6 MB para 45,3 MB (ADR-0060).
- `list` e `count` ainda criam o pool, o padrão seguro documentado em `src/concurrency_walk.inc.rs` (16 threads em `-vvv list .`, gaps.md W42, P3).
- Um `-j 72` explícito não é limitado (gaps.md W44 B7, P3).
- O teto de RAM existe só no Linux e não lê cgroup (lista P3 do gaps.md).
- Docs que dizem que o padrão usa todos os cores precisam dizer "todos os cores até um teto embutido, limitado pela RAM". A ajuda do `-j` já diz.

## Alternativas
- Uma thread por CPU para todo verbo: rejeitada, pelo custo medido de memória e de threads em verbos que nunca paralelizam.
- Limitar também um `-j` explícito ao teto: não feito, um pedido explícito vale (gaps.md W10 e W44 B7, P3).

## Evidência
- `CHANGELOG.md:69,137-145,193,276` e `gaps.md:177-181,288` (W10, W42).
- `src/constants_concurrency.inc.rs:42-47` (16 e 12), `src/concurrency.rs:40-47,146,179-190`, `src/concurrency_walk.inc.rs:242-281` (`ensure_global_pool`), `src/checksum.rs:49,107`, `src/config_tuning.inc.rs:18`, `src/constants_config.inc.rs:34,66`.
- `tests/cli_concurrency.rs` e os testes unitários `src/concurrency.rs:380-386`.
- Binário 0.1.37: `config get concurrency.max_threads` devolveu 16 (`origin` `default`).

## Gatilho para revisar
- Uma medição no `-j` padrão sair da meta de 64 MB.
- Limites de cgroup passarem a ser lidos para o teto de RAM.
