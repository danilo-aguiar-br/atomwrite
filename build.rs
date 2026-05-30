fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=ATOMWRITE_GIT_SHA={sha}");
        }
    }

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=TARGET={target}");
}
