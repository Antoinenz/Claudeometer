//! Self-install as a background OS service. One module per platform's
//! service manager; `main.rs` calls the plain functions below and never
//! needs to know which OS it's running on.

#[cfg(target_os = "linux")]
mod systemd;
#[cfg(target_os = "macos")]
mod launchd;
#[cfg(windows)]
pub mod windows;

pub fn install() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return systemd::install();
    #[cfg(target_os = "macos")]
    return launchd::install();
    #[cfg(windows)]
    return windows::install();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    Err("Self-install isn't supported on this platform yet — run `claudeometer-service run` \
         directly under whatever process supervisor is available.".to_string())
}

pub fn uninstall() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return systemd::uninstall();
    #[cfg(target_os = "macos")]
    return launchd::uninstall();
    #[cfg(windows)]
    return windows::uninstall();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    Err("Self-install isn't supported on this platform.".to_string())
}

pub fn start() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return systemd::start();
    #[cfg(target_os = "macos")]
    return launchd::start();
    #[cfg(windows)]
    return windows::start();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    Err("Self-install isn't supported on this platform.".to_string())
}

pub fn stop() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return systemd::stop();
    #[cfg(target_os = "macos")]
    return launchd::stop();
    #[cfg(windows)]
    return windows::stop();
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    Err("Self-install isn't supported on this platform.".to_string())
}

/// Shared helper: run a service-manager CLI command and turn a non-zero
/// exit into an `Err`.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let status = std::process::Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run `{cmd}`: {e}"))?;
    if !status.success() {
        return Err(format!("`{cmd} {}` exited with {status}", args.join(" ")));
    }
    Ok(())
}
