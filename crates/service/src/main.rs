//! Headless Claudeometer: polls claude.ai usage limits in the background and
//! serves the latest reading over a small local HTTP API, so it can be left
//! running on a server with no desktop session. Primary use case: a coding
//! agent curls `GET /usage` before starting new work, to stop cleanly ahead
//! of getting cut off — see docs/SERVICE.md.

mod api;
mod config;
mod poller;
mod run;
mod status;
mod svc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claudeometer-service", version, about = "Headless Claude.ai usage watcher")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Store a claude.ai session key (DevTools -> Application -> Cookies -> sessionKey)
    Login { session_key: String },
    /// Remove the stored session key
    Logout,
    /// Print a quick usage summary (reads a running service if there is one)
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Run in the foreground: starts the poller + HTTP API and blocks.
    /// This is what `install` registers with the OS service manager.
    Run {
        /// Override the configured bind address, e.g. 0.0.0.0:7842
        #[arg(long)]
        bind: Option<String>,
        /// Override the configured poll interval, in seconds
        #[arg(long)]
        interval: Option<u64>,
        /// Internal: set by the Windows Service Control Manager. Don't pass this by hand.
        #[arg(long, hide = true)]
        service: bool,
    },
    /// Self-register and start as a background OS service
    /// (systemd --user on Linux, a LaunchAgent on macOS, a Windows service on Windows)
    Install,
    /// Stop and remove the background OS service
    Uninstall,
    /// Start the already-installed background service
    Start,
    /// Stop the already-installed background service (without uninstalling it)
    Stop,
    /// View or change the config file (~/.config/claudeometer/config.json)
    /// without hand-editing it. Running with no flags just prints it.
    /// Restart the service (`claudeometer-service stop && ... start`) to
    /// pick up changes.
    Config {
        /// Address to bind to, e.g. 127.0.0.1:7842 or 0.0.0.0:7842
        #[arg(long)]
        bind: Option<String>,
        /// Poll interval in seconds
        #[arg(long)]
        interval: Option<u64>,
        /// Set the Authorization: Bearer key required on every request
        #[arg(long)]
        api_key: Option<String>,
        /// Generate and set a random API key instead of typing one
        #[arg(long)]
        generate_api_key: bool,
        /// Remove the API key requirement entirely (only safe on 127.0.0.1)
        #[arg(long)]
        clear_api_key: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Cmd::Login { session_key } => block_on(cmd_login(session_key)),
        Cmd::Logout => cmd_logout(),
        Cmd::Status { json } => block_on(status::run(json)),
        Cmd::Run { bind, interval, service } => {
            let overrides = run::RunOverrides { bind, interval_secs: interval };
            if service {
                #[cfg(windows)]
                {
                    svc::windows::run_as_service_and_block()
                }
                #[cfg(not(windows))]
                {
                    run::run_foreground_blocking(overrides)
                }
            } else {
                run::run_foreground_blocking(overrides)
            }
        }
        Cmd::Install => svc::install(),
        Cmd::Uninstall => svc::uninstall(),
        Cmd::Start => svc::start(),
        Cmd::Stop => svc::stop(),
        Cmd::Config { bind, interval, api_key, generate_api_key, clear_api_key } => {
            cmd_config(bind, interval, api_key, generate_api_key, clear_api_key)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Small ad-hoc runtime for the short-lived async commands (`login`,
/// `status`). `run` builds its own runtime in `run::run_foreground_blocking`
/// so it can also be driven from the (non-async) Windows service callback.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to start async runtime")
        .block_on(fut)
}

async fn cmd_login(session_key: String) -> Result<(), String> {
    let usage = claudeometer_core::fetch_claude_usage(&session_key).await?;
    let backend = config::save_session_key(&session_key)?;
    println!("Signed in — session key stored in {backend}.");
    if let Some(name) = usage.name {
        let email = usage.email.map(|e| format!(" <{e}>")).unwrap_or_default();
        println!("Account: {name}{email}");
    }
    println!("Next: `claudeometer-service install` to run this in the background,");
    println!("or `claudeometer-service status` for a one-off check right now.");
    Ok(())
}

fn cmd_logout() -> Result<(), String> {
    config::delete_session_key();
    println!("Signed out — stored session key removed.");
    Ok(())
}

fn cmd_config(
    bind: Option<String>,
    interval: Option<u64>,
    api_key: Option<String>,
    generate_api_key: bool,
    clear_api_key: bool,
) -> Result<(), String> {
    let mut cfg = config::load_config();
    let mut changed = false;

    if let Some(b) = bind {
        cfg.bind = b;
        changed = true;
    }
    if let Some(i) = interval {
        cfg.poll_interval_secs = i;
        changed = true;
    }
    if clear_api_key {
        cfg.api_key = None;
        changed = true;
    } else if generate_api_key {
        cfg.api_key = Some(generate_api_key_value());
        changed = true;
    } else if let Some(k) = api_key {
        cfg.api_key = Some(k);
        changed = true;
    }

    if changed {
        config::save_config(&cfg)?;
    }

    println!("bind:                {}", cfg.bind);
    println!("poll_interval_secs:  {}", cfg.poll_interval_secs);
    match &cfg.api_key {
        Some(k) => println!("api_key:             {k}"),
        None => println!("api_key:             (none — auth disabled, keep bind on 127.0.0.1)"),
    }
    if changed {
        println!("\nSaved. Restart the service to pick this up:");
        println!("  claudeometer-service stop && claudeometer-service start");
    }
    Ok(())
}

fn generate_api_key_value() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
