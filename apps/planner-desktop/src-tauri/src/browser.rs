//! Opens the route in a map service with the operator's own browser.

use super::*;
use cargo_dispatch_planner_ui_contracts::{OpenRouteInput, OpenedRoute};
use std::process::Command;

#[cfg(target_os = "windows")]
fn opener(url: &str) -> Command {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let mut command = Command::new(Path::new(&root).join("System32").join("rundll32.exe"));
    // The address is one argument; no shell reads it.
    command.args(["url.dll,FileProtocolHandler", url]);
    command
}

#[cfg(target_os = "macos")]
fn opener(url: &str) -> Command {
    let mut command = Command::new("/usr/bin/open");
    command.arg(url);
    command
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn opener(url: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(url);
    command
}

/// Whether the system took the request. The window never chooses the address:
/// the service builds it from the route.
fn open(url: &str) -> bool {
    match opener(url).spawn() {
        Ok(mut child) => {
            // Collects the finished helper process.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            true
        }
        Err(_) => false,
    }
}

#[tauri::command]
pub(crate) fn planner_open_route(
    input: OpenRouteInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<OpenedRoute, String> {
    let url = state.route_link(input.service)?;
    let opened = open(&url);
    Ok(OpenedRoute { url, opened })
}
