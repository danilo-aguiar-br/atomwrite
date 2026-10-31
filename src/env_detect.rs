// SPDX-License-Identifier: MIT OR Apache-2.0

//! Runtime environment autodetect (WSL, container, Kubernetes, Termux, Snap, Flatpak).
//!
//! Cross-platform rules: specialized environments are detected once and
//! exposed to diagnostics (`atomwrite doctor`) without scattering `cfg`
//! through business logic.
//!
//! Detection reads **files only** (`/.dockerenv`, `/proc/*`, `/.flatpak-info`,
//! the Kubernetes service-account mount). No process environment variable is
//! read: product configuration is CLI flags plus XDG `config.toml`, and these
//! values never feed write/delete/watch/timeout/retry policy.
//!
//! On Windows every probe is skipped and only the compile-time and
//! `std::env::consts` facts are reported.

use serde::Serialize;

/// Snapshot of specialized runtime environment flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeEnvironment {
    /// Host OS family string from `std::env::consts::OS`.
    pub os: &'static str,
    /// CPU architecture from `std::env::consts::ARCH`.
    pub arch: &'static str,
    /// OS family (`unix` / `windows`) from `std::env::consts::FAMILY`.
    pub family: &'static str,
    /// Compile-time Rust target triple (from `build.rs` `TARGET` env).
    pub target: String,
    /// Running under Windows Subsystem for Linux (WSL1 or WSL2).
    pub wsl: bool,
    /// Docker / containerd style container (`.dockerenv` or cgroup hint).
    pub container: bool,
    /// Kubernetes pod (service-account secrets mounted).
    pub kubernetes: bool,
    /// Android Termux (`/data/data/com.termux/files/usr` exists).
    pub termux: bool,
    /// Flatpak sandbox (`/.flatpak-info` exists).
    pub flatpak: bool,
    /// Snap confinement (this executable runs from `/snap/`).
    pub snap: bool,
}

/// Raw facts read from the host filesystem. Tests build it by hand so no
/// real file is touched.
#[derive(Default)]
struct HostFacts {
    dockerenv: bool,
    cgroup: Option<String>,
    osrelease: Option<String>,
    version: Option<String>,
    flatpak_info: bool,
    kubernetes_secrets: bool,
    termux_usr: bool,
    exe: Option<std::path::PathBuf>,
}

impl HostFacts {
    #[cfg(unix)]
    fn read() -> Self {
        use std::path::Path;

        use crate::constants as C;
        let text = |p: &str| std::fs::read_to_string(p).ok();
        Self {
            dockerenv: Path::new(C::DOCKERENV_PATH).exists(),
            cgroup: text(C::PROC_INIT_CGROUP_PATH).or_else(|| text(C::PROC_SELF_CGROUP_PATH)),
            osrelease: text(C::PROC_OSRELEASE_PATH),
            version: text(C::PROC_VERSION_PATH),
            flatpak_info: Path::new(C::FLATPAK_INFO_PATH).is_file(),
            kubernetes_secrets: Path::new(C::KUBERNETES_SECRETS_DIR).is_dir(),
            termux_usr: Path::new(C::TERMUX_USR_DIR).is_dir(),
            exe: std::fs::read_link(C::PROC_SELF_EXE_PATH).ok(),
        }
    }

    #[cfg(not(unix))]
    fn read() -> Self {
        Self::default()
    }
}

/// Detect specialized runtime environment (file probes only; no env reads).
pub fn detect() -> RuntimeEnvironment {
    detect_with(&HostFacts::read())
}

fn detect_with(facts: &HostFacts) -> RuntimeEnvironment {
    RuntimeEnvironment {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        family: std::env::consts::FAMILY,
        target: option_env!("TARGET").unwrap_or("unknown").to_string(),
        wsl: is_wsl(facts),
        container: is_container(facts),
        kubernetes: facts.kubernetes_secrets,
        termux: facts.termux_usr,
        flatpak: facts.flatpak_info,
        snap: facts.exe.as_ref().is_some_and(|e| e.starts_with("/snap/")),
    }
}

fn is_wsl(facts: &HostFacts) -> bool {
    [&facts.osrelease, &facts.version]
        .into_iter()
        .flatten()
        .any(|text| text.to_ascii_lowercase().contains("microsoft"))
}

fn is_container(facts: &HostFacts) -> bool {
    facts.dockerenv
        || facts.cgroup.as_ref().is_some_and(|cgroup| {
            let lower = cgroup.to_ascii_lowercase();
            crate::constants::CONTAINER_CGROUP_MARKERS
                .iter()
                .any(|m| lower.contains(m))
        })
}

/// Human-readable one-line summary for doctor / logs.
pub fn summary(env: &RuntimeEnvironment) -> String {
    let flags: Vec<&str> = [
        (env.wsl, "wsl"),
        (env.container, "container"),
        (env.kubernetes, "kubernetes"),
        (env.termux, "termux"),
        (env.flatpak, "flatpak"),
        (env.snap, "snap"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect();
    if flags.is_empty() {
        format!(
            "os={} arch={} family={} target={} (no specialized runtime flags)",
            env.os, env.arch, env.family, env.target
        )
    } else {
        format!(
            "os={} arch={} family={} target={} flags={}",
            env.os,
            env.arch,
            env.family,
            env.target,
            flags.join(",")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_host_consts() {
        let env = detect();
        assert!(!env.os.is_empty());
        assert!(!env.arch.is_empty());
        assert!(!env.family.is_empty());
        assert!(!env.target.is_empty());
    }

    #[test]
    fn summary_includes_os() {
        let env = detect();
        let s = summary(&env);
        assert!(s.contains(env.os), "summary={s}");
    }

    #[test]
    fn empty_facts_report_no_flags() {
        let env = detect_with(&HostFacts::default());
        assert!(!env.wsl && !env.container && !env.kubernetes);
        assert!(!env.termux && !env.flatpak && !env.snap);
        assert!(summary(&env).contains("no specialized runtime flags"));
    }

    #[test]
    fn termux_from_filesystem_fact() {
        let env = detect_with(&HostFacts {
            termux_usr: true,
            ..HostFacts::default()
        });
        assert!(env.termux);
        assert!(!env.flatpak);
    }

    #[test]
    fn flatpak_snap_wsl_kubernetes_from_filesystem_facts() {
        let env = detect_with(&HostFacts {
            flatpak_info: true,
            exe: Some("/snap/atomwrite/current/bin/atomwrite".into()),
            version: Some("Linux version 5.15 (Microsoft@Microsoft.com)".into()),
            kubernetes_secrets: true,
            ..HostFacts::default()
        });
        assert!(env.flatpak);
        assert!(env.snap);
        assert!(env.wsl);
        assert!(env.kubernetes);
        let s = summary(&env);
        assert!(s.contains("flatpak"), "{s}");
        assert!(s.contains("wsl"), "{s}");
    }

    #[test]
    fn container_from_dockerenv_or_cgroup() {
        let by_file = detect_with(&HostFacts {
            dockerenv: true,
            ..HostFacts::default()
        });
        assert!(by_file.container);
        let by_cgroup = detect_with(&HostFacts {
            cgroup: Some("0::/kubepods/burstable/pod1".into()),
            ..HostFacts::default()
        });
        assert!(by_cgroup.container);
        let host = detect_with(&HostFacts {
            cgroup: Some("0::/user.slice/session-1.scope".into()),
            ..HostFacts::default()
        });
        assert!(!host.container);
    }
}
