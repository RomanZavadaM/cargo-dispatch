use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ControlInstanceId = Uuid;
pub type TripSessionId = Uuid;
pub type BrowserSessionId = Uuid;
pub type MessageId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IceServerConfig {
    pub urls: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub username: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub credential: String,
}

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
    IceConfig {
        trip_session_id: TripSessionId,
        servers: Vec<IceServerConfig>,
        expires_at: Option<u64>,
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


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlChannelMessage {
    Hello {
        source: String,
    },
    DriverEvent {
        event_id: Uuid,
        trip_session_id: TripSessionId,
        event_type: String,
        created_at: String,
        payload: serde_json::Value,
    },
    EventAck {
        event_id: Uuid,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileTransferControl {
    FileStart {
        file_id: Uuid,
        name: String,
        mime_type: String,
        size: u64,
        sha256: String,
        chunk_size: u32,
        chunk_count: u32,
    },
    FileEnd {
        file_id: Uuid,
    },
    FileAck {
        file_id: Uuid,
        received_size: u64,
        sha256: String,
        ok: bool,
        message: String,
    },
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
