mod turn_auth;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{Html, IntoResponse},
    routing::get,
};
use cargo_connectivity_protocol::{
    BrowserSessionId, ControlInstanceId, SignalMessage, TripSessionId,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{RwLock, mpsc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};
use turn_auth::IceSettings;

type Tx = mpsc::UnboundedSender<Message>;

#[derive(Clone)]
struct AppState {
    controls: Arc<RwLock<HashMap<ControlInstanceId, Tx>>>,
    trips: Arc<RwLock<HashMap<TripSessionId, TripRegistration>>>,
    browsers: Arc<RwLock<HashMap<BrowserSessionId, Tx>>>,
    ice_settings: IceSettings,
}

impl AppState {
    fn new() -> Self {
        Self {
            controls: Arc::new(RwLock::new(HashMap::new())),
            trips: Arc::new(RwLock::new(HashMap::new())),
            browsers: Arc::new(RwLock::new(HashMap::new())),
            ice_settings: IceSettings::from_env(),
        }
    }
}

#[derive(Clone)]
struct TripRegistration {
    control_instance_id: ControlInstanceId,
    access_token: String,
    browser_session_id: Option<BrowserSessionId>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cargo_connectivity_gateway=info".into()),
        )
        .init();

    let state = AppState::new();
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../../../web/index.html")) }),
        )
        .route("/app.js", get(app_js))
        .route("/health", get(|| async { "ok" }))
        .route("/signal", get(signal_ws))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:8080".parse().expect("valid bind address");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind gateway");
    info!(%addr, "gateway listening");
    axum::serve(listener, app).await.expect("serve gateway");
}

async fn app_js() -> impl IntoResponse {
    (
        [("content-type", "text/javascript; charset=utf-8")],
        include_str!("../../../web/app.js"),
    )
}

async fn signal_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });

    let mut bound_control: Option<ControlInstanceId> = None;
    let mut bound_browser: Option<BrowserSessionId> = None;

    while let Some(Ok(message)) = stream.next().await {
        let Message::Text(text) = message else {
            continue;
        };

        let parsed = match serde_json::from_str::<SignalMessage>(&text) {
            Ok(v) => v,
            Err(err) => {
                send_json(
                    &tx,
                    &SignalMessage::Error {
                        code: "bad_json".into(),
                        message: err.to_string(),
                    },
                );
                continue;
            }
        };

        match parsed {
            SignalMessage::RegisterControl {
                control_instance_id,
            } => {
                state
                    .controls
                    .write()
                    .await
                    .insert(control_instance_id, tx.clone());
                bound_control = Some(control_instance_id);
                info!(%control_instance_id, "control registered");
            }
            SignalMessage::RegisterTrip {
                control_instance_id,
                trip_session_id,
                access_token,
            } => {
                if !state
                    .controls
                    .read()
                    .await
                    .contains_key(&control_instance_id)
                {
                    send_json(
                        &tx,
                        &SignalMessage::Error {
                            code: "control_not_registered".into(),
                            message: "register control before trip".into(),
                        },
                    );
                    continue;
                }
                let browser_session_id = {
                    let mut trips = state.trips.write().await;
                    let existing_browser = trips
                        .get(&trip_session_id)
                        .and_then(|registration| registration.browser_session_id);
                    trips.insert(
                        trip_session_id,
                        TripRegistration {
                            control_instance_id,
                            access_token,
                            browser_session_id: existing_browser,
                        },
                    );
                    existing_browser
                };

                let (servers, expires_at) = state.ice_settings.for_trip(trip_session_id);
                let ice_config = SignalMessage::IceConfig {
                    trip_session_id,
                    servers,
                    expires_at,
                };
                send_json(&tx, &ice_config);

                if let Some(browser_session_id) = browser_session_id
                    && let Some(browser_tx) = state
                        .browsers
                        .read()
                        .await
                        .get(&browser_session_id)
                        .cloned()
                {
                    send_json(&browser_tx, &ice_config);
                    info!(
                        %browser_session_id,
                        %trip_session_id,
                        "control re-registered; browser asked to renegotiate"
                    );
                }

                info!(%control_instance_id, %trip_session_id, "trip registered");
            }
            SignalMessage::BrowserHello {
                browser_session_id,
                trip_session_id,
                access_token,
            } => {
                let mut trips = state.trips.write().await;
                let Some(registration) = trips.get_mut(&trip_session_id) else {
                    send_json(
                        &tx,
                        &SignalMessage::Error {
                            code: "trip_not_found".into(),
                            message: "trip session is not active".into(),
                        },
                    );
                    continue;
                };
                if registration.access_token != access_token {
                    send_json(
                        &tx,
                        &SignalMessage::Error {
                            code: "invalid_token".into(),
                            message: "invalid trip access token".into(),
                        },
                    );
                    continue;
                }
                registration.browser_session_id = Some(browser_session_id);
                state
                    .browsers
                    .write()
                    .await
                    .insert(browser_session_id, tx.clone());
                bound_browser = Some(browser_session_id);
                let (servers, expires_at) = state.ice_settings.for_trip(trip_session_id);
                send_json(
                    &tx,
                    &SignalMessage::IceConfig {
                        trip_session_id,
                        servers,
                        expires_at,
                    },
                );
                info!(%browser_session_id, %trip_session_id, "browser joined trip");
            }
            SignalMessage::Offer {
                trip_session_id, ..
            }
            | SignalMessage::Answer {
                trip_session_id, ..
            }
            | SignalMessage::IceCandidate {
                trip_session_id, ..
            } => {
                relay_to_peer(&state, trip_session_id, &tx, parsed).await;
            }
            SignalMessage::Heartbeat { .. } => {}
            SignalMessage::IceConfig { .. } => {}
            SignalMessage::Error { .. } => {}
        }
    }

    if let Some(id) = bound_control {
        state.controls.write().await.remove(&id);
        warn!(%id, "control disconnected");
    }
    if let Some(id) = bound_browser {
        state.browsers.write().await.remove(&id);
        warn!(%id, "browser disconnected");
    }
    writer.abort();
}

async fn relay_to_peer(
    state: &AppState,
    trip_session_id: TripSessionId,
    sender: &Tx,
    message: SignalMessage,
) {
    let registration = state.trips.read().await.get(&trip_session_id).cloned();
    let Some(registration) = registration else {
        send_json(
            sender,
            &SignalMessage::Error {
                code: "trip_not_found".into(),
                message: "trip session is not active".into(),
            },
        );
        return;
    };

    let control_tx = state
        .controls
        .read()
        .await
        .get(&registration.control_instance_id)
        .cloned();
    let browser_tx = match registration.browser_session_id {
        Some(id) => state.browsers.read().await.get(&id).cloned(),
        None => None,
    };

    for target in [control_tx, browser_tx].into_iter().flatten() {
        if !target.same_channel(sender) {
            send_json(&target, &message);
        }
    }
}

fn send_json(tx: &Tx, value: &SignalMessage) {
    if let Ok(text) = serde_json::to_string(value) {
        let _ = tx.send(Message::Text(text.into()));
    }
}
