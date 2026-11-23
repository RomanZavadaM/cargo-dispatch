use cargo_connectivity_protocol::{SignalMessage, TripPoint, TripSnapshot};
use tracing::info;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let control_instance_id = Uuid::new_v4();
    let trip_session_id = Uuid::new_v4();

    let snapshot = TripSnapshot {
        trip_session_id,
        version: 1,
        vehicle_label: "TEST-TRUCK-01".into(),
        points: vec![
            TripPoint {
                point_id: Uuid::new_v4(),
                order: 1,
                title: "Завантаження".into(),
                description: "Синтетична тестова точка".into(),
                latitude: 49.8397,
                longitude: 24.0297,
            },
            TripPoint {
                point_id: Uuid::new_v4(),
                order: 2,
                title: "Розвантаження".into(),
                description: "Синтетична тестова точка".into(),
                latitude: 50.0647,
                longitude: 19.9450,
            },
        ],
    };

    let register = SignalMessage::RegisterControl { control_instance_id };

    info!(%control_instance_id, %trip_session_id, "connectivity control prototype");
    println!("{}", serde_json::to_string_pretty(&register).expect("serialize register"));
    println!("{}", serde_json::to_string_pretty(&snapshot).expect("serialize snapshot"));
}
