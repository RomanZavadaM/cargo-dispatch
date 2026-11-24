use std::sync::Mutex;
use tauri::State;
use ui_probe_core::{ApplicationService, ProbeSummary};

struct ProbeState(Mutex<ApplicationService>);

#[tauri::command]
fn probe_summary(state: State<'_, ProbeState>) -> Result<ProbeSummary, String> {
    state
        .0
        .lock()
        .map(|service| service.summary())
        .map_err(|_| "synthetic application state lock is poisoned".to_owned())
}

fn main() {
    tauri::Builder::default()
        .manage(ProbeState(Mutex::new(ApplicationService::synthetic())))
        .invoke_handler(tauri::generate_handler![probe_summary])
        .run(tauri::generate_context!())
        .expect("failed to run Cargo dispatch UI probe");
}
