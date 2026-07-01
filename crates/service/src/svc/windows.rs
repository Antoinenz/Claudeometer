//! Windows: a real Windows Service (registered with the Service Control
//! Manager), not just a Task Scheduler login-trigger — so it runs on a
//! Windows Server with nobody logged in, matching how systemd/launchd behave
//! on the other platforms.
//!
//! NOTE: this module cannot be built or exercised on the Linux sandbox this
//! was developed on (there is no Windows target available here). It follows
//! the `windows-service` crate's documented patterns closely, but treat it
//! as needing a real smoke test on Windows before you rely on it.

use std::env;
use std::ffi::OsString;
use windows_service::service::{
    ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

const SERVICE_NAME: &str = "Claudeometer";
const DISPLAY_NAME: &str = "Claudeometer Usage Service";

pub fn install() -> Result<(), String> {
    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
            .map_err(|e| e.to_string())?;
    let exe = env::current_exe().map_err(|e| format!("couldn't resolve current executable: {e}"))?;

    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe,
        // `--service` tells main() to dispatch into the SCM callback instead
        // of running as a normal foreground process.
        launch_arguments: vec![OsString::from("run"), OsString::from("--service")],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };

    let service = manager
        .create_service(&info, ServiceAccess::START | ServiceAccess::CHANGE_CONFIG)
        .map_err(|e| e.to_string())?;
    service.start(&[] as &[&str]).map_err(|e| e.to_string())?;

    println!("Installed and started the '{SERVICE_NAME}' Windows service.");
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| e.to_string())?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::STOP | ServiceAccess::DELETE)
        .map_err(|e| e.to_string())?;
    let _ = service.stop();
    service.delete().map_err(|e| e.to_string())?;
    println!("Removed the '{SERVICE_NAME}' Windows service.");
    Ok(())
}

pub fn start() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| e.to_string())?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::START)
        .map_err(|e| e.to_string())?;
    service.start(&[] as &[&str]).map_err(|e| e.to_string())
}

pub fn stop() -> Result<(), String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| e.to_string())?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::STOP)
        .map_err(|e| e.to_string())?;
    service.stop().map_err(|e| e.to_string())?;
    Ok(())
}

windows_service::define_windows_service!(ffi_service_main, service_main);

/// Runs on the SCM's dispatcher thread (not async, not inside any Tokio
/// runtime) — hands off to the same blocking entry point a foreground
/// `claudeometer-service run` uses.
fn service_main(_arguments: Vec<OsString>) {
    let overrides = crate::run::RunOverrides { bind: None, interval_secs: None };
    if let Err(e) = crate::run::run_foreground_blocking(overrides) {
        eprintln!("claudeometer-service (Windows service mode) exited with error: {e}");
    }
}

/// Blocks forever, dispatching SCM control requests to `service_main`.
/// Called from `main()` when invoked as `claudeometer-service run --service`.
pub fn run_as_service_and_block() -> Result<(), String> {
    windows_service::service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .map_err(|e| e.to_string())
}
