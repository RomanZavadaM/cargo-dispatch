#![forbid(unsafe_code)]

use axum::{
    Json, Router,
    body::{Body, Bytes, to_bytes},
    extract::{Path, Request, State},
    http::{
        HeaderMap, HeaderName, HeaderValue, StatusCode,
        header::{AUTHORIZATION, CONTENT_LENGTH},
    },
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::{Mutex, RwLock, mpsc, oneshot},
    time::timeout,
};

const CONTROL_POLL_WAIT: Duration = Duration::from_secs(25);
const CONTROL_ONLINE_WINDOW: Duration = Duration::from_secs(55);
const QUEUE_SEND_WAIT: Duration = Duration::from_secs(2);
const CLIENT_RESPONSE_WAIT: Duration = Duration::from_secs(90);
const SESSION_QUEUE_DEPTH: usize = 64;

const REQUEST_ID_HEADER: &str = "x-cargo-relay-request-id";
const METHOD_HEADER: &str = "x-cargo-relay-method";
const PATH_HEADER: &str = "x-cargo-relay-path";
const STATUS_HEADER: &str = "x-cargo-relay-status";
const FORWARD_PREFIX: &str = "x-cargo-relay-header-";
const RESPONSE_PREFIX: &str = "x-cargo-relay-response-";

const FORWARDED_REQUEST_HEADERS: &[&str] = &[
    "content-type",
    "x-file-name",
    "x-created-at-unix-ms",
    "x-point-id",
    "accept",
    "range",
];

const FORWARDED_RESPONSE_HEADERS: &[&str] = &[
    "content-type",
    "content-disposition",
    "cache-control",
    "location",
];

#[derive(Clone)]
pub struct GatewayState {
    inner: Arc<GatewayInner>,
}

struct GatewayInner {
    control_token: Arc<str>,
    sessions: RwLock<HashMap<String, Arc<RelaySession>>>,
    next_request_id: AtomicU64,
    max_relay_bytes: usize,
}

struct RelaySession {
    queue_tx: mpsc::Sender<QueuedRequest>,
    queue_rx: Mutex<mpsc::Receiver<QueuedRequest>>,
    pending: Mutex<HashMap<u64, oneshot::Sender<RelayResponse>>>,
    last_control_seen: Mutex<Instant>,
}

struct QueuedRequest {
    id: u64,
    method: String,
    path_and_query: String,
    headers: Vec<(String, String)>,
    body: Bytes,
    response_tx: oneshot::Sender<RelayResponse>,
}

struct RelayResponse {
    status: StatusCode,
    headers: Vec<(String, String)>,
    body: Bytes,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct SessionStatusResponse {
    registered: bool,
    control_online: bool,
    queued_requests: usize,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

impl RelaySession {
    fn new() -> Self {
        let (queue_tx, queue_rx) = mpsc::channel(SESSION_QUEUE_DEPTH);
        Self {
            queue_tx,
            queue_rx: Mutex::new(queue_rx),
            pending: Mutex::new(HashMap::new()),
            last_control_seen: Mutex::new(Instant::now()),
        }
    }

    async fn touch_control(&self) {
        *self.last_control_seen.lock().await = Instant::now();
    }

    async fn control_online(&self) -> bool {
        self.last_control_seen.lock().await.elapsed() <= CONTROL_ONLINE_WINDOW
    }

    fn queued_requests(&self) -> usize {
        self.queue_tx.max_capacity() - self.queue_tx.capacity()
    }
}

impl GatewayState {
    pub fn new(control_token: impl Into<String>, max_relay_bytes: usize) -> Self {
        Self {
            inner: Arc::new(GatewayInner {
                control_token: Arc::from(control_token.into()),
                sessions: RwLock::new(HashMap::new()),
                next_request_id: AtomicU64::new(1),
                max_relay_bytes,
            }),
        }
    }

    pub fn max_relay_bytes(&self) -> usize {
        self.inner.max_relay_bytes
    }

    async fn register_session(&self, session: String) -> Arc<RelaySession> {
        let relay = Arc::new(RelaySession::new());
        self.inner
            .sessions
            .write()
            .await
            .insert(session, relay.clone());
        relay
    }

    async fn session(&self, session: &str) -> Option<Arc<RelaySession>> {
        self.inner.sessions.read().await.get(session).cloned()
    }

    async fn unregister_session(&self, session: &str) -> bool {
        self.inner.sessions.write().await.remove(session).is_some()
    }

    fn next_request_id(&self) -> u64 {
        self.inner.next_request_id.fetch_add(1, Ordering::Relaxed)
    }

    fn authorized_control(&self, headers: &HeaderMap) -> bool {
        let expected = format!("Bearer {}", self.inner.control_token);
        headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value == expected)
    }
}

pub fn router(state: GatewayState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v1/session/{session}", get(session_status))
        .route(
            "/v1/control/{session}",
            put(register_control).delete(unregister_control),
        )
        .route("/v1/control/{session}/next", get(control_next))
        .route(
            "/v1/control/{session}/response/{request_id}",
            post(control_response),
        )
        .fallback(public_relay)
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

fn valid_session_token(value: &str) -> bool {
    (16..=256).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
}

fn session_from_public_path(path: &str) -> Option<&str> {
    let parts = path
        .trim_start_matches('/')
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    match parts.as_slice() {
        ["driver", session] => Some(session),
        ["api", "trip", session, ..] => Some(session),
        _ => None,
    }
}

async fn session_status(
    State(state): State<GatewayState>,
    Path(session): Path<String>,
) -> Response {
    if !valid_session_token(&session) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "invalid session token",
            }),
        )
            .into_response();
    }

    let Some(relay) = state.session(&session).await else {
        return Json(SessionStatusResponse {
            registered: false,
            control_online: false,
            queued_requests: 0,
        })
        .into_response();
    };

    Json(SessionStatusResponse {
        registered: true,
        control_online: relay.control_online().await,
        queued_requests: relay.queued_requests(),
    })
    .into_response()
}

async fn register_control(
    State(state): State<GatewayState>,
    Path(session): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !state.authorized_control(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !valid_session_token(&session) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    state.register_session(session).await;
    StatusCode::NO_CONTENT.into_response()
}

async fn unregister_control(
    State(state): State<GatewayState>,
    Path(session): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !state.authorized_control(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    if state.unregister_session(&session).await {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn control_next(
    State(state): State<GatewayState>,
    Path(session): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !state.authorized_control(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(relay) = state.session(&session).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    relay.touch_control().await;
    let queued = {
        let mut receiver = relay.queue_rx.lock().await;
        match timeout(CONTROL_POLL_WAIT, receiver.recv()).await {
            Ok(Some(request)) => request,
            Ok(None) => return StatusCode::GONE.into_response(),
            Err(_) => return StatusCode::NO_CONTENT.into_response(),
        }
    };

    let QueuedRequest {
        id,
        method,
        path_and_query,
        headers: forwarded_headers,
        body,
        response_tx,
    } = queued;

    relay.pending.lock().await.insert(id, response_tx);

    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;

    let response_headers = response.headers_mut();
    insert_text_header(response_headers, REQUEST_ID_HEADER, &id.to_string());
    insert_text_header(response_headers, METHOD_HEADER, &method);
    insert_text_header(response_headers, PATH_HEADER, &path_and_query);

    for (name, value) in forwarded_headers {
        let relay_name = format!("{FORWARD_PREFIX}{name}");
        insert_text_header(response_headers, &relay_name, &value);
    }

    response
}

async fn control_response(
    State(state): State<GatewayState>,
    Path((session, request_id)): Path<(String, u64)>,
    headers: HeaderMap,
    request: Request,
) -> Response {
    if !state.authorized_control(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(relay) = state.session(&session).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    relay.touch_control().await;

    let status = headers
        .get(STATUS_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u16>().ok())
        .and_then(|value| StatusCode::from_u16(value).ok())
        .unwrap_or(StatusCode::BAD_GATEWAY);

    let body = match to_bytes(request.into_body(), state.max_relay_bytes()).await {
        Ok(body) => body,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };

    let response_headers = FORWARDED_RESPONSE_HEADERS
        .iter()
        .filter_map(|name| {
            let relay_name = format!("{RESPONSE_PREFIX}{name}");
            headers
                .get(relay_name.as_str())
                .and_then(|value| value.to_str().ok())
                .map(|value| ((*name).to_owned(), value.to_owned()))
        })
        .collect::<Vec<_>>();

    let sender = relay.pending.lock().await.remove(&request_id);
    let Some(sender) = sender else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let _ = sender.send(RelayResponse {
        status,
        headers: response_headers,
        body,
    });
    StatusCode::NO_CONTENT.into_response()
}

async fn public_relay(State(state): State<GatewayState>, request: Request) -> Response {
    let Some(session) = session_from_public_path(request.uri().path()).map(str::to_owned) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !valid_session_token(&session) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let Some(relay) = state.session(&session).await else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Control session is not registered",
        )
            .into_response();
    };
    if !relay.control_online().await {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Control session is offline",
        )
            .into_response();
    }

    let method = request.method().as_str().to_owned();
    let path_and_query = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or(request.uri().path())
        .to_owned();
    let forwarded_headers = collect_forward_headers(request.headers());
    let body = match to_bytes(request.into_body(), state.max_relay_bytes()).await {
        Ok(body) => body,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };

    let request_id = state.next_request_id();
    let (response_tx, response_rx) = oneshot::channel();
    let queued = QueuedRequest {
        id: request_id,
        method,
        path_and_query,
        headers: forwarded_headers,
        body,
        response_tx,
    };

    match timeout(QUEUE_SEND_WAIT, relay.queue_tx.send(queued)).await {
        Ok(Ok(())) => {}
        _ => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Control relay queue is unavailable",
            )
                .into_response();
        }
    }

    let relay_response = match timeout(CLIENT_RESPONSE_WAIT, response_rx).await {
        Ok(Ok(response)) => response,
        _ => {
            relay.pending.lock().await.remove(&request_id);
            return (StatusCode::GATEWAY_TIMEOUT, "Control did not answer").into_response();
        }
    };

    client_response(relay_response)
}

fn collect_forward_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    FORWARDED_REQUEST_HEADERS
        .iter()
        .filter_map(|name| {
            headers
                .get(*name)
                .and_then(|value| value.to_str().ok())
                .map(|value| ((*name).to_owned(), value.to_owned()))
        })
        .collect()
}

fn client_response(relay: RelayResponse) -> Response {
    let body_len = relay.body.len();
    let mut response = Response::new(Body::from(relay.body));
    *response.status_mut() = relay.status;

    for (name, value) in relay.headers {
        if let (Ok(name), Ok(value)) = (HeaderName::try_from(name), HeaderValue::try_from(value)) {
            response.headers_mut().insert(name, value);
        }
    }

    if !response.headers().contains_key(CONTENT_LENGTH)
        && let Ok(value) = HeaderValue::from_str(&body_len.to_string())
    {
        response.headers_mut().insert(CONTENT_LENGTH, value);
    }

    response
}

fn insert_text_header(headers: &mut HeaderMap, name: &str, value: &str) {
    if let (Ok(name), Ok(value)) = (HeaderName::try_from(name), HeaderValue::try_from(value)) {
        headers.insert(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_paths_resolve_only_driver_and_trip_sessions() {
        assert_eq!(
            session_from_public_path("/driver/abcdefghijklmnop"),
            Some("abcdefghijklmnop")
        );
        assert_eq!(
            session_from_public_path("/api/trip/abcdefghijklmnop/location"),
            Some("abcdefghijklmnop")
        );
        assert_eq!(session_from_public_path("/healthz"), None);
        assert_eq!(
            session_from_public_path("/v1/control/abcdefghijklmnop"),
            None
        );
    }

    #[test]
    fn session_token_rejects_paths_and_short_values() {
        assert!(valid_session_token("abcdefghijklmnop"));
        assert!(valid_session_token("abcDEF0123456789-_.~"));
        assert!(!valid_session_token("short"));
        assert!(!valid_session_token("abcdefghijklmnop/extra"));
    }

    #[tokio::test]
    async fn registering_again_replaces_old_relay_session() {
        let state = GatewayState::new("control-secret-control-secret-0001", 1024);
        let first = state.register_session("abcdefghijklmnop".to_owned()).await;
        let second = state.register_session("abcdefghijklmnop".to_owned()).await;
        assert!(!Arc::ptr_eq(&first, &second));
        assert!(state.session("abcdefghijklmnop").await.is_some());
    }

    #[test]
    fn request_header_forwarding_is_explicit() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/pdf"),
        );
        headers.insert(
            HeaderName::from_static("x-file-name"),
            HeaderValue::from_static("sample.pdf"),
        );
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("should-not-pass"),
        );

        let forwarded = collect_forward_headers(&headers);
        assert!(forwarded.iter().any(|(name, _)| name == "content-type"));
        assert!(forwarded.iter().any(|(name, _)| name == "x-file-name"));
        assert!(!forwarded.iter().any(|(name, _)| name == "authorization"));
    }
}
