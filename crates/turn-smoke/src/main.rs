use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use webrtc::{
    data_channel::{DataChannel, DataChannelEvent},
    peer_connection::{
        MediaEngine, PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
        RTCConfigurationBuilder, RTCIceGatheringState, RTCIceServer, RTCIceTransportPolicy,
        RTCPeerConnectionState, Registry, register_default_interceptors,
    },
    runtime::{Runtime, Sender, channel, default_runtime},
};

type HmacSha1 = Hmac<Sha1>;

#[derive(Clone)]
struct SmokeHandler {
    runtime: Arc<dyn Runtime>,
    gather_tx: Sender<()>,
    opened_tx: Sender<()>,
    received_tx: Sender<String>,
}

#[async_trait]
impl PeerConnectionEventHandler for SmokeHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        if state == RTCIceGatheringState::Complete {
            let _ = self.gather_tx.try_send(());
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        eprintln!("peer state: {state:?}");
    }

    async fn on_data_channel(&self, data_channel: Arc<dyn DataChannel>) {
        let opened_tx = self.opened_tx.clone();
        let received_tx = self.received_tx.clone();
        self.runtime.spawn(Box::pin(async move {
            while let Some(event) = data_channel.poll().await {
                match event {
                    DataChannelEvent::OnOpen => {
                        let _ = opened_tx.try_send(());
                    }
                    DataChannelEvent::OnMessage(message) if message.is_string => {
                        let text = String::from_utf8_lossy(&message.data).to_string();
                        let _ = received_tx.try_send(text);
                    }
                    DataChannelEvent::OnClose => break,
                    _ => {}
                }
            }
        }));
    }
}

struct Peer {
    pc: Box<dyn PeerConnection>,
    gather_rx: webrtc::runtime::Receiver<()>,
    opened_rx: webrtc::runtime::Receiver<()>,
    received_rx: webrtc::runtime::Receiver<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let turn_url = std::env::var("CARGO_TURN_SMOKE_URL")
        .unwrap_or_else(|_| "turn:127.0.0.1:3478?transport=udp".into());
    let secret = std::env::var("CARGO_TURN_SMOKE_SECRET")
        .unwrap_or_else(|_| "cargo-smoke-secret".into());
    let runtime = default_runtime().context("webrtc runtime unavailable")?;

    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .saturating_add(600);
    let username = format!("{expires}:turn-smoke");
    let credential = turn_credential(&secret, &username);

    let ice_server = RTCIceServer {
        urls: vec![turn_url],
        username,
        credential,
    };

    let mut offerer = build_peer(runtime.clone(), ice_server.clone()).await?;
    let mut answerer = build_peer(runtime, ice_server).await?;

    let data_channel = offerer.pc.create_data_channel("relay-smoke", None).await?;

    let offer = offerer.pc.create_offer(None).await?;
    offerer.pc.set_local_description(offer).await?;
    wait_signal(&mut offerer.gather_rx, "offer ICE gathering").await?;

    let offer = offerer
        .pc
        .local_description()
        .await
        .context("offer local description missing")?;
    answerer.pc.set_remote_description(offer).await?;

    let answer = answerer.pc.create_answer(None).await?;
    answerer.pc.set_local_description(answer).await?;
    wait_signal(&mut answerer.gather_rx, "answer ICE gathering").await?;

    let answer = answerer
        .pc
        .local_description()
        .await
        .context("answer local description missing")?;
    offerer.pc.set_remote_description(answer).await?;

    wait_signal(&mut answerer.opened_rx, "answer data channel open").await?;

    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match data_channel.ready_state().await {
                Ok(state) if state.to_string().eq_ignore_ascii_case("open") => break,
                _ => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
    })
    .await
    .context("offer data channel did not open")?;

    data_channel.send_text("relay-only-ok").await?;
    let received = wait_value(&mut answerer.received_rx, "relay message").await?;
    anyhow::ensure!(received == "relay-only-ok", "unexpected relay payload: {received}");

    println!("TURN relay-only WebRTC data channel: PASS");

    offerer.pc.close().await?;
    answerer.pc.close().await?;
    Ok(())
}

async fn build_peer(runtime: Arc<dyn Runtime>, ice_server: RTCIceServer) -> Result<Peer> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;
    let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;

    let (gather_tx, gather_rx) = channel::<()>(1);
    let (opened_tx, opened_rx) = channel::<()>(4);
    let (received_tx, received_rx) = channel::<String>(4);

    let handler = SmokeHandler {
        runtime: runtime.clone(),
        gather_tx,
        opened_tx,
        received_tx,
    };

    let pc = PeerConnectionBuilder::new()
        .with_configuration(
            RTCConfigurationBuilder::new()
                .with_ice_servers(vec![ice_server])
                .with_ice_transport_policy(RTCIceTransportPolicy::Relay)
                .build(),
        )
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_handler(Arc::new(handler))
        .with_runtime(runtime)
        .with_udp_addrs(vec!["0.0.0.0:0".to_owned()])
        .build()
        .await?;

    Ok(Peer {
        pc: Box::new(pc),
        gather_rx,
        opened_rx,
        received_rx,
    })
}

async fn wait_signal(receiver: &mut webrtc::runtime::Receiver<()>, name: &str) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(20), receiver.recv())
        .await
        .with_context(|| format!("timeout waiting for {name}"))?
        .with_context(|| format!("channel closed waiting for {name}"))?;
    Ok(())
}

async fn wait_value(
    receiver: &mut webrtc::runtime::Receiver<String>,
    name: &str,
) -> Result<String> {
    tokio::time::timeout(Duration::from_secs(20), receiver.recv())
        .await
        .with_context(|| format!("timeout waiting for {name}"))?
        .with_context(|| format!("channel closed waiting for {name}"))
}

fn turn_credential(secret: &str, username: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(secret.as_bytes()).expect("valid HMAC key");
    mac.update(username.as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}
