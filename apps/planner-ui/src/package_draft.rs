//! The trip package section as it is kept between starts: every field as typed.

use crate::{CargoDraft, RoutingProfileDraft};
use cargo_dispatch_planner_ui_contracts::{CargoRowDraft, FiguresDraft, PackageDraft, ProfileDraft};

pub(crate) fn snapshot(
    vehicle_type: &str,
    cargos: &[CargoDraft],
    next_cargo_row: u64,
    profile: &RoutingProfileDraft,
    figures: FiguresDraft,
) -> PackageDraft {
    PackageDraft {
        vehicle_type: vehicle_type.to_owned(),
        cargos: cargos
            .iter()
            .map(|row| CargoRowDraft {
                row_id: row.row_id,
                title: row.title.clone(),
                weight_kg: row.weight_kg.clone(),
                pallets: row.pallets.clone(),
                pickup_point_id: row.pickup_point_id.clone(),
                unload_point_id: row.unload_point_id.clone(),
            })
            .collect(),
        next_cargo_row,
        profile: ProfileDraft {
            height_m: profile.height_m.clone(),
            width_m: profile.width_m.clone(),
            length_m: profile.length_m.clone(),
            gross_weight_t: profile.gross_weight_t.clone(),
            axle_load_t: profile.axle_load_t.clone(),
            axle_count: profile.axle_count.clone(),
            hazmat: profile.hazmat.clone(),
            hgv: profile.hgv.clone(),
            exclude_unpaved: profile.exclude_unpaved.clone(),
            prefer_truck_routes: profile.prefer_truck_routes.clone(),
            toll_preference: profile.toll_preference.clone(),
            max_speed_kmh: profile.max_speed_kmh.clone(),
        },
        figures,
    }
}

/// The saved rows and the number the next added row receives.
pub(crate) fn cargo_rows(draft: &PackageDraft) -> (Vec<CargoDraft>, u64) {
    let rows: Vec<CargoDraft> = draft
        .cargos
        .iter()
        .map(|row| CargoDraft {
            row_id: row.row_id,
            title: row.title.clone(),
            weight_kg: row.weight_kg.clone(),
            pallets: row.pallets.clone(),
            pickup_point_id: row.pickup_point_id.clone(),
            unload_point_id: row.unload_point_id.clone(),
        })
        .collect();
    let after_last = rows.iter().map(|row| row.row_id).max().unwrap_or(0) + 1;
    (rows, draft.next_cargo_row.max(after_last))
}

pub(crate) fn profile_from(draft: &ProfileDraft) -> RoutingProfileDraft {
    RoutingProfileDraft {
        height_m: draft.height_m.clone(),
        width_m: draft.width_m.clone(),
        length_m: draft.length_m.clone(),
        gross_weight_t: draft.gross_weight_t.clone(),
        axle_load_t: draft.axle_load_t.clone(),
        axle_count: draft.axle_count.clone(),
        hazmat: draft.hazmat.clone(),
        hgv: draft.hgv.clone(),
        exclude_unpaved: draft.exclude_unpaved.clone(),
        prefer_truck_routes: draft.prefer_truck_routes.clone(),
        toll_preference: draft.toll_preference.clone(),
        max_speed_kmh: draft.max_speed_kmh.clone(),
    }
}
