# ADR-0.1.35 — G-035: OS environment probe allowlist for `env_detect`

## Status

Superseded in 0.1.37: no environment variables are read. `src/env_detect.rs` now uses file probes only (`/.dockerenv`, cgroup, `/proc/version`, `/.flatpak-info`, the Kubernetes service-account mount, the Termux prefix, the executable path), and the `ci`, `github_actions` and `sudo` flags were removed from `doctor`. The text below is kept as the record of the 0.1.35 decision.

Accepted (2026-07-19)

## Context

Project rules forbid product runtime configuration via environment variables
(`ATOMWRITE_*`, clap `env =`, etc.). Configuration must be CLI flags + XDG
`config.toml`.

`atomwrite doctor` still needs to report whether the host is WSL, a container,
CI, Flatpak, Snap, etc. Those signals are conventionally exposed as OS/host
environment variables set by the platform, not by the product.

## Decision

1. Product knobs: never read process env for atomwrite configuration.
2. Doctor diagnostics only: `src/env_detect.rs` may read a fixed allowlist
   of host variables: `CI`, `GITHUB_ACTIONS`, `WSL_DISTRO_NAME`, `WSL_INTEROP`,
   `container`, `CONTAINER`, `KUBERNETES_SERVICE_HOST`, `PREFIX`, `FLATPAK_ID`,
   `SNAP`, `SUDO_USER`.
3. Prefer filesystem probes when equivalent (A-007): `/.dockerenv`, cgroup,
   `/proc/sys/kernel/osrelease` (WSL), `/.flatpak-info`, `/proc/self/exe` under
   `/snap/`, `/var/run/secrets/kubernetes.io/serviceaccount`, Termux data path.
4. Residual env allowlist only where FS is insufficient (`CI`, `GITHUB_ACTIONS`,
   `SUDO_USER`, and fallbacks for the above).
5. Document the allowlist in the module rustdoc (G-035).

## Consequences
- Grep for `std::env::var` will still hit `env_detect` — that is intentional and
  scoped.
- Compliance audits must distinguish product knobs vs OS probes.
- No network, no secrets, no telemetry export from these reads.
