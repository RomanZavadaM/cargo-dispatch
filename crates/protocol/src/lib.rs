use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ControlInstanceId = Uuid;
pub type TripSessionId = Uuid;
pub type BrowserSessionId = Uuid;
pub type MessageId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalMessage {
    RegisterControl {
        control_instance_id: ControlInstanceId,
    },
    RegisterTrip {
        control_instance_id: ControlInstanceId,
        trip_session_id: TripSessionId,
        access_token: String,
    },
    BrowserHello {
        browser_session_id: BrowserSessionId,
        trip_session_id: TripSessionId,
        access_token: String,
    },
    Offer {
        trip_session_id: TripSessionId,
        from: Endpoint,
        sdp: String,
    },
    Answer {
        trip_session_id: TripSessionId,
        from: Endpoint,
        sdp: String,
    },
    IceCandidate {
        trip_session_id: TripSessionId,
        from: Endpoint,
        candidate: String,
    },
    Heartbeat {
        trip_session_id: Option<TripSessionId>,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    Control,
    Browser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TripSnapshot {
    pub trip_session_id: TripSessionId,
    pub version: u64,
    pub vehicle_label: String,
    pub points: Vec<TripPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TripPoint {
    pub point_id: Uuid,
    pub order: u32,
    pub title: String,
    pub description: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_message_roundtrips_json() {
        let msg = SignalMessage::Heartbeat {
            trip_session_id: Some(Uuid::new_v4()),
        };
        let encoded = serde_json::to_string(&msg).expect("serialize");
        let decoded: SignalMessage = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(msg, decoded);
    }
}
