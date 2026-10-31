# ADR-0.1.35 — G-035: allowlist de sondagem do ambiente do SO para `env_detect`

## Status

Superado na 0.1.37: nenhuma variável de ambiente é lida. O `src/env_detect.rs` agora usa apenas sondagens de arquivo (`/.dockerenv`, cgroup, `/proc/version`, `/.flatpak-info`, o mount da service-account do Kubernetes, o prefixo do Termux, o caminho do executável), e as flags `ci`, `github_actions` e `sudo` foram removidas do `doctor`. O texto abaixo é mantido como registro da decisão da 0.1.35.

Aceito (2026-07-19)

## Contexto

As regras do projeto proíbem configuração de runtime do produto por variáveis de ambiente
(`ATOMWRITE_*`, `env =` do clap etc.). A configuração deve ser flags de CLI + `config.toml`
XDG.

O `atomwrite doctor` ainda precisa informar se o host é WSL, contêiner,
CI, Flatpak, Snap etc. Esses sinais são convencionalmente expostos como variáveis de
ambiente do SO/host, definidas pela plataforma, não pelo produto.

## Decisão

1. Knobs do produto: nunca ler o env do processo para configurar o atomwrite.
2. Apenas diagnóstico do doctor: o `src/env_detect.rs` pode ler uma allowlist fixa
   de variáveis do host: `CI`, `GITHUB_ACTIONS`, `WSL_DISTRO_NAME`, `WSL_INTEROP`,
   `container`, `CONTAINER`, `KUBERNETES_SERVICE_HOST`, `PREFIX`, `FLATPAK_ID`,
   `SNAP`, `SUDO_USER`.
3. Preferir sondagens do sistema de arquivos quando equivalentes (A-007): `/.dockerenv`, cgroup,
   `/proc/sys/kernel/osrelease` (WSL), `/.flatpak-info`, `/proc/self/exe` sob
   `/snap/`, `/var/run/secrets/kubernetes.io/serviceaccount`, caminho de dados do Termux.
4. Allowlist residual de env só onde o sistema de arquivos é insuficiente (`CI`, `GITHUB_ACTIONS`,
   `SUDO_USER` e os fallbacks dos itens acima).
5. Documentar a allowlist no rustdoc do módulo (G-035).

## Consequências
- Um grep por `std::env::var` ainda vai achar o `env_detect`, e isso é intencional e
  delimitado.
- Auditorias de conformidade precisam distinguir knobs do produto de sondagens do SO.
- Sem rede, sem segredos, sem exportação de telemetria a partir dessas leituras.
