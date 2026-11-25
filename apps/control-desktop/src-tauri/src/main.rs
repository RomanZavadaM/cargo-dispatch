#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cargo_dispatch_control_desktop::run();
}
