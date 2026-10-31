# ADR-0049: fiação ativa da config `[defaults]` do `.atomwrite.toml`

> Histórico (pré-0.1.35): os knobs de produto `ATOMWRITE_*` / env descritos abaixo foram superados. A configuração de runtime é apenas flags de CLI + `config.toml` XDG / `atomwrite set|get`.


- Status: Aceito
- Data: 2026-07-06
- Contexto: As chaves `backup`/`retention` de `[defaults]` do `.atomwrite.toml` eram lidas pelo `load_config`, mas nunca consultadas por nenhum comando mutador (GAP-CONFIG-DEFAULTS-DEAD): o resolvedor que existia na época recebia apenas `(backup: bool, no_backup: bool)` como argumentos e não tinha caminho para alcançar os valores de config lidos. Usuários que definiam `[defaults]` na config do projeto viam isso ser ignorado em silêncio.
- Decisão: O `load_config` agora é chamado exatamente uma vez, em `lib.rs::run()`, e o `DefaultsSection` resultante é repassado e propagado a todo handler de comando mutador. O `resolve_backup()` (ADR-0048) agora recebe `&DefaultsSection` como segundo argumento e consulta `defaults.backup`/`defaults.retention` como o terceiro nível de precedência: variável de ambiente `ATOMWRITE_BACKUP` > flags de CLI (`--backup`/`--no-backup`/`--retention`) > `[defaults]` do `.atomwrite.toml` > padrão embutido (`true` / `5`).
- Consequências:
  - + As chaves `backup`/`retention` de `[defaults]` do `.atomwrite.toml` agora são efetivas em todo subcomando mutador, fechando o GAP-CONFIG-DEFAULTS-DEAD.
  - + A config é carregada uma vez por invocação, em vez de ser relida ou ignorada por handler, evitando I/O redundante e divergência entre handlers.
  - - Handlers que alteram conteúdo agora carregam um parâmetro extra `&DefaultsSection`, uma mudança de assinatura pontual absorvida pela refatoração do `BackupOpts` (ADR-0048) no mesmo release.
- Alternativas consideradas:
  1. Carregar a config de forma independente dentro de cada handler. Rejeitado: duplica I/O e arrisca reintroduzir a divergência por handler, exatamente o modo de falha que este ADR fecha.
  2. **Manter `[defaults]` lido-mas-não-usado e documentá-lo como reservado para um release futuro.** Rejeitado: a config já documentava as chaves como efetivas, e deixá-las mortas é uma divergência entre documentação e comportamento, não um adiamento.
- Gatilho para revisitar: Se `[defaults]` ganhar chaves adicionais além de `backup`/`retention`, confirmar que o `DefaultsSection` e seu caminho de propagação são estendidos de forma consistente, em vez de adicionar um caminho de resolução paralelo.
