use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use cargo_connectivity_protocol::{Endpoint, SignalMessage};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{info, warn};
use uuid::Uuid;
use webrtc::{
    data_channel::{DataChannel, DataChannelEvent},
    peer_connection::{
        MediaEngine, PeerConnectionBuilder, PeerConnectionEventHandler, RTCConfigurationBuilder,
        RTCIceGatheringState, RTCIceServer, RTCPeerConnectionState, RTCSessionDescription,
        Registry, register_default_interceptors,
    },
    runtime::{Runtime, Sender, channel, default_runtime},
};

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
            info!(%label, %id, "data channel announced");

            while let Some(event) = data_channel.poll().await {
                match event {
                    DataChannelEvent::OnOpen => {
                        info!(%label, %id, "data channel open");
                        if label == "control" {
                            let _ = data_channel.send_text("control-ready").await;
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
    let stun_url =
        std::env::var("CARGO_STUN_URL").unwrap_or_else(|_| "stun:stun.l.google.com:19302".into());

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
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;
    let (gather_complete_tx, mut gather_complete_rx) = channel::<()>(1);

    let peer_connection = PeerConnectionBuilder::new()
        .with_configuration(
            RTCConfigurationBuilder::new()
                .with_ice_servers(vec![RTCIceServer {
                    urls: vec![stun_url],
                    ..Default::default()
                }])
                .build(),
        )
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_handler(Arc::new(ControlHandler {
            runtime: runtime.clone(),
            gather_complete_tx,
        }))
        .with_runtime(runtime)
        .with_udp_addrs(vec!["0.0.0.0:0".to_owned()])
        .build()
        .await?;

    while let Some(message) = ws_read.next().await {
        let message = message?;
        let WsMessage::Text(text) = message else {
            continue;
        };
        let signal: SignalMessage = serde_json::from_str(&text)?;

        match signal {
            SignalMessage::Offer {
                trip_session_id: offer_trip,
                from: Endpoint::Browser,
                sdp,
            } if offer_trip == trip_session_id => {
                info!(%offer_trip, "received browser offer");
                let offer: RTCSessionDescription = serde_json::from_str(&sdp)?;
                peer_connection.set_remote_description(offer).await?;

                let answer = peer_connection.create_answer(None).await?;
                peer_connection.set_local_description(answer).await?;
                let _ = gather_complete_rx.recv().await;

                let local = peer_connection
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
                info!(%trip_session_id, "answer sent");
            }
            SignalMessage::Error { code, message } => {
                warn!(%code, %message, "gateway error");
            }
            _ => {}
        }
    }

    peer_connection.close().await?;
    Ok(())
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
