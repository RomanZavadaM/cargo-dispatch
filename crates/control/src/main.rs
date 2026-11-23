use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use cargo_connectivity_protocol::{
    Endpoint, FileTransferControl, IceServerConfig, SignalMessage,
};
use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
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
                                        match save_received_file(&file) {
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
                                Ok(FileTransferControl::FileAck { .. }) => {}
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

    let control_instance_id = Uuid::new_v4();
    let trip_session_id = Uuid::new_v4();
    let access_token = Uuid::new_v4().simple().to_string();

    info!(
        %gateway_url,
        %control_instance_id,
        %trip_session_id,
        "connecting control prototype"
    );
    println!("trip_session_id={trip_session_id}");
    println!("access_token={access_token}");

    let (socket, _) = connect_async(&gateway_url)
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
    send_signal(
        &mut ws_write,
        &SignalMessage::RegisterTrip {
            control_instance_id,
            trip_session_id,
            access_token: access_token.clone(),
        },
    )
    .await?;

    let runtime = default_runtime().context("webrtc runtime unavailable")?;
    let mut ice_servers = vec![RTCIceServer {
        urls: vec!["stun:stun.l.google.com:19302".to_owned()],
        ..Default::default()
    }];
    let mut peer_connection = None;

    while let Some(message) = ws_read.next().await {
        let message = message?;
        let WsMessage::Text(text) = message else {
            continue;
        };
        let signal: SignalMessage = serde_json::from_str(&text)?;

        match signal {
            SignalMessage::IceConfig {
                trip_session_id: config_trip,
                servers,
                expires_at,
            } if config_trip == trip_session_id => {
                ice_servers = servers.into_iter().map(to_rtc_ice_server).collect();
                info!(
                    %trip_session_id,
                    server_count = ice_servers.len(),
                    ?expires_at,
                    "ICE config received"
                );
            }
            SignalMessage::Offer {
                trip_session_id: offer_trip,
                from: Endpoint::Browser,
                sdp,
            } if offer_trip == trip_session_id => {
                info!(%offer_trip, "received browser offer");

                let mut media_engine = MediaEngine::default();
                media_engine.register_default_codecs()?;
                let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;
                let (gather_complete_tx, mut gather_complete_rx) = channel::<()>(1);

                let pc = PeerConnectionBuilder::new()
                    .with_configuration(
                        RTCConfigurationBuilder::new()
                            .with_ice_servers(ice_servers.clone())
                            .build(),
                    )
                    .with_media_engine(media_engine)
                    .with_interceptor_registry(registry)
                    .with_handler(Arc::new(ControlHandler {
                        runtime: runtime.clone(),
                        gather_complete_tx,
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

                peer_connection = Some(pc);
                info!(%trip_session_id, "answer sent");
            }
            SignalMessage::Error { code, message } => {
                warn!(%code, %message, "gateway error");
            }
            _ => {}
        }
    }

    if let Some(pc) = peer_connection {
        pc.close().await?;
    }
    Ok(())
}

fn to_rtc_ice_server(server: IceServerConfig) -> RTCIceServer {
    RTCIceServer {
        urls: server.urls,
        username: server.username,
        credential: server.credential,
    }
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
