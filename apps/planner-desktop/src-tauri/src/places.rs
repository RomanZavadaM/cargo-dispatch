//! A point from what the operator pasted; a shared link is followed to Google.

use cargo_dispatch_geo::short_link::redirect_target;
use cargo_dispatch_planner_ui_contracts::{PastedPlace, PlaceInput, PlaceView, read_pasted_place};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);

/// Only short links of Google are ever requested; every other address is read
/// as text.
pub(crate) async fn resolve_place(text: &str) -> Result<PlaceView, String> {
    let mut next = read_pasted_place(text);
    for _ in 0..3 {
        match next {
            Some(PastedPlace::Point(place)) => return Ok(place),
            Some(PastedPlace::ShortLink(link)) => {
                let target = redirect_target(&link, TIMEOUT)
                    .await
                    .map_err(|error| format!("Map link could not be opened: {error}"))?;
                next = read_pasted_place(&target);
            }
            None => break,
        }
    }
    match next {
        Some(PastedPlace::Point(place)) => Ok(place),
        _ => Err("Map link has no point".to_owned()),
    }
}

#[tauri::command]
pub(crate) async fn planner_place_resolve(input: PlaceInput) -> Result<PlaceView, String> {
    resolve_place(&input.text).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pasted_text_becomes_a_point_or_an_explanation() {
        let place = resolve_place("49.5, 24.5").await.expect("coordinates");
        assert_eq!((place.latitude, place.longitude), (49.5, 24.5));
        let problem = resolve_place("https://www.google.com/maps/@49.5,24.5,17z")
            .await
            .expect_err("only the centre of a map");
        assert_eq!(problem, "Map link has no point");
    }
}
