#![forbid(unsafe_code)]

use cargo_dispatch_domain::{PointId, WorkingRouteId};
use cargo_dispatch_planner::{
    WorkingRoute, WorkingRouteState, edit_manual_working_route, manual_working_route,
};
use cargo_dispatch_planner_ui_contracts::{
    PlannerPointInput, PlannerPointView, PlannerRouteView, PointActionInput, ReorderPointsInput,
};
use std::{collections::BTreeMap, sync::Mutex};
use tauri::State;

#[derive(Debug, Clone)]
struct ManualPoint {
    point_id: PointId,
    title: String,
    latitude: f64,
    longitude: f64,
}

struct PlannerState {
    route: WorkingRoute,
    points: BTreeMap<PointId, ManualPoint>,
    next_serial: u64,
}

impl Default for PlannerState {
    fn default() -> Self {
        Self {
            route: manual_working_route(route_id(1), &[]).expect("empty manual route is valid"),
            points: BTreeMap::new(),
            next_serial: 2,
        }
    }
}

#[derive(Default)]
struct PlannerDesktopService {
    inner: Mutex<PlannerState>,
}

fn id_bytes(kind: u8, serial: u64) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0] = kind;
    bytes[8..].copy_from_slice(&serial.to_be_bytes());
    bytes
}
fn route_id(serial: u64) -> WorkingRouteId {
    WorkingRouteId::from_bytes(id_bytes(0x52, serial)).expect("route id is non-nil")
}
fn point_id(serial: u64) -> PointId {
    PointId::from_bytes(id_bytes(0x50, serial)).expect("point id is non-nil")
}
fn hex_id(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn validate_point(input: &PlannerPointInput) -> Result<(), String> {
    if input.title.trim().is_empty() {
        return Err("Point title is required".to_owned());
    }
    if !input.latitude.is_finite() || !(-90.0..=90.0).contains(&input.latitude) {
        return Err("Latitude must be between -90 and 90".to_owned());
    }
    if !input.longitude.is_finite() || !(-180.0..=180.0).contains(&input.longitude) {
        return Err("Longitude must be between -180 and 180".to_owned());
    }
    Ok(())
}
fn point_from_text(state: &PlannerState, value: &str) -> Result<PointId, String> {
    state
        .points
        .keys()
        .copied()
        .find(|id| hex_id(id.as_bytes()) == value)
        .ok_or_else(|| "Unknown point id".to_owned())
}
fn route_view(state: &PlannerState) -> Result<PlannerRouteView, String> {
    let points = state
        .route
        .points
        .iter()
        .enumerate()
        .map(|(index, route_point)| {
            let point = state
                .points
                .get(&route_point.point_id)
                .ok_or_else(|| "Planner point metadata is missing".to_owned())?;
            Ok(PlannerPointView {
                point_id: hex_id(point.point_id.as_bytes()),
                sequence: index + 1,
                title: point.title.clone(),
                latitude: point.latitude,
                longitude: point.longitude,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let state_name = match state.route.state {
        WorkingRouteState::Draft => "draft",
        WorkingRouteState::Selected => "selected",
        WorkingRouteState::Negotiating => "negotiating",
        WorkingRouteState::PartiallyConfirmed => "partially_confirmed",
        WorkingRouteState::Ready => "ready",
        WorkingRouteState::TransferredToControl => "transferred_to_control",
        WorkingRouteState::Replanning => "replanning",
        WorkingRouteState::Cancelled => "cancelled",
    };
    Ok(PlannerRouteView {
        working_route_id: hex_id(state.route.working_route_id.as_bytes()),
        revision: state.route.current_revision,
        state: state_name.to_owned(),
        points,
    })
}

impl PlannerDesktopService {
    fn status(&self) -> Result<PlannerRouteView, String> {
        let state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        route_view(&state)
    }
    fn reset(&self) -> Result<PlannerRouteView, String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        let serial = state.next_serial;
        state.next_serial = state
            .next_serial
            .checked_add(1)
            .ok_or("Planner id overflow")?;
        state.route = manual_working_route(route_id(serial), &[])
            .map_err(|error| format!("Could not create manual route: {error:?}"))?;
        state.points.clear();
        route_view(&state)
    }
    fn add_point(&self, input: PlannerPointInput) -> Result<PlannerRouteView, String> {
        validate_point(&input)?;
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        let serial = state.next_serial;
        state.next_serial = state
            .next_serial
            .checked_add(1)
            .ok_or("Planner id overflow")?;
        let id = point_id(serial);
        let mut order = state
            .route
            .points
            .iter()
            .map(|point| point.point_id)
            .collect::<Vec<_>>();
        order.push(id);
        let updated = edit_manual_working_route(&state.route, &order)
            .map_err(|error| format!("Planner rejected the point edit: {error:?}"))?;
        state.points.insert(
            id,
            ManualPoint {
                point_id: id,
                title: input.title.trim().to_owned(),
                latitude: input.latitude,
                longitude: input.longitude,
            },
        );
        state.route = updated;
        route_view(&state)
    }
    fn remove_point(&self, input: PointActionInput) -> Result<PlannerRouteView, String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        let id = point_from_text(&state, &input.point_id)?;
        let order = state
            .route
            .points
            .iter()
            .map(|point| point.point_id)
            .filter(|candidate| *candidate != id)
            .collect::<Vec<_>>();
        let updated = edit_manual_working_route(&state.route, &order)
            .map_err(|error| format!("Planner rejected the point removal: {error:?}"))?;
        state.route = updated;
        state.points.remove(&id);
        route_view(&state)
    }
    fn reorder(&self, input: ReorderPointsInput) -> Result<PlannerRouteView, String> {
        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Planner state unavailable".to_owned())?;
        if input.point_ids.len() != state.route.points.len() {
            return Err("Reorder must contain every current point exactly once".to_owned());
        }
        let order = input
            .point_ids
            .iter()
            .map(|value| point_from_text(&state, value))
            .collect::<Result<Vec<_>, String>>()?;
        let updated = edit_manual_working_route(&state.route, &order)
            .map_err(|error| format!("Planner rejected the point order: {error:?}"))?;
        state.route = updated;
        route_view(&state)
    }
}

#[tauri::command]
fn planner_status(state: State<'_, PlannerDesktopService>) -> Result<PlannerRouteView, String> {
    state.status()
}
#[tauri::command]
fn planner_reset_route(
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.reset()
}
#[tauri::command]
fn planner_add_point(
    input: PlannerPointInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.add_point(input)
}
#[tauri::command]
fn planner_remove_point(
    input: PointActionInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.remove_point(input)
}
#[tauri::command]
fn planner_reorder_points(
    input: ReorderPointsInput,
    state: State<'_, PlannerDesktopService>,
) -> Result<PlannerRouteView, String> {
    state.reorder(input)
}

pub fn run() {
    tauri::Builder::default()
        .manage(PlannerDesktopService::default())
        .invoke_handler(tauri::generate_handler![
            planner_status,
            planner_reset_route,
            planner_add_point,
            planner_remove_point,
            planner_reorder_points
        ])
        .run(tauri::generate_context!())
        .expect("Cargo Dispatch Planner failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    fn point(title: &str, latitude: f64, longitude: f64) -> PlannerPointInput {
        PlannerPointInput {
            title: title.to_owned(),
            latitude,
            longitude,
        }
    }

    #[test]
    fn manual_route_service_uses_core_revision_for_add_reorder_remove() {
        let service = PlannerDesktopService::default();
        let initial = service.status().expect("initial");
        assert_eq!(initial.revision, 1);
        assert_eq!(initial.state, "draft");
        assert!(initial.points.is_empty());
        assert_eq!(
            service
                .add_point(point("A", 49.8393, 24.0296))
                .expect("A")
                .revision,
            2
        );
        assert_eq!(
            service
                .add_point(point("B", 49.8430, 24.0360))
                .expect("B")
                .revision,
            3
        );
        let current = service.add_point(point("C", 49.8500, 24.0500)).expect("C");
        assert_eq!(current.revision, 4);
        let reordered = service
            .reorder(ReorderPointsInput {
                point_ids: vec![
                    current.points[2].point_id.clone(),
                    current.points[0].point_id.clone(),
                    current.points[1].point_id.clone(),
                ],
            })
            .expect("reorder");
        assert_eq!(reordered.revision, 5);
        assert_eq!(
            reordered
                .points
                .iter()
                .map(|point| point.title.as_str())
                .collect::<Vec<_>>(),
            vec!["C", "A", "B"]
        );
        assert_eq!(reordered.state, "draft");
        let removed = service
            .remove_point(PointActionInput {
                point_id: reordered.points[1].point_id.clone(),
            })
            .expect("remove");
        assert_eq!(removed.revision, 6);
        assert_eq!(
            removed
                .points
                .iter()
                .map(|point| point.title.as_str())
                .collect::<Vec<_>>(),
            vec!["C", "B"]
        );
    }

    #[test]
    fn invalid_coordinates_do_not_change_revision() {
        let service = PlannerDesktopService::default();
        assert!(service.add_point(point("bad", 91.0, 24.0)).is_err());
        assert_eq!(service.status().expect("status").revision, 1);
    }
}
