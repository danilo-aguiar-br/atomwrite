fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // rust_i18n::i18n!("locales") embeds translations at compile time. Cargo does
    // not always see those file deps through the proc-macro alone (see longbridge
    // rust-i18n#46); declare them so locale edits invalidate the binary.
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-changed=locales/en.toml");
    println!("cargo:rerun-if-changed=locales/pt-BR.toml");

    // G-032: embed short SHA; append `-dirty` when the worktree has uncommitted changes
    // so operators never confuse a dirty 0.1.35 tree with a clean release tag.
    let mut sha = if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        && output.status.success()
    {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        "unknown".to_string()
    };

    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some_and(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty());
    if dirty && sha != "unknown" {
        sha.push_str("-dirty");
    }
    // Rebuild when tree dirtiness may change (best-effort; cargo may still cache).
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
    println!("cargo:rustc-env=ATOMWRITE_GIT_SHA={sha}");

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=TARGET={target}");
}
