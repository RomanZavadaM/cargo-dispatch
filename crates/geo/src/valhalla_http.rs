use crate::{
    GeoContractError, GeoWarning, NormalizedStatus, RouteRequest, RouteResult, RoutingIdentity,
    valhalla::{normalize_route_response, route_payload},
};
use core::fmt;
use reqwest::Url;
use serde_json::Value;
use std::time::Duration;

const STATUS_ENDPOINT: &str = "status";
const ROUTE_ENDPOINT: &str = "route";

#[derive(Debug, Clone)]
pub struct ValhallaRuntimeConfig {
    base_url: Url,
    map_dataset_id: String,
    connect_timeout: Duration,
    request_timeout: Duration,
}

impl ValhallaRuntimeConfig {
    pub fn new(
        base_url: &str,
        map_dataset_id: impl Into<String>,
        connect_timeout: Duration,
        request_timeout: Duration,
    ) -> Result<Self, ValhallaHttpError> {
        let mut base_url = Url::parse(base_url)
            .map_err(|_| ValhallaHttpError::InvalidConfig("Valhalla base URL is invalid"))?;

        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(ValhallaHttpError::InvalidConfig(
                "Valhalla base URL must use http or https",
            ));
        }
        if base_url.cannot_be_a_base() || base_url.host_str().is_none() {
            return Err(ValhallaHttpError::InvalidConfig(
                "Valhalla base URL must contain a host",
            ));
        }
        if !base_url.username().is_empty() || base_url.password().is_some() {
            return Err(ValhallaHttpError::InvalidConfig(
                "credentials are not allowed in the Valhalla base URL",
            ));
        }
        if base_url.query().is_some() || base_url.fragment().is_some() {
            return Err(ValhallaHttpError::InvalidConfig(
                "Valhalla base URL must not contain query or fragment",
            ));
        }
        if connect_timeout.is_zero() || request_timeout.is_zero() {
            return Err(ValhallaHttpError::InvalidConfig(
                "Valhalla timeouts must be greater than zero",
            ));
        }

        let map_dataset_id = map_dataset_id.into();
        if map_dataset_id.trim().is_empty() {
            return Err(ValhallaHttpError::InvalidConfig(
                "map dataset id must not be empty",
            ));
        }

        if !base_url.path().ends_with('/') {
            let normalized_path = format!("{}/", base_url.path());
            base_url.set_path(&normalized_path);
        }

        Ok(Self {
            base_url,
            map_dataset_id,
            connect_timeout,
            request_timeout,
        })
    }

    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    #[must_use]
    pub fn map_dataset_id(&self) -> &str {
        &self.map_dataset_id
    }

    #[must_use]
    pub const fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    #[must_use]
    pub const fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    fn endpoint(&self, relative: &str) -> Result<Url, ValhallaHttpError> {
        self.base_url
            .join(relative)
            .map_err(|_| ValhallaHttpError::InvalidConfig("Valhalla endpoint URL is invalid"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValhallaAvailability {
    pub status: NormalizedStatus,
    pub engine_version: Option<String>,
    pub tileset_last_modified: Option<u64>,
}

impl ValhallaAvailability {
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self.status, NormalizedStatus::Ok)
    }
}

#[derive(Debug, Clone)]
pub struct ValhallaHttpClient {
    config: ValhallaRuntimeConfig,
    http: reqwest::Client,
}

impl ValhallaHttpClient {
    pub fn new(config: ValhallaRuntimeConfig) -> Result<Self, ValhallaHttpError> {
        let http = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.request_timeout)
            .build()
            .map_err(ValhallaHttpError::ClientBuild)?;
        Ok(Self { config, http })
    }

    #[must_use]
    pub const fn config(&self) -> &ValhallaRuntimeConfig {
        &self.config
    }

    pub async fn availability(&self) -> Result<ValhallaAvailability, ValhallaHttpError> {
        let url = self.config.endpoint(STATUS_ENDPOINT)?;
        let response = match self.http.get(url).send().await {
            Ok(response) => response,
            Err(error) => {
                return Ok(ValhallaAvailability {
                    status: transport_status(&error),
                    engine_version: None,
                    tileset_last_modified: None,
                });
            }
        };

        let http_status = response.status().as_u16();
        if !(200..300).contains(&http_status) {
            return Ok(ValhallaAvailability {
                status: if http_status >= 500 {
                    NormalizedStatus::EngineUnavailable
                } else {
                    NormalizedStatus::EngineError
                },
                engine_version: None,
                tileset_last_modified: None,
            });
        }

        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                return Ok(ValhallaAvailability {
                    status: transport_status(&error),
                    engine_version: None,
                    tileset_last_modified: None,
                });
            }
        };
        let parsed: Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(_) => {
                return Ok(ValhallaAvailability {
                    status: NormalizedStatus::EngineError,
                    engine_version: None,
                    tileset_last_modified: None,
                });
            }
        };

        let engine_version = parsed
            .get("version")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let tileset_last_modified = parsed
            .get("tileset_last_modified")
            .and_then(Value::as_u64);

        let status = if engine_version.is_none() {
            NormalizedStatus::EngineError
        } else if tileset_last_modified.is_none() {
            NormalizedStatus::DatasetMissing
        } else {
            NormalizedStatus::Ok
        };

        Ok(ValhallaAvailability {
            status,
            engine_version,
            tileset_last_modified,
        })
    }

    pub async fn route(
        &self,
        request: &RouteRequest,
    ) -> Result<RouteResult, ValhallaHttpError> {
        let availability = self.availability().await?;
        if !availability.is_available() {
            return Err(ValhallaHttpError::Unavailable(availability.status));
        }

        let engine_version = availability
            .engine_version
            .as_deref()
            .ok_or(ValhallaHttpError::Unavailable(
                NormalizedStatus::EngineError,
            ))?;
        let url = self.config.endpoint(ROUTE_ENDPOINT)?;
        let response = match self.http.post(url).json(&route_payload(request)).send().await {
            Ok(response) => response,
            Err(error) => {
                return self.transport_result(request, engine_version, transport_status(&error));
            }
        };
        let http_status = response.status().as_u16();
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                return self.transport_result(request, engine_version, transport_status(&error));
            }
        };

        normalize_route_response(
            request,
            engine_version,
            &self.config.map_dataset_id,
            http_status,
            &body,
        )
        .map_err(ValhallaHttpError::GeoContract)
    }

    fn transport_result(
        &self,
        request: &RouteRequest,
        engine_version: &str,
        status: NormalizedStatus,
    ) -> Result<RouteResult, ValhallaHttpError> {
        let identity = RoutingIdentity::new(
            crate::valhalla::ENGINE_NAME,
            engine_version,
            &self.config.map_dataset_id,
            request.profile().profile_fingerprint(),
        )
        .map_err(ValhallaHttpError::GeoContract)?;

        Ok(RouteResult {
            status,
            identity,
            distance_m: None,
            duration_s: None,
            geometry: Vec::new(),
            snapped_points: Vec::new(),
            warnings: request.profile().warnings(),
        })
    }
}

fn transport_status(error: &reqwest::Error) -> NormalizedStatus {
    if error.is_timeout() {
        NormalizedStatus::Timeout
    } else {
        NormalizedStatus::EngineUnavailable
    }
}

#[derive(Debug)]
pub enum ValhallaHttpError {
    InvalidConfig(&'static str),
    ClientBuild(reqwest::Error),
    Unavailable(NormalizedStatus),
    GeoContract(GeoContractError),
}

impl fmt::Display for ValhallaHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => formatter.write_str(message),
            Self::ClientBuild(error) => write!(formatter, "cannot build Valhalla HTTP client: {error}"),
            Self::Unavailable(status) => write!(formatter, "Valhalla is unavailable: {status:?}"),
            Self::GeoContract(error) => write!(formatter, "Valhalla route contract failed: {error}"),
        }
    }
}

impl std::error::Error for ValhallaHttpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ClientBuild(error) => Some(error),
            Self::GeoContract(error) => Some(error),
            Self::InvalidConfig(_) | Self::Unavailable(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeoPoint, TollPreference, VehicleRoutingProfile};
    use axum::{
        Json, Router,
        http::StatusCode,
        routing::{get, post},
    };
    use serde_json::{Value, json};
    use tokio::{net::TcpListener, task::JoinHandle, time::sleep};

    fn profile() -> VehicleRoutingProfile {
        VehicleRoutingProfile {
            height_m: None,
            width_m: Some(2.4),
            length_m: Some(8.5),
            gross_weight_t: Some(12.0),
            axle_load_t: None,
            axle_count: Some(2),
            hazmat: false,
            hgv: true,
            exclude_unpaved: true,
            prefer_truck_routes: true,
            toll_preference: TollPreference::Allow,
            max_speed_kmh: None,
        }
    }

    fn request() -> RouteRequest {
        RouteRequest::new(
            vec![
                GeoPoint::new(49.8393, 24.0296).expect("point"),
                GeoPoint::new(49.8430, 24.0360).expect("point"),
                GeoPoint::new(49.8485, 24.0250).expect("point"),
            ],
            profile(),
        )
        .expect("request")
    }

    async fn spawn(app: Router) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
        let address = listener.local_addr().expect("address");
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock server");
        });
        (format!("http://{address}/"), task)
    }

    fn config(base_url: &str, request_timeout: Duration) -> ValhallaRuntimeConfig {
        ValhallaRuntimeConfig::new(
            base_url,
            "fixture-europe-2026-11",
            Duration::from_millis(250),
            request_timeout,
        )
        .expect("runtime config")
    }

    async fn status_ok() -> Json<Value> {
        Json(json!({
            "version": "3.9.2",
            "tileset_last_modified": 1_795_550_000_u64
        }))
    }

    async fn route_ok(Json(payload): Json<Value>) -> Json<Value> {
        assert_eq!(payload["costing"], "truck");
        assert!(
            payload["costing_options"]["truck"]
                .get("height")
                .is_none(),
            "missing vehicle dimensions must not be invented"
        );
        assert_eq!(payload["costing_options"]["truck"]["width"], 2.4);

        Json(json!({
            "trip": {
                "summary": {"length": 2.75, "time": 240.0},
                "legs": [
                    {"shape": "gk}`~A_ysyl@gfF_oK"},
                    {"shape": "orda~A_i`zl@wvInnT"}
                ]
            }
        }))
    }

    #[test]
    fn runtime_config_rejects_ambiguous_or_unsafe_values() {
        assert!(ValhallaRuntimeConfig::new(
            "file:///tmp/valhalla",
            "dataset",
            Duration::from_secs(1),
            Duration::from_secs(5)
        )
        .is_err());
        assert!(ValhallaRuntimeConfig::new(
            "https://user:pass@example.test",
            "dataset",
            Duration::from_secs(1),
            Duration::from_secs(5)
        )
        .is_err());
        assert!(ValhallaRuntimeConfig::new(
            "https://example.test?x=1",
            "dataset",
            Duration::from_secs(1),
            Duration::from_secs(5)
        )
        .is_err());
        assert!(ValhallaRuntimeConfig::new(
            "https://example.test",
            "   ",
            Duration::from_secs(1),
            Duration::from_secs(5)
        )
        .is_err());
        assert!(ValhallaRuntimeConfig::new(
            "https://example.test",
            "dataset",
            Duration::ZERO,
            Duration::from_secs(5)
        )
        .is_err());
    }

    #[tokio::test]
    async fn real_http_transport_uses_runtime_status_and_route_endpoint() {
        let app = Router::new()
            .route("/status", get(status_ok))
            .route("/route", post(route_ok));
        let (base_url, server) = spawn(app).await;
        let client = ValhallaHttpClient::new(config(&base_url, Duration::from_secs(2)))
            .expect("client");

        let availability = client.availability().await.expect("availability");
        assert!(availability.is_available());
        assert_eq!(availability.engine_version.as_deref(), Some("3.9.2"));
        assert_eq!(
            availability.tileset_last_modified,
            Some(1_795_550_000_u64)
        );

        let result = client.route(&request()).await.expect("route");
        assert_eq!(result.status, NormalizedStatus::Ok);
        assert_eq!(result.identity.engine(), "valhalla");
        assert_eq!(result.identity.engine_version(), "3.9.2");
        assert_eq!(
            result.identity.map_dataset_id(),
            "fixture-europe-2026-11"
        );
        assert_eq!(result.distance_m, Some(2750.0));
        assert_eq!(result.duration_s, Some(240.0));
        assert_eq!(result.geometry.len(), 3);
        assert_eq!(
            result.warnings,
            vec![
                GeoWarning::MissingVehicleDimension(crate::VehicleDimension::Height),
                GeoWarning::MissingVehicleDimension(crate::VehicleDimension::AxleLoad),
            ]
        );

        server.abort();
    }

    async fn status_without_tiles() -> Json<Value> {
        Json(json!({"version": "3.9.2"}))
    }

    #[tokio::test]
    async fn status_without_tiles_is_dataset_missing() {
        let app = Router::new().route("/status", get(status_without_tiles));
        let (base_url, server) = spawn(app).await;
        let client = ValhallaHttpClient::new(config(&base_url, Duration::from_secs(2)))
            .expect("client");

        let availability = client.availability().await.expect("availability");
        assert_eq!(availability.status, NormalizedStatus::DatasetMissing);
        assert_eq!(availability.engine_version.as_deref(), Some("3.9.2"));

        server.abort();
    }

    async fn status_slow() -> Json<Value> {
        sleep(Duration::from_millis(150)).await;
        status_ok().await
    }

    #[tokio::test]
    async fn status_timeout_is_reported_without_external_network() {
        let app = Router::new().route("/status", get(status_slow));
        let (base_url, server) = spawn(app).await;
        let client = ValhallaHttpClient::new(config(&base_url, Duration::from_millis(25)))
            .expect("client");

        let availability = client.availability().await.expect("availability");
        assert_eq!(availability.status, NormalizedStatus::Timeout);
        assert!(availability.engine_version.is_none());

        server.abort();
    }

    async fn route_slow(Json(_payload): Json<Value>) -> Json<Value> {
        sleep(Duration::from_millis(150)).await;
        route_ok(Json(json!({
            "costing": "truck",
            "costing_options": {"truck": {"width": 2.4}}
        })))
        .await
    }

    #[tokio::test]
    async fn route_timeout_keeps_runtime_identity_but_no_geometry() {
        let app = Router::new()
            .route("/status", get(status_ok))
            .route("/route", post(route_slow));
        let (base_url, server) = spawn(app).await;
        let client = ValhallaHttpClient::new(config(&base_url, Duration::from_millis(25)))
            .expect("client");

        let result = client.route(&request()).await.expect("normalized timeout");
        assert_eq!(result.status, NormalizedStatus::Timeout);
        assert_eq!(result.identity.engine_version(), "3.9.2");
        assert_eq!(
            result.identity.map_dataset_id(),
            "fixture-europe-2026-11"
        );
        assert!(result.geometry.is_empty());
        assert_eq!(result.distance_m, None);

        server.abort();
    }

    #[tokio::test]
    async fn server_unavailable_is_normalized() {
        async fn unavailable() -> (StatusCode, Json<Value>) {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"status": "down"})),
            )
        }

        let app = Router::new().route("/status", get(unavailable));
        let (base_url, server) = spawn(app).await;
        let client = ValhallaHttpClient::new(config(&base_url, Duration::from_secs(2)))
            .expect("client");

        let availability = client.availability().await.expect("availability");
        assert_eq!(
            availability.status,
            NormalizedStatus::EngineUnavailable
        );

        server.abort();
    }
}
