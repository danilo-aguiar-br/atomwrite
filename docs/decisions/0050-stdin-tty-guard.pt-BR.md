# ADR-0050: guard de stdin-tty para os modos consumidores de stdin do edit

- **Status**: Aceito
- **Date**: 2026-07-06
- **Context**: Os modos consumidores de stdin do `edit` (`--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between` e `--multi`) leem conteúdo novo do stdin. Quando o stdin era um terminal (sessão interativa, sem pipe ou redirecionamento), esses modos bloqueavam indefinidamente aguardando entrada que o usuário nunca pretendia digitar, exigindo anteriormente um wrapper externo de `timeout` e saindo com exit 143 (SIGTERM) em vez de falhar com um erro acionável.
- **Decision**: `main.rs` computa `stdin_is_tty = stdin.is_terminal()` uma única vez, usando a trait `IsTerminal` da standard library (estável desde Rust 1.70), e a passa como um `bool` simples através de `atomwrite::run()` até `cmd_edit` e seus helpers consumidores de stdin (`edit_by_line`, `edit_by_marker` e os handlers individuais de cada modo), que chamam o helper compartilhado `read_stdin_text_guarded(stdin, max_size, stdin_is_tty, mode_name)`. Quando `stdin_is_tty` é `true`, o guard retorna `AtomwriteError::InvalidInput` (exit 65) com uma `suggestion` acionável em vez de bloquear. `is_terminal()` em si é chamado exatamente uma vez, em `main.rs`; `edit.rs` e seus helpers apenas recebem o `bool` já computado, nunca chamam `is_terminal()` diretamente — isso mantém o caminho de código consumidor de stdin testável sem um terminal real (testes podem passar `stdin_is_tty: false` com um `Cursor` como stdin).
- **Consequences**:
  - **+** Modos de edit consumidores de stdin falham rápido (exit 65) com uma `suggestion` quando invocados interativamente, em vez de travar exigindo um `timeout` externo + SIGTERM (exit 143).
  - **+** `edit.rs` permanece testável com stdin baseado em `Cursor` em testes unitários/integração, já que `is_terminal()` nunca é chamado internamente.
  - **-** `cmd_edit` e seus helpers carregam um parâmetro adicional `stdin_is_tty: bool` encadeado desde `main.rs` através de `lib.rs::run()`.
- **Alternatives considered**:
  1. **Chamar `is_terminal()` diretamente dentro de `edit.rs`.** Rejeitado: acopla a lógica do comando a um handle real de stdin e quebra testes baseados em `Cursor` que não respaldam um terminal real.
  2. **Confiar em um wrapper externo de `timeout` como única salvaguarda (status quo).** Rejeitado: bloquear silenciosamente sem erro acionável é uma experiência ruim em pipeline de agentes; a assinatura exit 143 não dá pista sobre a causa raiz.
  3. **Perguntar interativamente quando um terminal for detectado.** Rejeitado: `atomwrite` é projetado para uso não interativo/por agentes; um prompt interativo reintroduziria comportamento de bloqueio em outra forma.
- **Trigger to revisit**: Se um subcomando que não é `edit` ganhar um novo modo consumidor de stdin, propagar `stdin_is_tty` da mesma forma em vez de chamar `is_terminal()` localmente.
