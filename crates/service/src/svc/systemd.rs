//! Linux: a systemd **user** unit. Deliberately per-user (not a system-wide
//! unit in /etc/systemd/system) so `install` never needs root/sudo — it only
//! touches files under the invoking user's own `$XDG_CONFIG_HOME`.

use std::env;
use std::fs;
use std::path::PathBuf;

const UNIT_NAME: &str = "claudeometer.service";

fn unit_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("systemd/user")
}

fn unit_path() -> PathBuf {
    unit_dir().join(UNIT_NAME)
}

pub fn install() -> Result<(), String> {
    let exe = env::current_exe().map_err(|e| format!("couldn't resolve current executable: {e}"))?;
    fs::create_dir_all(unit_dir()).map_err(|e| e.to_string())?;

    let unit = format!(
        "[Unit]\n\
         Description=Claudeometer headless usage service\n\
         After=network-online.target\n\
         Wants=network-online.target\n\
         \n\
         [Service]\n\
         ExecStart={} run\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exe.display()
    );
    fs::write(unit_path(), unit).map_err(|e| e.to_string())?;

    super::run("systemctl", &["--user", "daemon-reload"])?;
    super::run("systemctl", &["--user", "enable", "--now", UNIT_NAME])?;

    println!("Installed and started as a systemd --user service ({}).", unit_path().display());
    println!(
        "If this account has no active login session (a true headless box), also run:\n  \
         loginctl enable-linger {}\n\
         so the service keeps running after you log out.",
        env::var("USER").unwrap_or_else(|_| "<your-username>".to_string())
    );
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let _ = super::run("systemctl", &["--user", "disable", "--now", UNIT_NAME]);
    let path = unit_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    let _ = super::run("systemctl", &["--user", "daemon-reload"]);
    println!("Removed the systemd --user service.");
    Ok(())
}

pub fn start() -> Result<(), String> {
    super::run("systemctl", &["--user", "start", UNIT_NAME])
}

pub fn stop() -> Result<(), String> {
    super::run("systemctl", &["--user", "stop", UNIT_NAME])
}
