# ADR-0050: guarda de stdin-tty para os modos do edit que consomem stdin

- Status: Aceito
- Data: 2026-07-06
- Contexto: Os modos do `edit` que consomem stdin (`--after-line`, `--before-line`, `--range`, `--after-match`, `--before-match`, `--between` e `--multi`) leem o novo conteúdo do stdin. Quando o stdin era um terminal (sessão interativa, sem pipe nem redirecionamento), esses modos bloqueavam indefinidamente esperando uma entrada que o usuário nunca pretendeu digitar, exigindo antes um wrapper externo de `timeout` e saindo com exit 143 (SIGTERM) em vez de falhar com um erro acionável.
- Decisão: O `main.rs` calcula `stdin_is_tty = stdin.is_terminal()` uma única vez, usando o trait `IsTerminal` da biblioteca padrão (estável desde o Rust 1.70), e o passa como um `bool` simples por `atomwrite::run()` até o `cmd_edit` e seus helpers que consomem stdin (`edit_by_line`, `edit_by_marker` e os handlers de cada modo), que chamam o helper compartilhado `read_stdin_text_guarded(stdin, max_size, stdin_is_tty, mode_name)`. Quando `stdin_is_tty` é `true`, a guarda devolve `AtomwriteError::InvalidInput` (exit 65) com uma `suggestion` acionável, em vez de bloquear. O próprio `is_terminal()` é chamado exatamente uma vez, no `main.rs`, e o `edit.rs` e seus helpers só recebem o `bool` já calculado, nunca chamando `is_terminal()` diretamente. Isso mantém o caminho de código que consome stdin testável sem um terminal real (os testes podem passar `stdin_is_tty: false` com um `Cursor` como stdin).
- Consequências:
  - + Os modos do edit que consomem stdin falham rápido (exit 65) com uma `suggestion` quando invocados de forma interativa, em vez de travar e exigir um `timeout` externo + SIGTERM (exit 143).
  - + O `edit.rs` continua testável com stdin baseado em `Cursor` em testes unitários/de integração, já que o `is_terminal()` nunca é chamado dentro dele.
  - - O `cmd_edit` e seus helpers carregam um parâmetro adicional `stdin_is_tty: bool` repassado do `main.rs` por `lib.rs::run()`.
- Alternativas consideradas:
  1. **Chamar `is_terminal()` diretamente dentro do `edit.rs`.** Rejeitado: acopla a lógica do comando a um handle de stdin real e quebra testes baseados em `Cursor`, que não têm um terminal real por trás.
  2. **Confiar em um wrapper externo de `timeout` como única proteção (status quo).** Rejeitado: bloquear em silêncio, sem erro acionável, é uma experiência ruim em pipelines de agentes, e a assinatura de exit 143 não dá nenhuma pista da causa raiz.
  3. Perguntar interativamente quando um terminal for detectado. Rejeitado: o `atomwrite` é projetado para uso não interativo/por agentes, e um prompt interativo reintroduziria o comportamento de bloqueio de outra forma.
- Gatilho para revisitar: Se um subcomando que não seja `edit` ganhar um novo modo que consome stdin, propagar o `stdin_is_tty` a ele da mesma forma, em vez de chamar `is_terminal()` localmente.
