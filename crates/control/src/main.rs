use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::BytesMut;
use cargo_connectivity_protocol::{
    ControlChannelMessage, Endpoint, FileTransferControl, IceServerConfig, SignalMessage,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{info, warn};
use uuid::Uuid;
use webrtc::{
    data_channel::{DataChannel, DataChannelEvent},
    peer_connection::{
        MediaEngine, PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
        RTCConfigurationBuilder, RTCIceGatheringState, RTCIceServer, RTCPeerConnectionState,
        RTCSessionDescription, Registry, register_default_interceptors,
    },
    runtime::{Runtime, Sender, channel, default_runtime},
};

const MAX_PROTOTYPE_FILE_SIZE: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedControlState {
    control_instance_id: Uuid,
    trips: Vec<PersistedTripState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedTripState {
    trip_session_id: Uuid,
    access_token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyControlState {
    control_instance_id: Uuid,
    trip_session_id: Uuid,
    access_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ControlStateFile {
    Current(PersistedControlState),
    Legacy(LegacyControlState),
}

struct IncomingFile {
    file_id: Uuid,
    name: String,
    mime_type: String,
    expected_size: u64,
    expected_sha256: String,
    expected_chunks: u32,
    received_chunks: u32,
    data: Vec<u8>,
}

#[derive(Clone)]
struct ControlHandler {
    runtime: Arc<dyn Runtime>,
    gather_complete_tx: Sender<()>,
    seen_event_ids: Arc<Mutex<HashSet<Uuid>>>,
    trip_session_id: Uuid,
}

#[async_trait]
impl PeerConnectionEventHandler for ControlHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        info!(?state, "ICE gathering state");
        if state == RTCIceGatheringState::Complete {
            let _ = self.gather_complete_tx.try_send(());
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        info!(?state, "peer connection state");
    }

    async fn on_data_channel(&self, data_channel: Arc<dyn DataChannel>) {
        let runtime = self.runtime.clone();
        let seen_event_ids = self.seen_event_ids.clone();
        let trip_session_id = self.trip_session_id;
        self.runtime.spawn(Box::pin(async move {
            let label = data_channel.label().await.unwrap_or_default();
            let id = data_channel.id();
            let mut incoming_file: Option<IncomingFile> = None;
            info!(%label, %id, "data channel announced");

            while let Some(event) = data_channel.poll().await {
                match event {
                    DataChannelEvent::OnOpen => {
                        info!(%label, %id, "data channel open");
                        if label == "control" {
                            let _ = data_channel.send_text("control-ready").await;
                        } else if label == "files"
                            && let Err(error) =
                                send_synthetic_pdf(&data_channel, trip_session_id).await
                        {
                            warn!(%error, "failed to send synthetic PDF");
                        }
                    }
                    DataChannelEvent::OnMessage(message) if label == "files" => {
                        if message.is_string {
                            let text = String::from_utf8_lossy(&message.data);
                            match serde_json::from_str::<FileTransferControl>(&text) {
                                Ok(FileTransferControl::FileStart {
                                    file_id,
                                    name,
                                    mime_type,
                                    size,
                                    sha256,
                                    chunk_count,
                                    ..
                                }) => {
                                    if size > MAX_PROTOTYPE_FILE_SIZE {
                                        send_file_ack(
                                            &data_channel,
                                            file_id,
                                            0,
                                            String::new(),
                                            false,
                                            format!(
                                                "prototype limit exceeded: {size} > {MAX_PROTOTYPE_FILE_SIZE}"
                                            ),
                                        )
                                        .await;
                                        continue;
                                    }

                                    incoming_file = Some(IncomingFile {
                                        file_id,
                                        name,
                                        mime_type,
                                        expected_size: size,
                                        expected_sha256: sha256,
                                        expected_chunks: chunk_count,
                                        received_chunks: 0,
                                        data: Vec::new(),
                                    });
                                    info!(%file_id, size, "file transfer started");
                                }
                                Ok(FileTransferControl::FileEnd { file_id }) => {
                                    let Some(file) = incoming_file.take() else {
                                        send_file_ack(
                                            &data_channel,
                                            file_id,
                                            0,
                                            String::new(),
                                            false,
                                            "file_end without file_start".into(),
                                        )
                                        .await;
                                        continue;
                                    };

                                    if file.file_id != file_id {
                                        send_file_ack(
                                            &data_channel,
                                            file_id,
                                            file.data.len() as u64,
                                            String::new(),
                                            false,
                                            "file_id mismatch".into(),
                                        )
                                        .await;
                                        continue;
                                    }

                                    let actual_sha256 =
                                        format!("{:x}", Sha256::digest(&file.data));
                                    let size_ok = file.data.len() as u64 == file.expected_size;
                                    let hash_ok = actual_sha256 == file.expected_sha256;
                                    let chunks_ok = file.received_chunks == file.expected_chunks;
                                    let ok = size_ok && hash_ok && chunks_ok;

                                    let message = if ok {
                                        match save_received_file(&file, trip_session_id) {
                                            Ok(path) => format!("saved to {}", path.display()),
                                            Err(error) => format!("verified but save failed: {error}"),
                                        }
                                    } else {
                                        format!(
                                            "verification failed: size={size_ok}, sha256={hash_ok}, chunks={chunks_ok}"
                                        )
                                    };

                                    info!(
                                        %file_id,
                                        received_size = file.data.len(),
                                        %actual_sha256,
                                        ok,
                                        "file transfer completed"
                                    );

                                    send_file_ack(
                                        &data_channel,
                                        file_id,
                                        file.data.len() as u64,
                                        actual_sha256,
                                        ok,
                                        message,
                                    )
                                    .await;
                                }
                                Ok(FileTransferControl::FileAck {
                                    file_id,
                                    received_size,
                                    sha256,
                                    ok,
                                    message,
                                }) => {
                                    info!(
                                        %file_id,
                                        received_size,
                                        %sha256,
                                        ok,
                                        %message,
                                        "browser acknowledged outbound file"
                                    );
                                }
                                Err(error) => {
                                    warn!(%error, message = %text, "invalid file control message");
                                }
                            }
                        } else if let Some(file) = incoming_file.as_mut() {
                            file.data.extend_from_slice(&message.data);
                            file.received_chunks = file.received_chunks.saturating_add(1);
                        } else {
                            warn!("binary file chunk received without file_start");
                        }
                    }
                    DataChannelEvent::OnMessage(message) if label == "control" => {
                        if !message.is_string {
                            warn!("binary message received on control channel");
                            continue;
                        }

                        let text = String::from_utf8_lossy(&message.data);
                        match serde_json::from_str::<ControlChannelMessage>(&text) {
                            Ok(ControlChannelMessage::Hello { source }) => {
                                info!(%source, "control channel hello");
                            }
                            Ok(ControlChannelMessage::DriverEvent {
                                event_id,
                                trip_session_id,
                                event_type,
                                created_at,
                                payload,
                            }) => {
                                let is_new = seen_event_ids.lock().await.insert(event_id);
                                info!(
                                    %event_id,
                                    %trip_session_id,
                                    %event_type,
                                    %created_at,
                                    %payload,
                                    is_new,
                                    "driver event received"
                                );

                                let ack = ControlChannelMessage::EventAck { event_id };
                                if let Ok(text) = serde_json::to_string(&ack) {
                                    let _ = data_channel.send_text(&text).await;
                                }
                            }
                            Ok(ControlChannelMessage::EventAck { event_id }) => {
                                info!(%event_id, "unexpected event ack from browser");
                            }
                            Err(error) => {
                                warn!(%error, message = %text, "invalid control message");
                            }
                        }
                    }
                    DataChannelEvent::OnMessage(message) => {
                        let text = String::from_utf8_lossy(&message.data);
                        info!(%label, message = %text, "data channel message");
                    }
                    DataChannelEvent::OnClose => {
                        warn!(%label, %id, "data channel closed");
                        break;
                    }
                    _ => {}
                }
            }

            drop(runtime);
        }));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cargo_connectivity_control=info".into()),
        )
        .init();

    let gateway_url =
        std::env::var("CARGO_GATEWAY_WS").unwrap_or_else(|_| "ws://127.0.0.1:8080/signal".into());

    let state_path = std::env::var("CARGO_CONTROL_STATE")
        .unwrap_or_else(|_| "prototype-control-state.json".into());
    let persisted = load_or_create_control_state(Path::new(&state_path))?;

    info!(
        %gateway_url,
        control_instance_id = %persisted.control_instance_id,
        trip_count = persisted.trips.len(),
        "starting control prototype"
    );
    for (index, trip) in persisted.trips.iter().enumerate() {
        println!("trip_{}_session_id={}", index + 1, trip.trip_session_id);
        println!("trip_{}_access_token={}", index + 1, trip.access_token);
    }

    let runtime = default_runtime().context("webrtc runtime unavailable")?;
    let seen_event_ids = Arc::new(Mutex::new(HashSet::new()));

    loop {
        match run_signaling_session(
            &gateway_url,
            &persisted,
            runtime.clone(),
            seen_event_ids.clone(),
        )
        .await
        {
            Ok(()) => warn!("gateway signaling disconnected; reconnecting"),
            Err(error) => warn!(%error, "gateway signaling failed; reconnecting"),
        }

        runtime.sleep(Duration::from_secs(2)).await;
    }
}

async fn run_signaling_session(
    gateway_url: &str,
    persisted: &PersistedControlState,
    runtime: Arc<dyn Runtime>,
    seen_event_ids: Arc<Mutex<HashSet<Uuid>>>,
) -> Result<()> {
    let control_instance_id = persisted.control_instance_id;

    let (socket, _) = connect_async(gateway_url)
        .await
        .with_context(|| format!("connect signaling websocket {gateway_url}"))?;
    let (mut ws_write, mut ws_read) = socket.split();

    send_signal(
        &mut ws_write,
        &SignalMessage::RegisterControl {
            control_instance_id,
        },
    )
    .await?;

    for trip in &persisted.trips {
        send_signal(
            &mut ws_write,
            &SignalMessage::RegisterTrip {
                control_instance_id,
                trip_session_id: trip.trip_session_id,
                access_token: trip.access_token.clone(),
            },
        )
        .await?;
    }

    info!(
        %control_instance_id,
        trip_count = persisted.trips.len(),
        "control registered with gateway"
    );

    let known_trips: HashSet<Uuid> = persisted
        .trips
        .iter()
        .map(|trip| trip.trip_session_id)
        .collect();
    let mut ice_servers_by_trip: HashMap<Uuid, Vec<RTCIceServer>> = known_trips
        .iter()
        .copied()
        .map(|trip_id| (trip_id, default_ice_servers()))
        .collect();
    let mut peer_connections = HashMap::new();

    while let Some(message) = ws_read.next().await {
        let message = message?;
        let WsMessage::Text(text) = message else {
            continue;
        };
        let signal: SignalMessage = serde_json::from_str(&text)?;

        match signal {
            SignalMessage::IceConfig {
                trip_session_id,
                servers,
                expires_at,
            } if known_trips.contains(&trip_session_id) => {
                let ice_servers: Vec<_> =
                    servers.into_iter().map(to_rtc_ice_server).collect();
                info!(
                    %trip_session_id,
                    server_count = ice_servers.len(),
                    ?expires_at,
                    "ICE config received"
                );
                ice_servers_by_trip.insert(trip_session_id, ice_servers);
            }
            SignalMessage::Offer {
                trip_session_id,
                from: Endpoint::Browser,
                sdp,
            } if known_trips.contains(&trip_session_id) => {
                info!(%trip_session_id, "received browser offer");

                let mut media_engine = MediaEngine::default();
                media_engine.register_default_codecs()?;
                let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;
                let (gather_complete_tx, mut gather_complete_rx) = channel::<()>(1);

                let ice_servers = ice_servers_by_trip
                    .get(&trip_session_id)
                    .cloned()
                    .unwrap_or_else(default_ice_servers);

                let pc = PeerConnectionBuilder::new()
                    .with_configuration(
                        RTCConfigurationBuilder::new()
                            .with_ice_servers(ice_servers)
                            .build(),
                    )
                    .with_media_engine(media_engine)
                    .with_interceptor_registry(registry)
                    .with_handler(Arc::new(ControlHandler {
                        runtime: runtime.clone(),
                        gather_complete_tx,
                        seen_event_ids: seen_event_ids.clone(),
                        trip_session_id,
                    }))
                    .with_runtime(runtime.clone())
                    .with_udp_addrs(vec!["0.0.0.0:0".to_owned()])
                    .build()
                    .await?;

                let offer: RTCSessionDescription = serde_json::from_str(&sdp)?;
                pc.set_remote_description(offer).await?;

                let answer = pc.create_answer(None).await?;
                pc.set_local_description(answer).await?;
                let _ = gather_complete_rx.recv().await;

                let local = pc
                    .local_description()
                    .await
                    .context("missing local description")?;
                let sdp = serde_json::to_string(&local)?;

                send_signal(
                    &mut ws_write,
                    &SignalMessage::Answer {
                        trip_session_id,
                        from: Endpoint::Control,
                        sdp,
                    },
                )
                .await?;

                if let Some(old_pc) = peer_connections.insert(trip_session_id, pc) {
                    old_pc.close().await?;
                }
                info!(%trip_session_id, "answer sent");
            }
            SignalMessage::Error { code, message } => {
                warn!(%code, %message, "gateway error");
            }
            _ => {}
        }
    }

    for (_, pc) in peer_connections {
        pc.close().await?;
    }
    Ok(())
}

fn load_or_create_control_state(path: &Path) -> Result<PersistedControlState> {
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("read control state {}", path.display()))?;
        let parsed: ControlStateFile = serde_json::from_str(&content)
            .with_context(|| format!("parse control state {}", path.display()))?;

        let mut state = match parsed {
            ControlStateFile::Current(state) => state,
            ControlStateFile::Legacy(legacy) => PersistedControlState {
                control_instance_id: legacy.control_instance_id,
                trips: vec![PersistedTripState {
                    trip_session_id: legacy.trip_session_id,
                    access_token: legacy.access_token,
                }],
            },
        };

        let changed = ensure_two_prototype_trips(&mut state);
        if changed {
            persist_control_state(path, &state)?;
        }
        return Ok(state);
    }

    let mut state = PersistedControlState {
        control_instance_id: Uuid::new_v4(),
        trips: Vec::new(),
    };
    ensure_two_prototype_trips(&mut state);
    persist_control_state(path, &state)?;
    Ok(state)
}

fn ensure_two_prototype_trips(state: &mut PersistedControlState) -> bool {
    let mut changed = false;
    while state.trips.len() < 2 {
        state.trips.push(PersistedTripState {
            trip_session_id: Uuid::new_v4(),
            access_token: Uuid::new_v4().simple().to_string(),
        });
        changed = true;
    }
    changed
}

fn persist_control_state(path: &Path, state: &PersistedControlState) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create control state dir {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(state)?;
    std::fs::write(path, content)
        .with_context(|| format!("write control state {}", path.display()))?;
    Ok(())
}

fn default_ice_servers() -> Vec<RTCIceServer> {
    vec![RTCIceServer {
        urls: vec!["stun:stun.l.google.com:19302".to_owned()],
        ..Default::default()
    }]
}

fn to_rtc_ice_server(server: IceServerConfig) -> RTCIceServer {
    RTCIceServer {
        urls: server.urls,
        username: server.username,
        credential: server.credential,
    }
}

async fn send_synthetic_pdf(\n    data_channel: &Arc<dyn DataChannel>,\n    trip_session_id: Uuid,\n) -> Result<()> {
    const CHUNK_SIZE: usize = 12 * 1024;

    let data = build_synthetic_pdf();
    let file_id = Uuid::new_v4();
    let sha256 = format!("{:x}", Sha256::digest(&data));
    let chunk_count = data.len().div_ceil(CHUNK_SIZE) as u32;

    let start = FileTransferControl::FileStart {
        file_id,
        name: format!("cargo-connectivity-{trip_session_id}.pdf"),
        mime_type: "application/pdf".into(),
        size: data.len() as u64,
        sha256: sha256.clone(),
        chunk_size: CHUNK_SIZE as u32,
        chunk_count,
    };
    data_channel
        .send_text(&serde_json::to_string(&start)?)
        .await?;

    for chunk in data.chunks(CHUNK_SIZE) {
        data_channel.send(BytesMut::from(chunk)).await?;
    }

    let end = FileTransferControl::FileEnd { file_id };
    data_channel
        .send_text(&serde_json::to_string(&end)?)
        .await?;

    info!(
        %file_id,
        size = data.len(),
        %sha256,
        chunk_count,
        "synthetic PDF sent to browser"
    );
    Ok(())
}

fn build_synthetic_pdf() -> Vec<u8> {
    let content = b"BT /F1 18 Tf 72 720 Td (CARGO connectivity test PDF) Tj ET";
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();

    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        {
            let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
            stream.extend_from_slice(content);
            stream.extend_from_slice(b"\nendstream");
            stream
        },
    ];

    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );

    pdf
}

async fn send_file_ack(
    data_channel: &Arc<dyn DataChannel>,
    file_id: Uuid,
    received_size: u64,
    sha256: String,
    ok: bool,
    message: String,
) {
    let ack = FileTransferControl::FileAck {
        file_id,
        received_size,
        sha256,
        ok,
        message,
    };
    if let Ok(text) = serde_json::to_string(&ack) {
        let _ = data_channel.send_text(&text).await;
    }
}

fn save_received_file(file: &IncomingFile) -> std::io::Result<PathBuf> {
    let dir = PathBuf::from("prototype-received");
    std::fs::create_dir_all(&dir)?;

    let safe_name: String = file
        .name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    let path = dir.join(format!("{}-{}", file.file_id, safe_name));
    std::fs::write(&path, &file.data)?;
    info!(mime_type = %file.mime_type, path = %path.display(), "prototype file saved");
    Ok(path)
}

async fn send_signal<S>(sink: &mut S, signal: &SignalMessage) -> Result<()>
where
    S: futures_util::Sink<WsMessage> + Unpin,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    let text = serde_json::to_string(signal)?;
    sink.send(WsMessage::Text(text.into())).await?;
    Ok(())
}
