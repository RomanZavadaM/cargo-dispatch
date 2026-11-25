#![forbid(unsafe_code)]

use reqwest::{
    Client, Method, StatusCode, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
};
use std::{
    error::Error,
    fmt,
    net::IpAddr,
    time::Duration,
};
use tokio::{sync::watch, time::sleep};

pub const CONTROL_GATEWAY_CLIENT_PROTOCOL_VERSION: u32 = 1;

const REQUEST_ID_HEADER: &str = "x-cargo-relay-request-id";
const METHOD_HEADER: &str = "x-cargo-relay-method";
const PATH_HEADER: &str = "x-cargo-relay-path";
const STATUS_HEADER: &str = "x-cargo-relay-status";
const REQUEST_HEADER_PREFIX: &str = "x-cargo-relay-header-";
const RESPONSE_HEADER_PREFIX: &str = "x-cargo-relay-response-";

const RESPONSE_HEADERS: &[&str] = &[
    "content-type",
    "content-disposition",
    "cache-control",
    "location",
];

const DEFAULT_MAX_RELAY_BYTES: usize = 64 * 1024 * 1024;
const MAX_RELAY_BYTES: usize = 512 * 1024 * 1024;
const POLL_HTTP_TIMEOUT: Duration = Duration::from_secs(35);
const REQUEST_HTTP_TIMEOUT: Duration = Duration::from_secs(95);
const RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct GatewayRelayConfig {
    gateway_base_url: Url,
    local_control_base_url: Url,
    control_secret: String,
    session_token: String,
    max_relay_bytes: usize,
}

impl GatewayRelayConfig {
    pub fn new(
        gateway_base_url: &str,
        local_control_base_url: &str,
        control_secret: impl Into<String>,
        session_token: impl Into<String>,
    ) -> Result<Self, GatewayClientError> {
        Self::with_max_relay_bytes(
            gateway_base_url,
            local_control_base_url,
            control_secret,
            session_token,
            DEFAULT_MAX_RELAY_BYTES,
        )
    }

    pub fn with_max_relay_bytes(
        gateway_base_url: &str,
        local_control_base_url: &str,
        control_secret: impl Into<String>,
        session_token: impl Into<String>,
        max_relay_bytes: usize,
    ) -> Result<Self, GatewayClientError> {
        let gateway_base_url = normalized_base_url(gateway_base_url, false)?;
        let local_control_base_url = normalized_base_url(local_control_base_url, true)?;
        let control_secret = control_secret.into();
        let session_token = session_token.into();

        if control_secret.len() < 32 {
            return Err(GatewayClientError::InvalidConfig(
                "Control gateway secret must contain at least 32 characters",
            ));
        }
        if !valid_session_token(&session_token) {
            return Err(GatewayClientError::InvalidConfig(
                "Driver session token is not valid for gateway relay",
            ));
        }
        if max_relay_bytes == 0 || max_relay_bytes > MAX_RELAY_BYTES {
            return Err(GatewayClientError::InvalidConfig(
                "relay byte limit must be between 1 and 536870912",
            ));
        }

        Ok(Self {
            gateway_base_url,
            local_control_base_url,
            control_secret,
            session_token,
            max_relay_bytes,
        })
    }

    pub fn gateway_base_url(&self) -> &Url {
        &self.gateway_base_url
    }

    pub fn local_control_base_url(&self) -> &Url {
        &self.local_control_base_url
    }

    pub fn session_token(&self) -> &str {
        &self.session_token
    }

    pub fn public_driver_url(&self) -> Result<Url, GatewayClientError> {
        endpoint_url(
            &self.gateway_base_url,
            &format!("driver/{}", self.session_token),
        )
    }
}

#[derive(Debug)]
pub enum GatewayClientError {
    InvalidConfig(&'static str),
    InvalidRelayRequest(&'static str),
    Unauthorized,
    HttpStatus {
        operation: &'static str,
        status: StatusCode,
    },
    Http(reqwest::Error),
}

impl fmt::Display for GatewayClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => write!(formatter, "invalid gateway config: {message}"),
            Self::InvalidRelayRequest(message) => {
                write!(formatter, "invalid gateway relay request: {message}")
            }
            Self::Unauthorized => write!(formatter, "gateway rejected the Control secret"),
            Self::HttpStatus { operation, status } => {
                write!(formatter, "{operation} returned HTTP {status}")
            }
            Self::Http(error) => write!(formatter, "gateway HTTP error: {error}"),
        }
    }
}

impl Error for GatewayClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for GatewayClientError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayOnceOutcome {
    Relayed,
    NoRequest,
    SessionMissing,
}

#[derive(Clone)]
pub struct ControlGatewayClient {
    config: GatewayRelayConfig,
    gateway_http: Client,
    local_http: Client,
}

impl ControlGatewayClient {
    pub fn new(config: GatewayRelayConfig) -> Result<Self, GatewayClientError> {
        let gateway_http = Client::builder()
            .redirect(Policy::none())
            .timeout(REQUEST_HTTP_TIMEOUT)
            .build()?;
        let local_http = Client::builder()
            .redirect(Policy::none())
            .timeout(REQUEST_HTTP_TIMEOUT)
            .build()?;
        Ok(Self {
            config,
            gateway_http,
            local_http,
        })
    }

    pub fn config(&self) -> &GatewayRelayConfig {
        &self.config
    }

    pub async fn register(&self) -> Result<(), GatewayClientError> {
        let url = self.control_endpoint("")?;
        let response = self
            .gateway_http
            .put(url)
            .bearer_auth(&self.config.control_secret)
            .send()
            .await?;
        checked_empty_response("register Control session", response.status())
    }

    pub async fn unregister(&self) -> Result<(), GatewayClientError> {
        let url = self.control_endpoint("")?;
        let response = self
            .gateway_http
            .delete(url)
            .bearer_auth(&self.config.control_secret)
            .send()
            .await?;
        match response.status() {
            StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
            StatusCode::UNAUTHORIZED => Err(GatewayClientError::Unauthorized),
            status => Err(GatewayClientError::HttpStatus {
                operation: "unregister Control session",
                status,
            }),
        }
    }

    pub async fn relay_once(&self) -> Result<RelayOnceOutcome, GatewayClientError> {
        let url = self.control_endpoint("next")?;
        let response = self
            .gateway_http
            .get(url)
            .bearer_auth(&self.config.control_secret)
            .timeout(POLL_HTTP_TIMEOUT)
            .send()
            .await?;

        match response.status() {
            StatusCode::NO_CONTENT => return Ok(RelayOnceOutcome::NoRequest),
            StatusCode::NOT_FOUND => return Ok(RelayOnceOutcome::SessionMissing),
            StatusCode::UNAUTHORIZED => return Err(GatewayClientError::Unauthorized),
            StatusCode::OK => {}
            status => {
                return Err(GatewayClientError::HttpStatus {
                    operation: "poll gateway request",
                    status,
                });
            }
        }

        let request_id = required_header(response.headers(), REQUEST_ID_HEADER)?
            .parse::<u64>()
            .map_err(|_| GatewayClientError::InvalidRelayRequest("request id is not an integer"))?;
        let method = Method::from_bytes(required_header(response.headers(), METHOD_HEADER)?.as_bytes())
            .map_err(|_| GatewayClientError::InvalidRelayRequest("invalid HTTP method"))?;
        let path_and_query = required_header(response.headers(), PATH_HEADER)?.to_owned();

        if !relay_path_matches_session(&path_and_query, &self.config.session_token) {
            self.return_simple_error(
                request_id,
                StatusCode::BAD_REQUEST,
                b"gateway relay path rejected by Control",
            )
            .await?;
            return Ok(RelayOnceOutcome::Relayed);
        }

        if response
            .content_length()
            .is_some_and(|length| length > self.config.max_relay_bytes as u64)
        {
            self.return_simple_error(
                request_id,
                StatusCode::PAYLOAD_TOO_LARGE,
                b"gateway relay request too large",
            )
            .await?;
            return Ok(RelayOnceOutcome::Relayed);
        }

        let request_headers = forwarded_request_headers(response.headers());
        let body = response.bytes().await?;
        if body.len() > self.config.max_relay_bytes {
            self.return_simple_error(
                request_id,
                StatusCode::PAYLOAD_TOO_LARGE,
                b"gateway relay request too large",
            )
            .await?;
            return Ok(RelayOnceOutcome::Relayed);
        }

        let local_url = self
            .config
            .local_control_base_url
            .join(path_and_query.trim_start_matches('/'))
            .map_err(|_| GatewayClientError::InvalidRelayRequest("invalid local relay URL"))?;

        let mut local_request = self.local_http.request(method, local_url);
        for (name, value) in request_headers {
            local_request = local_request.header(name, value);
        }

        let local_response = match local_request.body(body).send().await {
            Ok(response) => response,
            Err(_) => {
                self.return_simple_error(
                    request_id,
                    StatusCode::BAD_GATEWAY,
                    b"local Control did not answer",
                )
                .await?;
                return Ok(RelayOnceOutcome::Relayed);
            }
        };

        let status = local_response.status();
        let response_headers = forwarded_response_headers(local_response.headers());
        if local_response
            .content_length()
            .is_some_and(|length| length > self.config.max_relay_bytes as u64)
        {
            self.return_simple_error(
                request_id,
                StatusCode::BAD_GATEWAY,
                b"local Control response too large",
            )
            .await?;
            return Ok(RelayOnceOutcome::Relayed);
        }
        let body = local_response.bytes().await?;
        if body.len() > self.config.max_relay_bytes {
            self.return_simple_error(
                request_id,
                StatusCode::BAD_GATEWAY,
                b"local Control response too large",
            )
            .await?;
            return Ok(RelayOnceOutcome::Relayed);
        }

        self.return_response(request_id, status, response_headers, body)
            .await?;
        Ok(RelayOnceOutcome::Relayed)
    }

    pub async fn run(&self, shutdown: watch::Receiver<bool>) -> Result<(), GatewayClientError> {
        self.register().await?;

        while !*shutdown.borrow() {
            match self.relay_once().await {
                Ok(RelayOnceOutcome::Relayed | RelayOnceOutcome::NoRequest) => {}
                Ok(RelayOnceOutcome::SessionMissing) => {
                    self.register().await?;
                }
                Err(GatewayClientError::Unauthorized) => {
                    let _ = self.unregister().await;
                    return Err(GatewayClientError::Unauthorized);
                }
                Err(_) => {
                    sleep(RETRY_DELAY).await;
                }
            }
        }

        self.unregister().await
    }

    fn control_endpoint(&self, suffix: &str) -> Result<Url, GatewayClientError> {
        let mut path = format!("v1/control/{}", self.config.session_token);
        if !suffix.is_empty() {
            path.push('/');
            path.push_str(suffix);
        }
        endpoint_url(&self.config.gateway_base_url, &path)
    }

    async fn return_simple_error(
        &self,
        request_id: u64,
        status: StatusCode,
        message: &'static [u8],
    ) -> Result<(), GatewayClientError> {
        self.return_response(
            request_id,
            status,
            vec![(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("text/plain; charset=utf-8"),
            )],
            message.to_vec(),
        )
        .await
    }

    async fn return_response(
        &self,
        request_id: u64,
        status: StatusCode,
        headers: Vec<(HeaderName, HeaderValue)>,
        body: impl Into<reqwest::Body>,
    ) -> Result<(), GatewayClientError> {
        let url = self.control_endpoint(&format!("response/{request_id}"))?;
        let mut request = self
            .gateway_http
            .post(url)
            .bearer_auth(&self.config.control_secret)
            .header(STATUS_HEADER, status.as_u16().to_string());

        for (name, value) in headers {
            let relay_name = HeaderName::from_bytes(
                format!("{RESPONSE_HEADER_PREFIX}{}", name.as_str()).as_bytes(),
            )
            .map_err(|_| GatewayClientError::InvalidRelayRequest("invalid response header name"))?;
            request = request.header(relay_name, value);
        }

        let response = request.body(body).send().await?;
        checked_empty_response("return Control response", response.status())
    }
}

fn normalized_base_url(value: &str, require_loopback: bool) -> Result<Url, GatewayClientError> {
    let mut url = Url::parse(value)
        .map_err(|_| GatewayClientError::InvalidConfig("base URL is invalid"))?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(GatewayClientError::InvalidConfig(
            "base URL must use http or https",
        ));
    }
    if url.cannot_be_a_base() || url.host_str().is_none() {
        return Err(GatewayClientError::InvalidConfig(
            "base URL must contain a host",
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(GatewayClientError::InvalidConfig(
            "credentials are not allowed in base URLs",
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(GatewayClientError::InvalidConfig(
            "base URL must not contain query or fragment",
        ));
    }

    if require_loopback {
        if url.scheme() != "http" {
            return Err(GatewayClientError::InvalidConfig(
                "local Control base URL must use http",
            ));
        }
        let host = url
            .host_str()
            .ok_or(GatewayClientError::InvalidConfig(
                "local Control base URL has no host",
            ))?;
        let loopback = host == "localhost"
            || host
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback());
        if !loopback {
            return Err(GatewayClientError::InvalidConfig(
                "local Control upstream must be loopback-only",
            ));
        }
    }

    url.set_path("/");
    Ok(url)
}

fn valid_session_token(value: &str) -> bool {
    (16..=256).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
}

fn relay_path_matches_session(path_and_query: &str, session: &str) -> bool {
    if !path_and_query.starts_with('/') || path_and_query.starts_with("//") {
        return false;
    }

    let path = path_and_query
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(path_and_query);

    path == format!("/driver/{session}")
        || path == format!("/api/trip/{session}")
        || path.starts_with(&format!("/api/trip/{session}/"))
}

fn endpoint_url(base: &Url, relative: &str) -> Result<Url, GatewayClientError> {
    base.join(relative)
        .map_err(|_| GatewayClientError::InvalidConfig("gateway endpoint URL is invalid"))
}

fn required_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, GatewayClientError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .ok_or(GatewayClientError::InvalidRelayRequest(
            "required relay header missing",
        ))
}

fn forwarded_request_headers(headers: &HeaderMap) -> Vec<(HeaderName, HeaderValue)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let original = name.as_str().strip_prefix(REQUEST_HEADER_PREFIX)?;
            let name = HeaderName::from_bytes(original.as_bytes()).ok()?;
            Some((name, value.clone()))
        })
        .collect()
}

fn forwarded_response_headers(headers: &HeaderMap) -> Vec<(HeaderName, HeaderValue)> {
    RESPONSE_HEADERS
        .iter()
        .filter_map(|name| {
            let value = headers.get(*name)?.clone();
            let name = HeaderName::from_bytes(name.as_bytes()).ok()?;
            Some((name, value))
        })
        .collect()
}

fn checked_empty_response(
    operation: &'static str,
    status: StatusCode,
) -> Result<(), GatewayClientError> {
    match status {
        StatusCode::NO_CONTENT => Ok(()),
        StatusCode::UNAUTHORIZED => Err(GatewayClientError::Unauthorized),
        status => Err(GatewayClientError::HttpStatus { operation, status }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        extract::Request,
        response::Response,
    };
    use cargo_dispatch_addressing_gateway::{GatewayState, router as gateway_router};
    use tokio::net::TcpListener;

    const SECRET: &str = "control-secret-control-secret-0001";
    const SESSION: &str = "native-smoke-session-00000001";

    #[test]
    fn local_upstream_must_be_loopback() {
        assert!(
            GatewayRelayConfig::new(
                "https://gateway.example.test",
                "http://192.168.1.20:8080",
                SECRET,
                SESSION,
            )
            .is_err()
        );
        assert!(
            GatewayRelayConfig::new(
                "https://gateway.example.test",
                "http://127.0.0.1:8080",
                SECRET,
                SESSION,
            )
            .is_ok()
        );
    }

    #[test]
    fn relay_path_is_locked_to_the_current_driver_session() {
        assert!(relay_path_matches_session(
            "/driver/native-smoke-session-00000001",
            SESSION
        ));
        assert!(relay_path_matches_session(
            "/api/trip/native-smoke-session-00000001/location?fresh=1",
            SESSION
        ));
        assert!(!relay_path_matches_session(
            "/api/trip/some-other-session/location",
            SESSION
        ));
        assert!(!relay_path_matches_session("/healthz", SESSION));
        assert!(!relay_path_matches_session("//example.com/escape", SESSION));
    }

    async fn local_control(request: Request) -> Response {
        assert_eq!(request.method(), Method::GET);
        assert_eq!(
            request.uri().path_and_query().map(|value| value.as_str()),
            Some("/api/trip/native-smoke-session-00000001?fresh=1")
        );

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .header("cache-control", "no-store")
            .body(Body::from(r#"{"trip_id":"relay-smoke"}"#))
            .expect("local response")
    }

    #[tokio::test]
    async fn public_request_round_trips_through_gateway_and_control_client() {
        let local_listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind local Control");
        let local_address = local_listener.local_addr().expect("local Control address");
        let local_server = tokio::spawn(async move {
            axum::serve(local_listener, Router::new().fallback(local_control))
                .await
                .expect("local Control server");
        });

        let gateway_listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind gateway");
        let gateway_address = gateway_listener.local_addr().expect("gateway address");
        let gateway_server = tokio::spawn(async move {
            axum::serve(
                gateway_listener,
                gateway_router(GatewayState::new(SECRET, 4096)),
            )
            .await
            .expect("gateway server");
        });

        let config = GatewayRelayConfig::new(
            &format!("http://{gateway_address}"),
            &format!("http://{local_address}"),
            SECRET,
            SESSION,
        )
        .expect("relay config");
        let relay = ControlGatewayClient::new(config).expect("relay client");
        relay.register().await.expect("register relay");

        let relay_task = {
            let relay = relay.clone();
            tokio::spawn(async move { relay.relay_once().await })
        };

        let public = reqwest::Client::new()
            .get(format!(
                "http://{gateway_address}/api/trip/{SESSION}?fresh=1"
            ))
            .send()
            .await
            .expect("public gateway request");

        assert_eq!(public.status(), StatusCode::OK);
        assert_eq!(
            public
                .headers()
                .get("content-type")
                .expect("content type")
                .to_str()
                .expect("content type text"),
            "application/json"
        );
        assert_eq!(
            public
                .headers()
                .get("cache-control")
                .expect("cache control")
                .to_str()
                .expect("cache control text"),
            "no-store"
        );
        assert_eq!(
            public.text().await.expect("public body"),
            r#"{"trip_id":"relay-smoke"}"#
        );

        assert_eq!(
            relay_task
                .await
                .expect("relay task")
                .expect("relay result"),
            RelayOnceOutcome::Relayed
        );
        relay.unregister().await.expect("unregister relay");

        local_server.abort();
        gateway_server.abort();
    }
}
