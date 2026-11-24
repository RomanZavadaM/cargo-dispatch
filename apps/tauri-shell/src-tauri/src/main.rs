use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};
use tauri::{
    AppHandle, Emitter, Manager, State, WebviewUrl,
    webview::WebviewWindowBuilder,
};
use ui_probe_core::{ApplicationService, IntakeMode, LoadRequest, ProbeSummary, SortField};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RequestPageInput {
    filter: String,
    offset: usize,
    limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RequestPage {
    total_filtered: usize,
    offset: usize,
    rows: Vec<LoadRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FieldChange {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct RequestUpdate {
    request_id: u64,
    mode: String,
    changes: Vec<FieldChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ProbeSettings {
    language: String,
    archive_root: String,
    compact_tables: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileChoiceInput {
    purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct FileChoicePlan {
    purpose: String,
    allowed_extensions: Vec<String>,
    platform_adapter_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TripExportInput {
    request_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TripExportPayload {
    file_name: String,
    mime_type: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CandidateCalculationInput {
    request_id: u64,
    vehicle_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CandidateCalculationStarted {
    calculation_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CancelCalculationInput {
    calculation_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CandidateProgress {
    calculation_id: u64,
    request_id: u64,
    percent: u8,
    state: String,
    candidate_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TripSession {
    trip_id: String,
    vehicle: String,
    status: String,
    revision: u64,
    event_count: u64,
    last_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ControlOverview {
    revision: u64,
    trips: Vec<TripSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TripWindowInput {
    trip_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TripStatusUpdate {
    trip_id: String,
    status: String,
    note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TripWindowOpened {
    trip_id: String,
    label: String,
    reused_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TripWindowPlan {
    trip_id: String,
    label: String,
    title: String,
}

#[derive(Debug, Clone)]
struct ControlService {
    revision: u64,
    trips: HashMap<String, TripSession>,
}

impl ControlService {
    fn synthetic() -> Self {
        let trips = [
            TripSession {
                trip_id: "TRIP-A".to_owned(),
                vehicle: "BC 1001 AA".to_owned(),
                status: "driving".to_owned(),
                revision: 1,
                event_count: 4,
                last_note: "Approaching pickup".to_owned(),
            },
            TripSession {
                trip_id: "TRIP-B".to_owned(),
                vehicle: "BC 2002 BB".to_owned(),
                status: "loading".to_owned(),
                revision: 1,
                event_count: 7,
                last_note: "Waiting for documents".to_owned(),
            },
        ]
        .into_iter()
        .map(|trip| (trip.trip_id.clone(), trip))
        .collect();

        Self { revision: 1, trips }
    }

    fn overview(&self) -> ControlOverview {
        let mut trips: Vec<_> = self.trips.values().cloned().collect();
        trips.sort_by(|left, right| left.trip_id.cmp(&right.trip_id));
        ControlOverview {
            revision: self.revision,
            trips,
        }
    }

    fn snapshot(&self, trip_id: &str) -> Result<TripSession, String> {
        self.trips
            .get(trip_id)
            .cloned()
            .ok_or_else(|| "Trip no longer exists".to_owned())
    }

    fn update(&mut self, input: &TripStatusUpdate) -> Result<TripSession, String> {
        if input.status.trim().is_empty() {
            return Err("Trip status cannot be empty".to_owned());
        }

        let trip = self
            .trips
            .get_mut(&input.trip_id)
            .ok_or_else(|| "Trip no longer exists".to_owned())?;
        trip.status = input.status.trim().to_owned();
        trip.last_note = input.note.trim().to_owned();
        trip.revision += 1;
        trip.event_count += 1;
        self.revision += 1;
        Ok(trip.clone())
    }

    fn window_plan(&self, trip_id: &str) -> Result<TripWindowPlan, String> {
        let trip = self.snapshot(trip_id)?;
        let normalized = trip_id
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>();

        Ok(TripWindowPlan {
            trip_id: trip.trip_id.clone(),
            label: format!("trip-{normalized}"),
            title: format!("{} · {}", trip.trip_id, trip.vehicle),
        })
    }
}

struct ProbeState {
    service: Mutex<ApplicationService>,
    settings: Mutex<ProbeSettings>,
    calculations: Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>,
    next_calculation_id: AtomicU64,
    control: Mutex<ControlService>,
}

impl ProbeState {
    fn new() -> Self {
        Self {
            service: Mutex::new(ApplicationService::synthetic()),
            settings: Mutex::new(ProbeSettings {
                language: "uk".to_owned(),
                archive_root: "synthetic-default".to_owned(),
                compact_tables: false,
            }),
            calculations: Arc::new(Mutex::new(HashMap::new())),
            next_calculation_id: AtomicU64::new(1),
            control: Mutex::new(ControlService::synthetic()),
        }
    }
}

fn state_lock_error(area: &str) -> String {
    format!("{area} is temporarily unavailable")
}

fn load_request_page_impl(
    service: &ApplicationService,
    input: &RequestPageInput,
) -> Result<RequestPage, String> {
    if input.limit == 0 || input.limit > 500 {
        return Err("Page size must be between 1 and 500".to_owned());
    }

    let rows = service.query(&input.filter, SortField::Id, true, None);
    let total_filtered = rows.len();
    let page = rows
        .into_iter()
        .skip(input.offset)
        .take(input.limit)
        .collect();

    Ok(RequestPage {
        total_filtered,
        offset: input.offset,
        rows: page,
    })
}

fn apply_request_update(
    service: &mut ApplicationService,
    input: &RequestUpdate,
) -> Result<LoadRequest, String> {
    let mut draft = service
        .editor_snapshot(input.request_id)
        .ok_or_else(|| "Request no longer exists".to_owned())?;

    draft.mode = match input.mode.as_str() {
        "manual" => IntakeMode::Manual,
        "phone" => IntakeMode::Phone,
        _ => return Err("Unknown request input mode".to_owned()),
    };

    for change in &input.changes {
        if !draft.set_field(&change.key, change.value.clone()) {
            return Err(format!("Unknown request field: {}", change.key));
        }
    }

    service.save_draft(&draft).map_err(|issues| {
        issues
            .into_iter()
            .map(|issue| format!("{}: {}", issue.field_key, issue.message))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

fn parse_settings(raw_json: &str) -> Result<ProbeSettings, String> {
    let settings: ProbeSettings =
        serde_json::from_str(raw_json).map_err(|error| format!("Invalid settings: {error}"))?;

    if !matches!(settings.language.as_str(), "uk" | "en" | "de") {
        return Err("Language must be uk, en or de in this probe".to_owned());
    }
    if settings.archive_root.trim().is_empty() {
        return Err("Archive folder cannot be empty".to_owned());
    }

    Ok(settings)
}

fn file_choice_plan(input: &FileChoiceInput) -> Result<FileChoicePlan, String> {
    let allowed_extensions = match input.purpose.as_str() {
        "attach_document" => vec!["pdf", "jpg", "jpeg", "png"],
        "import_request" => vec!["json", "txt", "pdf"],
        "export_folder" => Vec::new(),
        _ => return Err("Unknown file selection purpose".to_owned()),
    };

    Ok(FileChoicePlan {
        purpose: input.purpose.clone(),
        allowed_extensions: allowed_extensions.into_iter().map(str::to_owned).collect(),
        platform_adapter_required: true,
    })
}

fn export_trip_package(
    service: &ApplicationService,
    input: &TripExportInput,
) -> Result<TripExportPayload, String> {
    let request = service
        .get_request(input.request_id)
        .ok_or_else(|| "Request no longer exists".to_owned())?;
    let route = service.route_snapshot();

    let content = serde_json::to_string_pretty(&serde_json::json!({
        "schema_version": "ui-probe-trip-v1",
        "request": request,
        "route": route,
        "source": "desktop-command-boundary",
    }))
    .map_err(|error| format!("Could not serialize trip package: {error}"))?;

    Ok(TripExportPayload {
        file_name: format!("trip-probe-{}.json", input.request_id),
        mime_type: "application/json".to_owned(),
        content,
    })
}

fn run_candidate_work<F>(
    calculation_id: u64,
    request_id: u64,
    vehicle_ids: &[u64],
    cancelled: &AtomicBool,
    delay: Duration,
    mut report: F,
) -> CandidateProgress
where
    F: FnMut(CandidateProgress),
{
    for percent in (0..=100).step_by(10) {
        if cancelled.load(Ordering::Relaxed) {
            let progress = CandidateProgress {
                calculation_id,
                request_id,
                percent,
                state: "cancelled".to_owned(),
                candidate_count: 0,
            };
            report(progress.clone());
            return progress;
        }

        let progress = CandidateProgress {
            calculation_id,
            request_id,
            percent,
            state: if percent == 100 {
                "completed".to_owned()
            } else {
                "running".to_owned()
            },
            candidate_count: if percent == 100 {
                vehicle_ids.len().min(3)
            } else {
                0
            },
        };
        report(progress.clone());

        if percent < 100 && !delay.is_zero() {
            thread::sleep(delay);
        }
    }

    CandidateProgress {
        calculation_id,
        request_id,
        percent: 100,
        state: "completed".to_owned(),
        candidate_count: vehicle_ids.len().min(3),
    }
}

#[tauri::command]
fn control_overview(state: State<'_, ProbeState>) -> Result<ControlOverview, String> {
    state
        .control
        .lock()
        .map(|control| control.overview())
        .map_err(|_| state_lock_error("Control state"))
}

#[tauri::command]
fn trip_snapshot(
    input: TripWindowInput,
    state: State<'_, ProbeState>,
) -> Result<TripSession, String> {
    state
        .control
        .lock()
        .map_err(|_| state_lock_error("Control state"))?
        .snapshot(&input.trip_id)
}

#[tauri::command]
fn update_trip_status(
    input: TripStatusUpdate,
    state: State<'_, ProbeState>,
) -> Result<TripSession, String> {
    state
        .control
        .lock()
        .map_err(|_| state_lock_error("Control state"))?
        .update(&input)
}

#[tauri::command]
async fn open_trip_window(
    input: TripWindowInput,
    app: AppHandle,
    state: State<'_, ProbeState>,
) -> Result<TripWindowOpened, String> {
    let plan = state
        .control
        .lock()
        .map_err(|_| state_lock_error("Control state"))?
        .window_plan(&input.trip_id)?;

    if let Some(window) = app.get_webview_window(&plan.label) {
        window
            .show()
            .map_err(|error| format!("Could not show trip window: {error}"))?;
        window
            .set_focus()
            .map_err(|error| format!("Could not focus trip window: {error}"))?;
        return Ok(TripWindowOpened {
            trip_id: plan.trip_id,
            label: plan.label,
            reused_existing: true,
        });
    }

    let trip_json = serde_json::to_string(&plan.trip_id)
        .map_err(|error| format!("Could not prepare trip window context: {error}"))?;
    let init_script = format!("window.__CARGO_DISPATCH_TRIP_ID__ = {trip_json};");

    WebviewWindowBuilder::new(&app, &plan.label, WebviewUrl::App("index.html".into()))
        .title(&plan.title)
        .inner_size(1100.0, 760.0)
        .resizable(true)
        .initialization_script(init_script)
        .build()
        .map_err(|error| format!("Could not open trip window: {error}"))?;

    Ok(TripWindowOpened {
        trip_id: plan.trip_id,
        label: plan.label,
        reused_existing: false,
    })
}

#[tauri::command]
fn probe_summary(state: State<'_, ProbeState>) -> Result<ProbeSummary, String> {
    state
        .service
        .lock()
        .map(|service| service.summary())
        .map_err(|_| state_lock_error("Application state"))
}

#[tauri::command]
fn load_request_page(
    input: RequestPageInput,
    state: State<'_, ProbeState>,
) -> Result<RequestPage, String> {
    let service = state
        .service
        .lock()
        .map_err(|_| state_lock_error("Application state"))?;
    load_request_page_impl(&service, &input)
}

#[tauri::command]
fn update_request(
    input: RequestUpdate,
    state: State<'_, ProbeState>,
) -> Result<LoadRequest, String> {
    let mut service = state
        .service
        .lock()
        .map_err(|_| state_lock_error("Application state"))?;
    apply_request_update(&mut service, &input)
}

#[tauri::command]
fn save_local_settings(
    raw_json: String,
    state: State<'_, ProbeState>,
) -> Result<ProbeSettings, String> {
    let settings = parse_settings(&raw_json)?;
    let mut saved = state
        .settings
        .lock()
        .map_err(|_| state_lock_error("Settings"))?;
    *saved = settings.clone();
    Ok(settings)
}

#[tauri::command]
fn prepare_local_file_choice(input: FileChoiceInput) -> Result<FileChoicePlan, String> {
    file_choice_plan(&input)
}

#[tauri::command]
fn export_synthetic_trip_package(
    input: TripExportInput,
    state: State<'_, ProbeState>,
) -> Result<TripExportPayload, String> {
    let service = state
        .service
        .lock()
        .map_err(|_| state_lock_error("Application state"))?;
    export_trip_package(&service, &input)
}

#[tauri::command]
fn start_candidate_calculation(
    input: CandidateCalculationInput,
    app: AppHandle,
    state: State<'_, ProbeState>,
) -> Result<CandidateCalculationStarted, String> {
    {
        let service = state
            .service
            .lock()
            .map_err(|_| state_lock_error("Application state"))?;
        if service.get_request(input.request_id).is_none() {
            return Err("Request no longer exists".to_owned());
        }
    }
    if input.vehicle_ids.is_empty() {
        return Err("At least one vehicle is required".to_owned());
    }

    let calculation_id = state.next_calculation_id.fetch_add(1, Ordering::Relaxed);
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .calculations
        .lock()
        .map_err(|_| state_lock_error("Calculation registry"))?
        .insert(calculation_id, Arc::clone(&cancel));

    let calculations = Arc::clone(&state.calculations);
    let vehicle_ids = input.vehicle_ids;
    let request_id = input.request_id;

    tauri::async_runtime::spawn_blocking(move || {
        run_candidate_work(
            calculation_id,
            request_id,
            &vehicle_ids,
            &cancel,
            Duration::from_millis(15),
            |progress| {
                let _ = app.emit("candidate-calculation-progress", progress);
            },
        );

        if let Ok(mut registry) = calculations.lock() {
            registry.remove(&calculation_id);
        }
    });

    Ok(CandidateCalculationStarted { calculation_id })
}

#[tauri::command]
fn cancel_candidate_calculation(
    input: CancelCalculationInput,
    state: State<'_, ProbeState>,
) -> Result<(), String> {
    let registry = state
        .calculations
        .lock()
        .map_err(|_| state_lock_error("Calculation registry"))?;
    let cancel = registry
        .get(&input.calculation_id)
        .ok_or_else(|| "Calculation is not active".to_owned())?;
    cancel.store(true, Ordering::Relaxed);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(ProbeState::new())
        .invoke_handler(tauri::generate_handler![
            probe_summary,
            load_request_page,
            update_request,
            save_local_settings,
            prepare_local_file_choice,
            export_synthetic_trip_package,
            start_candidate_calculation,
            cancel_candidate_calculation,
            control_overview,
            trip_snapshot,
            update_trip_status,
            open_trip_window
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Cargo dispatch UI probe");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_page_is_bounded_and_uses_application_service() {
        let service = ApplicationService::synthetic();
        let page = load_request_page_impl(
            &service,
            &RequestPageInput {
                filter: "Berlin".to_owned(),
                offset: 2,
                limit: 25,
            },
        )
        .expect("page loads");

        assert!(page.total_filtered >= page.rows.len());
        assert!(page.rows.len() <= 25);
        assert_eq!(page.offset, 2);
    }

    #[test]
    fn request_update_goes_through_draft_validation() {
        let mut service = ApplicationService::synthetic();
        let saved = apply_request_update(
            &mut service,
            &RequestUpdate {
                request_id: 1,
                mode: "manual".to_owned(),
                changes: vec![FieldChange {
                    key: "origin".to_owned(),
                    value: "Vienna".to_owned(),
                }],
            },
        )
        .expect("valid update saves");
        assert_eq!(saved.origin, "Vienna");

        let error = apply_request_update(
            &mut service,
            &RequestUpdate {
                request_id: 1,
                mode: "manual".to_owned(),
                changes: vec![FieldChange {
                    key: "origin".to_owned(),
                    value: String::new(),
                }],
            },
        )
        .expect_err("invalid update stays visible");
        assert!(error.contains("origin"));
    }

    #[test]
    fn malformed_settings_are_an_explicit_error() {
        let error = parse_settings("{broken").expect_err("broken JSON fails");
        assert!(error.contains("Invalid settings"));

        let valid =
            parse_settings(r#"{"language":"de","archive_root":"D:/cargo","compact_tables":true}"#)
                .expect("valid settings parse");
        assert_eq!(valid.language, "de");
    }

    #[test]
    fn file_choice_boundary_does_not_open_or_read_files() {
        let plan = file_choice_plan(&FileChoiceInput {
            purpose: "attach_document".to_owned(),
        })
        .expect("known purpose");
        assert!(plan.platform_adapter_required);
        assert!(plan.allowed_extensions.contains(&"pdf".to_owned()));
    }

    #[test]
    fn trip_export_serializes_application_snapshot() {
        let service = ApplicationService::synthetic();
        let exported = export_trip_package(&service, &TripExportInput { request_id: 1 })
            .expect("synthetic trip exports");
        let parsed: serde_json::Value =
            serde_json::from_str(&exported.content).expect("export is valid JSON");
        assert_eq!(parsed["schema_version"], "ui-probe-trip-v1");
        assert_eq!(parsed["request"]["id"], 1);
        assert!(parsed["route"]["points"].as_array().is_some());
    }

    #[test]
    fn long_calculation_reports_progress_and_can_finish() {
        let cancel = AtomicBool::new(false);
        let mut progress = Vec::new();
        let final_state =
            run_candidate_work(7, 1, &[10, 20, 30, 40], &cancel, Duration::ZERO, |item| {
                progress.push(item)
            });
        assert_eq!(final_state.state, "completed");
        assert_eq!(final_state.percent, 100);
        assert_eq!(final_state.candidate_count, 3);
        assert!(progress.len() >= 2);
    }

    #[test]
    fn long_calculation_observes_cancellation() {
        let cancel = AtomicBool::new(true);
        let mut progress = Vec::new();
        let final_state = run_candidate_work(8, 1, &[10, 20], &cancel, Duration::ZERO, |item| {
            progress.push(item)
        });
        assert_eq!(final_state.state, "cancelled");
        assert_eq!(final_state.candidate_count, 0);
        assert_eq!(progress.last(), Some(&final_state));
    }

    #[test]
    fn two_trip_windows_share_one_control_state_without_leaking_updates() {
        let mut control = ControlService::synthetic();
        let before_b = control.snapshot("TRIP-B").expect("trip B exists");

        let updated_a = control
            .update(&TripStatusUpdate {
                trip_id: "TRIP-A".to_owned(),
                status: "at_pickup".to_owned(),
                note: "Gate 3".to_owned(),
            })
            .expect("trip A updates");

        assert_eq!(updated_a.status, "at_pickup");
        assert_eq!(updated_a.last_note, "Gate 3");
        assert_eq!(
            control.snapshot("TRIP-B").expect("trip B stays present"),
            before_b
        );

        let overview = control.overview();
        assert_eq!(overview.trips.len(), 2);
        assert!(overview.revision > 1);
    }

    #[test]
    fn closing_a_window_does_not_delete_trip_state() {
        let mut control = ControlService::synthetic();
        control
            .update(&TripStatusUpdate {
                trip_id: "TRIP-A".to_owned(),
                status: "unloading".to_owned(),
                note: "Dock 2".to_owned(),
            })
            .expect("trip updates before window close");

        let before_close = control.snapshot("TRIP-A").expect("trip exists");
        let plan = control.window_plan("TRIP-A").expect("window plan exists");
        assert_eq!(plan.label, "trip-trip-a");

        let after_reopen = control.snapshot("TRIP-A").expect("trip survives reopen");
        assert_eq!(after_reopen, before_close);
    }

    #[test]
    fn each_trip_has_a_stable_distinct_window_label() {
        let control = ControlService::synthetic();
        let a = control.window_plan("TRIP-A").expect("trip A plan");
        let b = control.window_plan("TRIP-B").expect("trip B plan");
        assert_ne!(a.label, b.label);
        assert!(a.title.contains("BC 1001 AA"));
        assert!(b.title.contains("BC 2002 BB"));
    }
}
