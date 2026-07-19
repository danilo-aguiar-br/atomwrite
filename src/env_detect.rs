// SPDX-License-Identifier: MIT OR Apache-2.0

//! Runtime environment autodetect (WSL, container, CI, Termux, Snap, Flatpak).
//!
//! Rules Rust multiplataforma — specialized environments must be detected once
//! and exposed to diagnostics (`atomwrite doctor`) without scattering `cfg` or
//! ad-hoc env reads through business logic.

use std::path::Path;

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
    /// Kubernetes pod (`KUBERNETES_SERVICE_HOST` set).
    pub kubernetes: bool,
    /// Continuous integration host (`CI` or `GITHUB_ACTIONS`).
    pub ci: bool,
    /// Android Termux (`PREFIX` contains `com.termux`).
    pub termux: bool,
    /// Flatpak sandbox (`FLATPAK_ID` set).
    pub flatpak: bool,
    /// Snap confinement (`SNAP` set).
    pub snap: bool,
    /// Elevated via sudo (`SUDO_USER` set).
    pub sudo: bool,
}

/// Detect specialized runtime environment (pure reads; no I/O side effects
/// beyond optional `/proc` and `/.dockerenv` probes on Unix).
pub fn detect() -> RuntimeEnvironment {
    detect_with(
        EnvProbe {
            wsl_distro: std::env::var_os("WSL_DISTRO_NAME"),
            wsl_interop: std::env::var_os("WSL_INTEROP"),
            container_env: std::env::var_os("container"),
            container_upper: std::env::var_os("CONTAINER"),
            kubernetes: std::env::var_os("KUBERNETES_SERVICE_HOST"),
            ci: std::env::var_os("CI"),
            github_actions: std::env::var_os("GITHUB_ACTIONS"),
            prefix: std::env::var_os("PREFIX"),
            flatpak_id: std::env::var_os("FLATPAK_ID"),
            snap: std::env::var_os("SNAP"),
            sudo_user: std::env::var_os("SUDO_USER"),
        },
        ProbeFs::Live,
    )
}

/// Injectable environment snapshot for unit tests (no process-global mutation).
#[derive(Default)]
struct EnvProbe {
    wsl_distro: Option<std::ffi::OsString>,
    wsl_interop: Option<std::ffi::OsString>,
    container_env: Option<std::ffi::OsString>,
    container_upper: Option<std::ffi::OsString>,
    kubernetes: Option<std::ffi::OsString>,
    ci: Option<std::ffi::OsString>,
    github_actions: Option<std::ffi::OsString>,
    prefix: Option<std::ffi::OsString>,
    flatpak_id: Option<std::ffi::OsString>,
    snap: Option<std::ffi::OsString>,
    sudo_user: Option<std::ffi::OsString>,
}

enum ProbeFs {
    /// Read `/proc` and `/.dockerenv` on Unix.
    Live,
    /// Unit tests: skip filesystem probes; only env-based flags apply.
    #[cfg(test)]
    None,
}

fn detect_with(probe: EnvProbe, fs: ProbeFs) -> RuntimeEnvironment {
    RuntimeEnvironment {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        family: std::env::consts::FAMILY,
        target: option_env!("TARGET").unwrap_or("unknown").to_string(),
        wsl: is_wsl(&probe, &fs),
        container: is_container(&probe, &fs),
        kubernetes: nonempty(&probe.kubernetes),
        ci: nonempty(&probe.ci) || nonempty(&probe.github_actions),
        termux: is_termux(&probe),
        flatpak: nonempty(&probe.flatpak_id),
        snap: nonempty(&probe.snap),
        sudo: nonempty(&probe.sudo_user),
    }
}

fn nonempty(v: &Option<std::ffi::OsString>) -> bool {
    v.as_ref().is_some_and(|s| !s.is_empty())
}

fn is_wsl(probe: &EnvProbe, fs: &ProbeFs) -> bool {
    if nonempty(&probe.wsl_distro) || nonempty(&probe.wsl_interop) {
        return true;
    }
    match fs {
        ProbeFs::Live => {
            #[cfg(unix)]
            {
                if let Ok(release) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
                    let lower = release.to_ascii_lowercase();
                    if lower.contains("microsoft") || lower.contains("wsl") {
                        return true;
                    }
                }
                if let Ok(version) = std::fs::read_to_string("/proc/version") {
                    let lower = version.to_ascii_lowercase();
                    if lower.contains("microsoft") || lower.contains("wsl") {
                        return true;
                    }
                }
            }
            false
        }
        #[cfg(test)]
        ProbeFs::None => false,
    }
}

fn is_container(probe: &EnvProbe, fs: &ProbeFs) -> bool {
    if nonempty(&probe.container_env) || nonempty(&probe.container_upper) {
        return true;
    }
    match fs {
        ProbeFs::Live => {
            if Path::new("/.dockerenv").exists() {
                return true;
            }
            #[cfg(unix)]
            {
                if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
                    let lower = cgroup.to_ascii_lowercase();
                    if lower.contains("docker")
                        || lower.contains("kubepods")
                        || lower.contains("containerd")
                        || lower.contains("libpod")
                    {
                        return true;
                    }
                }
            }
            false
        }
        #[cfg(test)]
        ProbeFs::None => false,
    }
}

fn is_termux(probe: &EnvProbe) -> bool {
    probe
        .prefix
        .as_ref()
        .and_then(|p| p.to_str())
        .is_some_and(|p| p.contains("com.termux"))
}

/// Human-readable one-line summary for doctor / logs.
pub fn summary(env: &RuntimeEnvironment) -> String {
    let mut flags = Vec::new();
    if env.wsl {
        flags.push("wsl");
    }
    if env.container {
        flags.push("container");
    }
    if env.kubernetes {
        flags.push("kubernetes");
    }
    if env.ci {
        flags.push("ci");
    }
    if env.termux {
        flags.push("termux");
    }
    if env.flatpak {
        flags.push("flatpak");
    }
    if env.snap {
        flags.push("snap");
    }
    if env.sudo {
        flags.push("sudo");
    }
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
    use std::ffi::OsString;

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
    fn ci_flag_from_probe() {
        let env = detect_with(
            EnvProbe {
                ci: Some(OsString::from("true")),
                ..EnvProbe::default()
            },
            ProbeFs::None,
        );
        assert!(env.ci);
        assert!(!env.termux);
    }

    #[test]
    fn termux_from_prefix_probe() {
        let env = detect_with(
            EnvProbe {
                prefix: Some(OsString::from("/data/data/com.termux/files/usr")),
                ..EnvProbe::default()
            },
            ProbeFs::None,
        );
        assert!(env.termux);
    }

    #[test]
    fn flatpak_snap_wsl_kubernetes_sudo_from_probe() {
        let env = detect_with(
            EnvProbe {
                flatpak_id: Some(OsString::from("com.example.App")),
                snap: Some(OsString::from("/snap/atomwrite/current")),
                wsl_distro: Some(OsString::from("Ubuntu")),
                kubernetes: Some(OsString::from("10.0.0.1")),
                sudo_user: Some(OsString::from("alice")),
                ..EnvProbe::default()
            },
            ProbeFs::None,
        );
        assert!(env.flatpak);
        assert!(env.snap);
        assert!(env.wsl);
        assert!(env.kubernetes);
        assert!(env.sudo);
        let s = summary(&env);
        assert!(s.contains("flatpak"), "{s}");
        assert!(s.contains("wsl"), "{s}");
    }

    #[test]
    fn container_from_env_probe() {
        let env = detect_with(
            EnvProbe {
                container_env: Some(OsString::from("docker")),
                ..EnvProbe::default()
            },
            ProbeFs::None,
        );
        assert!(env.container);
    }
}
