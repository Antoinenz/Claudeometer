fn main() {
    // Compute the display version at compile time.
    // In CI (GITHUB_REF_NAME is set by GitHub Actions), use the Cargo package version
    // which the release workflow has already patched from the git tag.
    // In local dev builds, show the short git commit hash, or "Development" as fallback.
    let version = if std::env::var("GITHUB_REF_NAME").is_ok() {
        format!("v{}", env!("CARGO_PKG_VERSION"))
    } else {
        let hash = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        if hash.is_empty() {
            "Development".to_string()
        } else {
            format!("dev-{hash}")
        }
    };

    println!("cargo:rustc-env=APP_VERSION={version}");
    // Rebuild when the checked-out commit changes.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    tauri_build::build()
}
