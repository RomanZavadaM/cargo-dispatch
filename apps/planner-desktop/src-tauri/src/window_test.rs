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
            let status = tokio::task::spawn_blocking(move || {
        // Older Node keeps the browser connection behind a flag.
        let ready = Command::new("node")
            .args(["-e", "process.exit(typeof WebSocket === 'function' ? 0 : 1)"])
            .status()?
            .success();
        let mut node = Command::new("node");
        if !ready {
            node.arg("--experimental-websocket");
        }
        node.arg(script).arg(url).status()
    })
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
