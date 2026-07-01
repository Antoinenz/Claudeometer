//! macOS: a per-user LaunchAgent (`~/Library/LaunchAgents`), loaded with the
//! modern `launchctl bootstrap`/`bootout` domain-target syntax rather than
//! the deprecated `load -w`/`unload`.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const LABEL: &str = "com.claudeometer.service";

fn plist_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library/LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

fn uid() -> Result<String, String> {
    let out = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|e| format!("failed to run `id -u`: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn domain_target() -> Result<String, String> {
    Ok(format!("gui/{}", uid()?))
}

pub fn install() -> Result<(), String> {
    let exe = env::current_exe().map_err(|e| format!("couldn't resolve current executable: {e}"))?;
    let dir = plist_path().parent().unwrap().to_path_buf();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>run</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/tmp/claudeometer-service.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/claudeometer-service.log</string>
</dict>
</plist>
"#,
        exe = exe.display(),
    );
    fs::write(plist_path(), plist).map_err(|e| e.to_string())?;

    let target = domain_target()?;
    // Ignore "already bootstrapped" failures from a previous install.
    let _ = super::run("launchctl", &["bootout", &format!("{target}/{LABEL}")]);
    super::run("launchctl", &["bootstrap", &target, &plist_path().to_string_lossy()])?;
    super::run("launchctl", &["enable", &format!("{target}/{LABEL}")])?;

    println!("Installed and started as a LaunchAgent ({}).", plist_path().display());
    println!("Logs: /tmp/claudeometer-service.log");
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let target = domain_target()?;
    let _ = super::run("launchctl", &["bootout", &format!("{target}/{LABEL}")]);
    let path = plist_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    println!("Removed the LaunchAgent.");
    Ok(())
}

pub fn start() -> Result<(), String> {
    let target = domain_target()?;
    super::run("launchctl", &["kickstart", "-k", &format!("{target}/{LABEL}")])
}

pub fn stop() -> Result<(), String> {
    let target = domain_target()?;
    super::run("launchctl", &["kill", "SIGTERM", &format!("{target}/{LABEL}")])
}
