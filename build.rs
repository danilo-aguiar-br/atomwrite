fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // rust_i18n::i18n!("locales") embeds translations at compile time. Cargo does
    // not always see those file deps through the proc-macro alone (see longbridge
    // rust-i18n#46); declare them so locale edits invalidate the binary.
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-changed=locales/en.toml");
    println!("cargo:rerun-if-changed=locales/pt-BR.toml");

    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        && output.status.success()
    {
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("cargo:rustc-env=ATOMWRITE_GIT_SHA={sha}");
    }

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=TARGET={target}");
}
