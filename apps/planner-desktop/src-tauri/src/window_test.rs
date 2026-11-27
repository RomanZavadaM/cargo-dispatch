//! The built window in a real headless browser against the real service.
//! Only the Tauri bridge and the file dialogs are replaced.

use super::*;
use axum::{
    Json, Router,
    extract::State as AxumState,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{
    process::Command,
    sync::{Arc, RwLock},
};

/// Synthetic orders; the second one lacks the pickup coordinates.
const ORDERS: &str = r#"{
  "schema": "cargo-dispatch.planning-input",
  "schema_version": "1.0",
  "vehicle": {
    "type": "тент 20 т",
    "payload_kg": 20000,
    "pallets": 33,
    "start": { "place": "Місто А", "lat": 50.0, "lon": 30.0 }
  },
  "orders": [
    {
      "id": "S1",
      "cargo": { "title": "Вантаж А", "weight_kg": 2500, "pallets": 6 },
      "pickup": { "place": "Місто А", "lat": 50.1, "lon": 30.1 },
      "delivery": { "place": "Місто Б", "lat": 50.4, "lon": 31.5 },
      "rate": { "amount": 12000, "currency": "UAH" }
    },
    {
      "id": "S2",
      "cargo": { "title": "Вантаж Б", "weight_kg": 3000, "pallets": 8 },
      "pickup": { "place": "Місто В" },
      "delivery": { "place": "Місто Г", "lat": 50.6, "lon": 32.0 }
    }
  ]
}"#;

struct Window {
    service: RwLock<Arc<PlannerDesktopService>>,
    folder: PathBuf,
    interface: PathBuf,
}

#[derive(Deserialize)]
struct Call {
    command: String,
    args: Value,
}

/// A service as the program starts it.
fn started(folder: &Path) -> Arc<PlannerDesktopService> {
    let service = PlannerDesktopService::open(folder.join(PLANNER_JOURNAL_FILE)).expect("service");
    service.restore_routing();
    Arc::new(service)
}

fn argument<T: DeserializeOwned>(args: &Value) -> Result<T, String> {
    serde_json::from_value(args["input"].clone()).map_err(|error| error.to_string())
}

fn answer<T: Serialize>(result: Result<T, String>) -> Result<Value, String> {
    result.and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string()))
}

async fn run(window: &Window, call: Call) -> Result<Value, String> {
    let service = window.service.read().expect("service").clone();
    let args = &call.args;
    match call.command.as_str() {
        "planner_status" => answer(service.status()),
        "planner_reset_route" => answer(service.reset()),
        "planner_add_point" => answer(service.add_point(argument(args)?)),
        "planner_remove_point" => answer(service.remove_point(argument(args)?)),
        "planner_reorder_points" => answer(service.reorder(argument(args)?)),
        "planner_routing_status" => answer(service.routing_status()),
        "planner_configure_routing" => {
            let input: PlannerRoutingConfigInput = argument(args)?;
            let status = service.configure_routing(input.clone())?;
            service.remember_routing(Some(input))?;
            answer(Ok(status))
        }
        "planner_disable_routing" => {
            service.disable_routing()?;
            answer(service.remember_routing(None))
        }
        "planner_test_routing" => answer(service.test_routing().await),
        "planner_preview_route" => answer(service.preview_route(argument(args)?).await),
        "planner_export_package" => answer(service.export_trip_package(argument(args)?)),
        "planner_pick_export_path" => {
            let path = window.folder.join(service.suggested_export_file_name()?);
            answer(Ok(Some(path.to_string_lossy().into_owned())))
        }
        "planner_planning_status" => answer(service.planning_view()),
        "planner_planning_package" => answer(service.remember_package(argument(args)?)),
        "planner_planning_update" => answer(service.update_planning(argument(args)?)),
        "planner_planning_open" => answer(
            service
                .import_input(&window.folder.join("orders.json"))
                .map(Some),
        ),
        "planner_planning_save" => {
            let path = window.folder.join("saved-orders.json");
            service.save_input(&path)?;
            answer(Ok(Some(path.to_string_lossy().into_owned())))
        }
        "planner_planning_propose" => answer(service.propose(argument(args)?).await),
        "planner_planning_apply" => answer(service.apply_plan(argument(args)?)),
        // Closing and starting the program again.
        "test_restart" => {
            *window.service.write().expect("service") = started(&window.folder);
            Ok(Value::Null)
        }
        other => Err(format!("unknown command {other}")),
    }
}

async fn invoke(AxumState(window): AxumState<Arc<Window>>, Json(call): Json<Call>) -> Response {
    match run(&window, call).await {
        Ok(value) => Json(json!({ "value": value })).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, Json(json!({ "error": error }))).into_response(),
    }
}

async fn file(AxumState(window): AxumState<Arc<Window>>, uri: Uri) -> Response {
    let name = match uri.path().trim_start_matches('/') {
        "" => "index.html",
        name => name,
    };
    let kind = match Path::new(name).extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    };
    if name.contains("..") {
        return StatusCode::NOT_FOUND.into_response();
    }
    match fs::read(window.interface.join(name)) {
        Ok(bytes) => ([(header::CONTENT_TYPE, kind)], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the built interface in apps/planner-ui/dist, Node and Chrome"]
async fn the_window_works_in_a_real_browser() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let interface = manifest.join("../../planner-ui/dist");
    let script = manifest.join("../../../.github/scripts/planner-window-browser-test.mjs");
    let folder = std::env::temp_dir().join(format!(
        "cargo-planner-window-{}",
        unix_time_ms().expect("clock")
    ));
    fs::create_dir_all(&folder).expect("folder");
    fs::write(folder.join("orders.json"), ORDERS).expect("orders");
    let window = Arc::new(Window {
        service: RwLock::new(started(&folder)),
        folder: folder.clone(),
        interface,
    });
    let app = Router::new()
        .route("/invoke", post(invoke))
        .fallback(file)
        .with_state(window);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let url = format!("http://{}/", listener.local_addr().expect("address"));
    let server = tokio::spawn(async move { axum::serve(listener, app).await });

    let status =
        tokio::task::spawn_blocking(move || Command::new("node").arg(script).arg(url).status())
            .await
            .expect("browser task")
            .expect("node");
    server.abort();
    assert!(status.success(), "the browser test of the window failed");
    let exported = fs::read_dir(&folder)
        .expect("folder")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|kind| kind == "zip"))
        .count();
    assert_eq!(exported, 1, "the trip package was written");
    let _ = fs::remove_dir_all(folder);
}
