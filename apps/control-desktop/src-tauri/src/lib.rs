#![forbid(unsafe_code)]

mod restore;

use cargo_dispatch_contracts::{
    TripNavigationTarget, TripPlanningBasis, TripPointKind, TripRoutePoint,
};
use cargo_dispatch_control_gateway_client::{ControlGatewayClient, GatewayRelayConfig};
use cargo_dispatch_control_ui_contracts::{
    ConfigureRoutingInput, ControlUiStatus, CreateManualTripInput, DashboardActualContext,
    DashboardDocument, DashboardDocumentDirection, DashboardLocation, DashboardNavigation,
    DashboardPlanningBasis, DashboardPlanningCargo, DashboardPoint, DashboardRoadGeometry,
    DashboardRoadGeometryPoint, DashboardRoutingIdentity, DashboardVehicleRoutingProfile,
    GatewayStatus, ListenerMode, ListenerStatus, OperatorDocumentInput, PickedFile,
    RegisterTripInput, ReorderRouteInput, RoutingAvailabilityState, RoutingProbeResult,
    RoutingStatus, RoutingTollPreference, SaveDocumentInput, SessionInput, SessionStatus,
    StartGatewayInput, StartLanInput, TripDashboard, UpdateRoutingContextInput,
    UpdateSessionMetadataInput,
};
use cargo_dispatch_control_web::{
    ControlAccessMode, ControlBrowserState, DriverTripSnapshot, LoopbackControlServer,
    local_lan_addresses,
};
use cargo_dispatch_domain::{DocumentId, DriverEndpointId, PointId, TripId};
use cargo_dispatch_geo::{
    NormalizedStatus, TollPreference, VehicleRoutingProfile, valhalla_http::ValhallaRuntimeConfig,
};
use cargo_dispatch_trip_files::{
    ActiveTripSessionMetadata, DocumentDirection, DocumentStoreDraft, ManualTripCreateDraft,
    TripFolder,
};
use std::{
    collections::BTreeMap,
    fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;
use tokio::{
    sync::{Mutex as AsyncMutex, watch},
    task::JoinHandle,
};

#[derive(Debug, Clone)]
struct RegisteredSession {
    trip_id: String,
    token: String,
    operator_display_name: Option<String>,
    endpoint_id: Option<DriverEndpointId>,
    endpoint_label: Option<String>,
    vehicle_label: Option<String>,
    vehicle_type: Option<String>,
}

struct GatewayRelayTask {
    client: ControlGatewayClient,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
    public_driver_url: String,
}

struct GatewayRelayRuntime {
    base_url: String,
    internal_server: LoopbackControlServer,
    relays: BTreeMap<String, GatewayRelayTask>,
}

struct DesktopControlService {
    browser: ControlBrowserState,
    server: AsyncMutex<Option<LoopbackControlServer>>,
    gateway: AsyncMutex<Option<GatewayRelayRuntime>>,
    sessions: Mutex<BTreeMap<String, RegisteredSession>>,
    /// Where connected trips are remembered; `None` keeps nothing.
    memory: Option<PathBuf>,
    memory_lock: Mutex<()>,
}

impl Default for DesktopControlService {
    fn default() -> Self {
        Self {
            browser: ControlBrowserState::new(),
            server: AsyncMutex::new(None),
            gateway: AsyncMutex::new(None),
            sessions: Mutex::new(BTreeMap::new()),
            memory: None,
            memory_lock: Mutex::new(()),
        }
    }
}

fn driver_url(address: SocketAddr, token: &str) -> String {
    format!("http://{address}/driver/{token}")
}

fn session_window_label(token: &str) -> String {
    let suffix = token
        .chars()
        .filter(|value| value.is_ascii_alphanumeric())
        .take(40)
        .collect::<String>();
    format!("trip-session-{suffix}")
}

fn parse_id_bytes(value: &str, label: &str) -> Result<[u8; 16], String> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{label} має містити рівно 32 hex-символи"));
    }
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| format!("Некоректний {label}"))?;
    }
    Ok(bytes)
}

fn point_id(value: &str) -> Result<PointId, String> {
    PointId::from_bytes(parse_id_bytes(value, "Point ID")?)
        .map_err(|error| format!("Некоректний Point ID: {error:?}"))
}

fn document_id(value: &str) -> Result<DocumentId, String> {
    DocumentId::from_bytes(parse_id_bytes(value, "Document ID")?)
        .map_err(|error| format!("Некоректний Document ID: {error:?}"))
}

fn trip_id(value: &str) -> Result<TripId, String> {
    TripId::from_bytes(parse_id_bytes(value, "Trip ID")?)
        .map_err(|error| format!("Некоректний Trip ID: {error:?}"))
}

fn coordinate_e7(value: f64, max_abs: f64, label: &str) -> Result<i32, String> {
    if !value.is_finite() || value.abs() > max_abs {
        return Err(format!("Некоректна координата {label}"));
    }
    let scaled = (value * 10_000_000.0).round();
    if scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
        return Err(format!("Координата {label} поза допустимим діапазоном"));
    }
    Ok(scaled as i32)
}

fn driver_endpoint_id(value: &str) -> Result<DriverEndpointId, String> {
    DriverEndpointId::from_bytes(parse_id_bytes(value, "Driver endpoint ID")?)
        .map_err(|error| format!("Некоректний Driver endpoint ID: {error:?}"))
}

fn valid_session_token(token: &str) -> bool {
    (16..=128).contains(&token.len())
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn id_hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn session_display_name(session: &RegisteredSession) -> String {
    session
        .operator_display_name
        .clone()
        .or_else(|| session.vehicle_label.clone())
        .or_else(|| session.endpoint_label.clone())
        .unwrap_or_else(|| format!("trip-{}", &session.trip_id[..8.min(session.trip_id.len())]))
}

fn routing_profile_view(profile: &VehicleRoutingProfile) -> DashboardVehicleRoutingProfile {
    DashboardVehicleRoutingProfile {
        height_m: profile.height_m,
        width_m: profile.width_m,
        length_m: profile.length_m,
        gross_weight_t: profile.gross_weight_t,
        axle_load_t: profile.axle_load_t,
        axle_count: profile.axle_count,
        hazmat: profile.hazmat,
        hgv: profile.hgv,
        exclude_unpaved: profile.exclude_unpaved,
        prefer_truck_routes: profile.prefer_truck_routes,
        toll_preference: match profile.toll_preference {
            TollPreference::Allow => RoutingTollPreference::Allow,
            TollPreference::PreferAvoid => RoutingTollPreference::PreferAvoid,
            TollPreference::Avoid => RoutingTollPreference::Avoid,
        },
        max_speed_kmh: profile.max_speed_kmh,
    }
}

fn routing_profile_from_view(
    profile: DashboardVehicleRoutingProfile,
) -> Result<VehicleRoutingProfile, String> {
    let profile = VehicleRoutingProfile {
        height_m: profile.height_m,
        width_m: profile.width_m,
        length_m: profile.length_m,
        gross_weight_t: profile.gross_weight_t,
        axle_load_t: profile.axle_load_t,
        axle_count: profile.axle_count,
        hazmat: profile.hazmat,
        hgv: profile.hgv,
        exclude_unpaved: profile.exclude_unpaved,
        prefer_truck_routes: profile.prefer_truck_routes,
        toll_preference: match profile.toll_preference {
            RoutingTollPreference::Allow => TollPreference::Allow,
            RoutingTollPreference::PreferAvoid => TollPreference::PreferAvoid,
            RoutingTollPreference::Avoid => TollPreference::Avoid,
        },
        max_speed_kmh: profile.max_speed_kmh,
    };
    profile
        .validate_for_routing()
        .map_err(|error| format!("Некоректний routing profile: {error}"))?;
    Ok(profile)
}

fn mime_for_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
    .to_owned()
}

fn dashboard(
    snapshot: DriverTripSnapshot,
    planning_basis: Option<TripPlanningBasis>,
    actual_context: ActiveTripSessionMetadata,
    active_routing_profile: Option<VehicleRoutingProfile>,
) -> Result<TripDashboard, String> {
    let documents = snapshot
        .documents
        .into_iter()
        .map(|document| {
            let direction = match document.direction.as_str() {
                "from_operator" => DashboardDocumentDirection::FromOperator,
                "from_driver" => DashboardDocumentDirection::FromDriver,
                other => return Err(format!("Невідомий напрямок документа: {other}")),
            };
            Ok(DashboardDocument {
                document_id: document.document_id,
                direction,
                original_name: document.original_name,
                mime_type: document.mime_type,
                size: document.size,
                point_id: document.point_id,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(TripDashboard {
        trip_id: snapshot.trip_id,
        trip_version: snapshot.trip_version,
        local_route_revision: snapshot.local_route_revision,
        planning_basis: planning_basis.map(|basis| DashboardPlanningBasis {
            vehicle_type: basis.vehicle_type,
            cargos: basis
                .cargos
                .into_iter()
                .map(|cargo| DashboardPlanningCargo {
                    cargo_id: id_hex(cargo.cargo_id.as_bytes()),
                    title: cargo.title,
                    weight_kg: cargo.weight_kg,
                    pallets: cargo.pallets,
                    pickup_point_id: cargo.pickup_point_id.map(|point| id_hex(point.as_bytes())),
                    unload_point_id: cargo.unload_point_id.map(|point| id_hex(point.as_bytes())),
                })
                .collect(),
        }),
        actual_context: DashboardActualContext {
            display_name: actual_context.display_name,
            endpoint_id: actual_context
                .endpoint_id
                .map(|endpoint| id_hex(endpoint.as_bytes())),
            endpoint_label: actual_context.endpoint_label,
            vehicle_label: actual_context.vehicle_label,
            vehicle_type: actual_context.vehicle_type,
            routing_profile: active_routing_profile.as_ref().map(routing_profile_view),
        },
        points: snapshot
            .points
            .into_iter()
            .map(|point| DashboardPoint {
                point_id: point.point_id,
                sequence: point.sequence,
                title: point.title,
                completed: point.completed,
                navigation: point.navigation.map(|navigation| DashboardNavigation {
                    latitude: navigation.latitude,
                    longitude: navigation.longitude,
                    address: navigation.address,
                }),
            })
            .collect(),
        road_geometry: snapshot
            .road_geometry
            .map(|geometry| DashboardRoadGeometry {
                routing: DashboardRoutingIdentity {
                    engine: geometry.routing.engine,
                    engine_version: geometry.routing.engine_version,
                    map_dataset_id: geometry.routing.map_dataset_id,
                    profile_fingerprint: geometry.routing.profile_fingerprint,
                },
                points: geometry
                    .points
                    .into_iter()
                    .map(|point| DashboardRoadGeometryPoint {
                        latitude: point.latitude,
                        longitude: point.longitude,
                    })
                    .collect(),
            }),
        documents,
        latest_location: snapshot.latest_location.map(|location| DashboardLocation {
            latitude: location.latitude,
            longitude: location.longitude,
            accuracy_m: location.accuracy_m,
            observed_at_unix_ms: location.observed_at_unix_ms,
        }),
    })
}

impl DesktopControlService {
    async fn ensure_session_available(
        &self,
        token: &str,
        trip_id_text: Option<&str>,
    ) -> Result<(), String> {
        if self.gateway.lock().await.is_some() {
            return Err(
                "Спочатку вимкніть публічну Gateway-трансляцію, потім додайте новий рейс."
                    .to_owned(),
            );
        }
        if !valid_session_token(token) {
            return Err(
                "Session token має містити 16–128 латинських літер, цифр, '-' або '_'".to_owned(),
            );
        }
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?;
        if sessions.contains_key(token) {
            return Err("Цей token уже зареєстрований".to_owned());
        }
        if let Some(trip_id_text) = trip_id_text
            && sessions
                .values()
                .any(|session| session.trip_id == trip_id_text)
        {
            return Err("Цей рейс уже зареєстрований у Control".to_owned());
        }
        Ok(())
    }

    async fn create_manual_trip(
        &self,
        input: CreateManualTripInput,
    ) -> Result<ControlUiStatus, String> {
        let folder_path = PathBuf::from(input.folder_path.trim());
        if folder_path.as_os_str().is_empty() {
            return Err("Потрібно вказати папку нового рейсу".to_owned());
        }
        if input.points.is_empty() {
            return Err("Додайте хоча б одну точку рейсу".to_owned());
        }
        let parsed_trip_id = trip_id(input.trip_id.trim())?;
        let token = input.token.trim().to_owned();
        let trip_id_text = id_hex(parsed_trip_id.as_bytes());
        self.ensure_session_available(&token, Some(&trip_id_text))
            .await?;
        let route_points = input
            .points
            .into_iter()
            .enumerate()
            .map(|(index, point)| {
                let title = point.title.trim();
                if title.is_empty() {
                    return Err("Кожна точка рейсу повинна мати назву".to_owned());
                }
                let sequence =
                    u32::try_from(index + 1).map_err(|_| "Забагато точок у рейсі".to_owned())?;
                Ok(TripRoutePoint {
                    point_id: point_id(point.point_id.trim())?,
                    sequence,
                    kind: TripPointKind::Custom("manual".to_owned()),
                    title: title.to_owned(),
                    navigation: Some(TripNavigationTarget {
                        latitude_e7: coordinate_e7(point.latitude, 90.0, "широти")?,
                        longitude_e7: coordinate_e7(point.longitude, 180.0, "довготи")?,
                        address: optional_text(point.address),
                    }),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let display_name = optional_text(input.display_name);
        let create_path = folder_path.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut folder = TripFolder::create_manual(
                &create_path,
                ManualTripCreateDraft {
                    trip_id: parsed_trip_id,
                    route_points,
                    planning_basis: None,
                },
            )
            .map_err(|error| format!("Не вдалося створити папку рейсу: {error}"))?;
            if display_name.is_some() {
                folder
                    .set_session_metadata(ActiveTripSessionMetadata {
                        display_name,
                        ..ActiveTripSessionMetadata::default()
                    })
                    .map_err(|error| format!("Не вдалося зберегти назву рейсу: {error}"))?;
            }
            Ok(())
        })
        .await
        .map_err(|error| format!("Не вдалося виконати створення рейсу: {error}"))??;

        self.register_trip(RegisterTripInput {
            folder_path: folder_path.to_string_lossy().into_owned(),
            token,
        })
        .await
    }

    async fn register_trip(&self, input: RegisterTripInput) -> Result<ControlUiStatus, String> {
        let token = input.token.trim().to_owned();
        let folder_path = PathBuf::from(input.folder_path.trim());
        if folder_path.as_os_str().is_empty() {
            return Err("Потрібно вказати папку рейсу".to_owned());
        }
        self.ensure_session_available(&token, None).await?;

        let folder = tokio::task::spawn_blocking(move || TripFolder::open_detected(&folder_path))
            .await
            .map_err(|error| format!("Не вдалося відкрити задачу читання рейсу: {error}"))?
            .map_err(|error| format!("Не вдалося відкрити папку рейсу: {error}"))?;
        let persisted_metadata = folder.session_metadata().clone();
        let trip_id_text = folder
            .trip_id()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        self.ensure_session_available(&token, Some(&trip_id_text))
            .await?;

        self.browser
            .register_trip(token.clone(), folder)
            .map_err(|error| format!("Не вдалося створити browser session: {error:?}"))?;

        self.sessions
            .lock()
            .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?
            .insert(
                token.clone(),
                RegisteredSession {
                    trip_id: trip_id_text,
                    token,
                    operator_display_name: persisted_metadata.display_name,
                    endpoint_id: persisted_metadata.endpoint_id,
                    endpoint_label: persisted_metadata.endpoint_label,
                    vehicle_label: persisted_metadata.vehicle_label,
                    vehicle_type: persisted_metadata.vehicle_type,
                },
            );

        self.status().await
    }

    async fn start_loopback(&self) -> Result<ControlUiStatus, String> {
        let mut server = self.server.lock().await;
        if server.is_some() {
            return Err("Browser listener уже запущений".to_owned());
        }
        let started = LoopbackControlServer::start(self.browser.clone())
            .await
            .map_err(|error| format!("Не вдалося запустити loopback: {error}"))?;
        *server = Some(started);
        drop(server);
        self.status().await
    }

    async fn start_lan(&self, input: StartLanInput) -> Result<ControlUiStatus, String> {
        let ip: IpAddr = input
            .ip
            .trim()
            .parse()
            .map_err(|_| "Некоректна LAN-адреса".to_owned())?;
        let address = SocketAddr::new(ip, input.port);

        let mut server = self.server.lock().await;
        if server.is_some() {
            return Err("Browser listener уже запущений; спочатку вимкніть його".to_owned());
        }
        let started = LoopbackControlServer::start_lan(self.browser.clone(), address)
            .await
            .map_err(|error| format!("Не вдалося запустити LAN test mode: {error}"))?;
        *server = Some(started);
        drop(server);
        self.status().await
    }

    async fn stop(&self) -> Result<ControlUiStatus, String> {
        let current = self.server.lock().await.take();
        if let Some(server) = current {
            server
                .shutdown()
                .await
                .map_err(|error| format!("Не вдалося зупинити listener: {error}"))?;
        }
        self.status().await
    }

    async fn start_gateway(&self, input: StartGatewayInput) -> Result<ControlUiStatus, String> {
        let gateway_base_url = input.gateway_base_url.trim().to_owned();
        if gateway_base_url.is_empty() {
            return Err("Потрібно вказати HTTPS-адресу Gateway.".to_owned());
        }

        let session_tokens = self
            .sessions
            .lock()
            .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?
            .values()
            .map(|session| session.token.clone())
            .collect::<Vec<_>>();
        if session_tokens.is_empty() {
            return Err("Спочатку підключіть хоча б один рейс.".to_owned());
        }

        let mut gateway = self.gateway.lock().await;
        if gateway.is_some() {
            return Err("Публічна Gateway-трансляція вже запущена.".to_owned());
        }

        let internal_server = LoopbackControlServer::start(self.browser.clone())
            .await
            .map_err(|error| {
                format!("Не вдалося запустити внутрішній Control listener: {error}")
            })?;
        let local_control_base_url = format!("http://{}", internal_server.address());

        let mut prepared: Vec<(String, ControlGatewayClient, String)> = Vec::new();
        for token in session_tokens {
            let config = match GatewayRelayConfig::new(
                &gateway_base_url,
                &local_control_base_url,
                input.control_secret.clone(),
                token.clone(),
            ) {
                Ok(config) => config,
                Err(error) => {
                    for (_, client, _) in &prepared {
                        let _ = client.unregister().await;
                    }
                    let _ = internal_server.shutdown().await;
                    return Err(format!("Некоректні налаштування Gateway: {error}"));
                }
            };
            let client = match ControlGatewayClient::new(config) {
                Ok(client) => client,
                Err(error) => {
                    for (_, client, _) in &prepared {
                        let _ = client.unregister().await;
                    }
                    let _ = internal_server.shutdown().await;
                    return Err(format!("Не вдалося підготувати Gateway-клієнт: {error}"));
                }
            };
            let public_driver_url = client
                .config()
                .public_driver_url()
                .map_err(|error| format!("Не вдалося сформувати публічний Driver URL: {error}"))?
                .to_string();

            if let Err(error) = client.register().await {
                for (_, client, _) in &prepared {
                    let _ = client.unregister().await;
                }
                let _ = internal_server.shutdown().await;
                return Err(format!("Gateway не прийняв Control session: {error}"));
            }
            prepared.push((token, client, public_driver_url));
        }

        let base_url = prepared
            .first()
            .map(|(_, client, _)| client.config().gateway_base_url().to_string())
            .unwrap_or(gateway_base_url);

        let mut relays = BTreeMap::new();
        for (token, client, public_driver_url) in prepared {
            let (shutdown, shutdown_rx) = watch::channel(false);
            let runner = client.clone();
            let task = tokio::spawn(async move {
                let _ = runner.run_registered(shutdown_rx).await;
            });
            relays.insert(
                token,
                GatewayRelayTask {
                    client,
                    shutdown,
                    task,
                    public_driver_url,
                },
            );
        }

        *gateway = Some(GatewayRelayRuntime {
            base_url,
            internal_server,
            relays,
        });
        drop(gateway);
        self.status().await
    }

    async fn stop_gateway(&self) -> Result<ControlUiStatus, String> {
        let current = self.gateway.lock().await.take();
        if let Some(mut gateway) = current {
            for relay in gateway.relays.values_mut() {
                let _ = relay.shutdown.send(true);
                let _ = relay.client.unregister().await;
                relay.task.abort();
            }
            gateway.internal_server.shutdown().await.map_err(|error| {
                format!("Не вдалося зупинити внутрішній Gateway listener: {error}")
            })?;
        }
        self.status().await
    }

    async fn status(&self) -> Result<ControlUiStatus, String> {
        let (listener, address) = {
            let server = self.server.lock().await;
            match server.as_ref() {
                Some(server) => {
                    let info = server.info();
                    (
                        Some(ListenerStatus {
                            mode: match info.mode {
                                ControlAccessMode::Loopback => ListenerMode::Loopback,
                                ControlAccessMode::Lan => ListenerMode::Lan,
                            },
                            address: info.address.to_string(),
                        }),
                        Some(server.address()),
                    )
                }
                None => (None, None),
            }
        };

        let (gateway_status, public_driver_urls) = {
            let gateway = self.gateway.lock().await;
            match gateway.as_ref() {
                Some(gateway) => {
                    let active_sessions = gateway
                        .relays
                        .values()
                        .filter(|relay| !relay.task.is_finished())
                        .count();
                    let urls = gateway
                        .relays
                        .iter()
                        .map(|(token, relay)| (token.clone(), relay.public_driver_url.clone()))
                        .collect::<BTreeMap<_, _>>();
                    (
                        Some(GatewayStatus {
                            base_url: gateway.base_url.clone(),
                            active_sessions,
                            total_sessions: gateway.relays.len(),
                        }),
                        urls,
                    )
                }
                None => (None, BTreeMap::new()),
            }
        };

        let routing = self
            .browser
            .valhalla_config()
            .map_err(|error| format!("Не вдалося прочитати налаштування маршрутизації: {error}"))?
            .map(|config| RoutingStatus {
                base_url: config.base_url().to_string(),
                map_dataset_id: config.map_dataset_id().to_owned(),
                connect_timeout_ms: config.connect_timeout().as_millis() as u64,
                request_timeout_ms: config.request_timeout().as_millis() as u64,
            });

        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?
            .values()
            .map(|session| {
                let routing_profile = self
                    .browser
                    .session_active_routing_profile(&session.token)
                    .map_err(|error| {
                        format!(
                            "Не вдалося прочитати routing context сесії {}: {error}",
                            session_display_name(session)
                        )
                    })?;
                Ok(SessionStatus {
                    trip_id: session.trip_id.clone(),
                    token: session.token.clone(),
                    display_name: session_display_name(session),
                    endpoint_id: session.endpoint_id.map(|id| id_hex(id.as_bytes())),
                    endpoint_label: session.endpoint_label.clone(),
                    vehicle_label: session.vehicle_label.clone(),
                    vehicle_type: session.vehicle_type.clone(),
                    routing_profile: routing_profile.as_ref().map(routing_profile_view),
                    driver_url: address.map(|address| driver_url(address, &session.token)),
                    public_driver_url: public_driver_urls.get(&session.token).cloned(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(ControlUiStatus {
            listener,
            gateway: gateway_status,
            routing,
            sessions,
        })
    }

    async fn update_session_metadata(
        &self,
        input: UpdateSessionMetadataInput,
    ) -> Result<ControlUiStatus, String> {
        let endpoint_id = input
            .endpoint_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(driver_endpoint_id)
            .transpose()?;
        let operator_display_name = {
            let value = input.display_name.trim();
            (!value.is_empty()).then(|| value.to_owned())
        };
        let endpoint_label = optional_text(input.endpoint_label);
        let vehicle_label = optional_text(input.vehicle_label);
        let vehicle_type = optional_text(input.vehicle_type);
        let token = input.token.trim().to_owned();

        self.browser
            .set_session_metadata(
                &token,
                ActiveTripSessionMetadata {
                    display_name: operator_display_name.clone(),
                    endpoint_id,
                    endpoint_label: endpoint_label.clone(),
                    vehicle_label: vehicle_label.clone(),
                    vehicle_type: vehicle_type.clone(),
                },
            )
            .map_err(|error| format!("Не вдалося зберегти дані сесії: {error}"))?;

        {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?;
            let session = sessions
                .get_mut(&token)
                .ok_or_else(|| "Сесію не знайдено".to_owned())?;
            session.operator_display_name = operator_display_name;
            session.endpoint_id = endpoint_id;
            session.endpoint_label = endpoint_label;
            session.vehicle_label = vehicle_label;
            session.vehicle_type = vehicle_type;
        }
        self.status().await
    }

    fn session_window_details(&self, token: &str) -> Result<(String, String), String> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?;
        let session = sessions
            .get(token)
            .ok_or_else(|| "Сесію не знайдено".to_owned())?;
        Ok((session_display_name(session), session.token.clone()))
    }

    async fn configure_routing(
        &self,
        input: ConfigureRoutingInput,
    ) -> Result<ControlUiStatus, String> {
        let base_url = input.base_url.trim();
        let map_dataset_id = input.map_dataset_id.trim();
        if base_url.is_empty() {
            return Err("Потрібно вказати адресу Valhalla".to_owned());
        }
        if map_dataset_id.is_empty() {
            return Err("Потрібно вказати ідентифікатор набору карт".to_owned());
        }
        if !(100..=30_000).contains(&input.connect_timeout_ms) {
            return Err("Таймаут підключення має бути 100–30000 мс".to_owned());
        }
        if !(250..=120_000).contains(&input.request_timeout_ms)
            || input.request_timeout_ms < input.connect_timeout_ms
        {
            return Err(
                "Таймаут запиту має бути 250–120000 мс і не меншим за таймаут підключення"
                    .to_owned(),
            );
        }
        let config = ValhallaRuntimeConfig::new(
            base_url,
            map_dataset_id,
            Duration::from_millis(input.connect_timeout_ms),
            Duration::from_millis(input.request_timeout_ms),
        )
        .map_err(|error| format!("Некоректні налаштування Valhalla: {error}"))?;
        self.browser
            .configure_valhalla(config)
            .map_err(|error| format!("Не вдалося налаштувати Valhalla: {error}"))?;
        self.status().await
    }

    async fn disable_routing(&self) -> Result<ControlUiStatus, String> {
        self.browser
            .clear_valhalla()
            .map_err(|error| format!("Не вдалося вимкнути Valhalla: {error}"))?;
        self.status().await
    }

    async fn test_routing(&self) -> Result<RoutingProbeResult, String> {
        let availability = self
            .browser
            .valhalla_availability()
            .await
            .map_err(|error| format!("Не вдалося перевірити Valhalla: {error}"))?;
        let Some(availability) = availability else {
            return Ok(RoutingProbeResult {
                state: RoutingAvailabilityState::NotConfigured,
                engine_version: None,
                tileset_last_modified: None,
            });
        };
        let state = match availability.status {
            NormalizedStatus::Ok => RoutingAvailabilityState::Ok,
            NormalizedStatus::Timeout => RoutingAvailabilityState::Timeout,
            NormalizedStatus::EngineUnavailable => RoutingAvailabilityState::EngineUnavailable,
            NormalizedStatus::DatasetMissing => RoutingAvailabilityState::DatasetMissing,
            NormalizedStatus::EngineError
            | NormalizedStatus::NoRoute
            | NormalizedStatus::InvalidPoint
            | NormalizedStatus::InvalidProfile => RoutingAvailabilityState::EngineError,
        };
        Ok(RoutingProbeResult {
            state,
            engine_version: availability.engine_version,
            tileset_last_modified: availability.tileset_last_modified,
        })
    }

    fn dashboard_from_snapshot(
        &self,
        token: &str,
        snapshot: DriverTripSnapshot,
    ) -> Result<TripDashboard, String> {
        let planning_basis = self
            .browser
            .session_planning_basis(token)
            .map_err(|error| format!("Не вдалося прочитати планову основу рейсу: {error}"))?;
        let actual_context = self
            .browser
            .session_metadata(token)
            .map_err(|error| format!("Не вдалося прочитати фактичний контекст рейсу: {error}"))?;
        let active_routing_profile = self
            .browser
            .session_active_routing_profile(token)
            .map_err(|error| format!("Не вдалося прочитати routing context рейсу: {error}"))?;
        dashboard(
            snapshot,
            planning_basis,
            actual_context,
            active_routing_profile,
        )
    }

    fn dashboard(&self, token: &str) -> Result<TripDashboard, String> {
        let snapshot = self
            .browser
            .session_snapshot(token)
            .map_err(|error| format!("Не вдалося прочитати рейс: {error}"))?;
        self.dashboard_from_snapshot(token, snapshot)
    }

    async fn update_routing_context(
        &self,
        input: UpdateRoutingContextInput,
    ) -> Result<TripDashboard, String> {
        let token = input.token.trim().to_owned();
        let profile = input.profile.map(routing_profile_from_view).transpose()?;
        self.browser
            .set_session_active_routing_profile(&token, profile.clone())
            .map_err(|error| format!("Не вдалося зберегти routing context рейсу: {error}"))?;

        if profile.is_some() {
            let outcome = self
                .browser
                .recalculate_session_route_live(&token)
                .await
                .map_err(|error| format!("Не вдалося перерахувати маршрут: {error}"))?;
            self.dashboard_from_snapshot(&token, outcome.snapshot)
        } else {
            self.dashboard(&token)
        }
    }

    async fn reorder_route(&self, input: ReorderRouteInput) -> Result<TripDashboard, String> {
        let order = input
            .open_point_ids
            .iter()
            .map(|value| point_id(value))
            .collect::<Result<Vec<_>, _>>()?;
        let outcome = self
            .browser
            .reorder_session_route_live(&input.token, input.based_on_revision, &order)
            .await
            .map_err(|error| format!("Control не прийняв нову черговість: {error}"))?;
        self.dashboard_from_snapshot(&input.token, outcome.snapshot)
    }

    async fn add_operator_document(
        &self,
        input: OperatorDocumentInput,
    ) -> Result<TripDashboard, String> {
        let id = document_id(&input.document_id)?;
        let point = input.point_id.as_deref().map(point_id).transpose()?;
        let source = PathBuf::from(&input.source_path);
        if !source.is_file() {
            return Err("Вибраний файл більше недоступний".to_owned());
        }
        let mime_type = if input.mime_type.trim().is_empty() {
            mime_for_path(&source)
        } else {
            input.mime_type
        };
        let draft = DocumentStoreDraft {
            document_id: id,
            direction: DocumentDirection::FromOperator,
            original_name: input.original_name,
            mime_type,
            created_at_unix_ms: input.created_at_unix_ms,
            point_id: point,
        };
        let browser = self.browser.clone();
        let token = input.token;
        let dashboard_token = token.clone();
        let snapshot = tokio::task::spawn_blocking(move || {
            browser.store_operator_document(&token, draft, &source)
        })
        .await
        .map_err(|error| format!("Не вдалося виконати файлову операцію: {error}"))?
        .map_err(|error| format!("Control не прийняв файл: {error}"))?;
        self.dashboard_from_snapshot(&dashboard_token, snapshot)
    }

    fn verified_document(
        &self,
        input: &SaveDocumentInput,
    ) -> Result<(cargo_dispatch_trip_files::StoredDocument, PathBuf), String> {
        let id = document_id(&input.document_id)?;
        self.browser
            .verified_session_document_path(&input.token, id)
            .map_err(|error| format!("Не вдалося перевірити файл рейсу: {error}"))
    }
}

#[tauri::command]
async fn pick_trip_folder(app: AppHandle) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|value| {
                value
                    .into_path()
                    .map(|path| path.to_string_lossy().into_owned())
                    .map_err(|error| format!("Не вдалося прочитати вибрану папку: {error}"))
            })
            .transpose()
    })
    .await
    .map_err(|error| format!("Не вдалося відкрити вибір папки: {error}"))?
}

#[tauri::command]
async fn pick_operator_file(app: AppHandle) -> Result<Option<PickedFile>, String> {
    tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_file()
            .map(|value| {
                let path = value
                    .into_path()
                    .map_err(|error| format!("Не вдалося прочитати вибраний файл: {error}"))?;
                let original_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| "Назва файла має бути UTF-8".to_owned())?
                    .to_owned();
                Ok(PickedFile {
                    mime_type: mime_for_path(&path),
                    path: path.to_string_lossy().into_owned(),
                    original_name,
                })
            })
            .transpose()
    })
    .await
    .map_err(|error| format!("Не вдалося відкрити вибір файла: {error}"))?
}

#[tauri::command]
async fn save_session_document(
    input: SaveDocumentInput,
    app: AppHandle,
    state: State<'_, DesktopControlService>,
) -> Result<Option<String>, String> {
    let (document, source) = state.verified_document(&input)?;
    tokio::task::spawn_blocking(move || {
        let target = app
            .dialog()
            .file()
            .set_file_name(document.original_name.clone())
            .blocking_save_file();
        let Some(target) = target else {
            return Ok(None);
        };
        let target = target
            .into_path()
            .map_err(|error| format!("Не вдалося прочитати шлях збереження: {error}"))?;
        let copied = fs::copy(&source, &target)
            .map_err(|error| format!("Не вдалося зберегти файл: {error}"))?;
        if copied != document.size {
            let _ = fs::remove_file(&target);
            return Err("Збережений файл має неправильний розмір".to_owned());
        }
        Ok(Some(target.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|error| format!("Не вдалося виконати збереження файла: {error}"))?
}

#[tauri::command]
fn trip_dashboard(
    input: SessionInput,
    state: State<'_, DesktopControlService>,
) -> Result<TripDashboard, String> {
    state.dashboard(&input.token)
}

#[tauri::command]
async fn reorder_trip_route(
    input: ReorderRouteInput,
    state: State<'_, DesktopControlService>,
) -> Result<TripDashboard, String> {
    state.reorder_route(input).await
}

#[tauri::command]
async fn update_routing_context(
    input: UpdateRoutingContextInput,
    state: State<'_, DesktopControlService>,
) -> Result<TripDashboard, String> {
    state.update_routing_context(input).await
}

#[tauri::command]
async fn send_operator_document(
    input: OperatorDocumentInput,
    state: State<'_, DesktopControlService>,
) -> Result<TripDashboard, String> {
    state.add_operator_document(input).await
}

#[tauri::command]
async fn control_status(
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.status().await
}

#[tauri::command]
async fn register_trip(
    input: RegisterTripInput,
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    let (token, folder) = (input.token.clone(), input.folder_path.clone());
    let status = state.register_trip(input).await?;
    state.remember_session(&token, &folder)?;
    Ok(status)
}

#[tauri::command]
async fn create_manual_trip(
    input: CreateManualTripInput,
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    let (token, folder) = (input.token.clone(), input.folder_path.clone());
    let status = state.create_manual_trip(input).await?;
    state.remember_session(&token, &folder)?;
    Ok(status)
}

#[tauri::command]
async fn update_session_metadata(
    input: UpdateSessionMetadataInput,
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.update_session_metadata(input).await
}

#[tauri::command]
async fn open_session_window(
    input: SessionInput,
    app: AppHandle,
    state: State<'_, DesktopControlService>,
) -> Result<(), String> {
    let (display_name, token) = state.session_window_details(input.token.trim())?;
    let label = session_window_label(&token);
    let title = format!("Cargo Dispatch Control — {display_name}");

    if let Some(window) = app.get_webview_window(&label) {
        window
            .set_title(&title)
            .map_err(|error| format!("Не вдалося оновити назву вікна: {error}"))?;
        window
            .show()
            .map_err(|error| format!("Не вдалося показати вікно: {error}"))?;
        window
            .set_focus()
            .map_err(|error| format!("Не вдалося активувати вікно: {error}"))?;
        return Ok(());
    }

    WebviewWindowBuilder::new(
        &app,
        label,
        WebviewUrl::App(format!("index.html?session={token}").into()),
    )
    .title(title)
    .inner_size(1080.0, 760.0)
    .min_inner_size(760.0, 520.0)
    .resizable(true)
    .build()
    .map_err(|error| format!("Не вдалося відкрити вікно сесії: {error}"))?;
    Ok(())
}

#[tauri::command]
async fn start_loopback(
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.start_loopback().await
}

#[tauri::command]
async fn start_lan(
    input: StartLanInput,
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.start_lan(input).await
}

#[tauri::command]
fn lan_addresses() -> Vec<String> {
    local_lan_addresses()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[tauri::command]
async fn stop_browser_server(
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.stop().await
}

#[tauri::command]
async fn configure_routing(
    input: ConfigureRoutingInput,
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    let status = state.configure_routing(input.clone()).await?;
    state.remember_routing(Some(input))?;
    Ok(status)
}

#[tauri::command]
async fn test_routing(
    state: State<'_, DesktopControlService>,
) -> Result<RoutingProbeResult, String> {
    state.test_routing().await
}

#[tauri::command]
async fn disable_routing(
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    let status = state.disable_routing().await?;
    state.remember_routing(None)?;
    Ok(status)
}

#[tauri::command]
async fn start_gateway(
    input: StartGatewayInput,
    state: State<'_, DesktopControlService>,
) -> Result<ControlUiStatus, String> {
    state.start_gateway(input).await
}

#[tauri::command]
async fn stop_gateway(state: State<'_, DesktopControlService>) -> Result<ControlUiStatus, String> {
    state.stop_gateway().await
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("control-state-v1.json");
            let service = DesktopControlService::with_memory(path);
            tauri::async_runtime::block_on(service.restore());
            app.manage(service);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            control_status,
            register_trip,
            create_manual_trip,
            update_session_metadata,
            open_session_window,
            start_loopback,
            start_lan,
            lan_addresses,
            stop_browser_server,
            configure_routing,
            test_routing,
            disable_routing,
            start_gateway,
            stop_gateway,
            pick_trip_folder,
            pick_operator_file,
            save_session_document,
            trip_dashboard,
            reorder_trip_route,
            update_routing_context,
            send_operator_document,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Cargo Dispatch Control");
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_addressing_gateway::{GatewayState, router as gateway_router};
    use cargo_dispatch_contracts::{
        TripPackageHeader, TripPlanningBasis, TripPointKind, TripRoutePoint, TripVersion,
    };
    use cargo_dispatch_domain::{OperatorId, PointId, TripId, WorkingRouteId};
    use cargo_dispatch_planner::PlannerTripExportDraft;
    use cargo_dispatch_trip_files::export_package;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn connected_trips_come_back_with_the_same_link_after_a_restart() {
        let workspace = tempfile::tempdir().expect("workspace");
        let (_, folder) = create_trip(workspace.path()).expect("trip");
        let folder = folder.to_string_lossy().into_owned();
        let memory = workspace.path().join("control-state-v1.json");
        let token = "restart-session-token-0001";

        let first = DesktopControlService::with_memory(memory.clone());
        first
            .register_trip(RegisterTripInput {
                folder_path: folder.clone(),
                token: token.to_owned(),
            })
            .await
            .expect("registered");
        first.remember_session(token, &folder).expect("remembered");
        drop(first);

        let second = DesktopControlService::with_memory(memory);
        second.restore().await;
        assert!(
            second
                .sessions
                .lock()
                .expect("sessions")
                .contains_key(token)
        );
        assert!(second.server.lock().await.is_none(), "sharing stays off");
    }

    fn create_trip(root: &std::path::Path) -> std::io::Result<(TripId, PathBuf)> {
        let trip = TripId::from_bytes([21; 16]).expect("trip");
        let point = PointId::from_bytes([22; 16]).expect("point");
        let route = vec![TripRoutePoint {
            point_id: point,
            sequence: 1,
            kind: TripPointKind::Load,
            title: "Synthetic point".to_owned(),
            navigation: None,
        }];
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip, TripVersion::new(1).expect("version"), None)
                .expect("header"),
            working_route_id: WorkingRouteId::from_bytes([23; 16]).expect("route"),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([24; 16]).expect("operator"),
            ordered_point_ids: vec![point],
            planning_basis: Some(TripPlanningBasis {
                vehicle_type: "synthetic truck".to_owned(),
                cargos: Vec::new(),
            }),
            routing_profile: None,
            route_geometry: None,
        };
        let package = root.join("desktop-trip.zip");
        let folder = root.join("desktop-trip");
        export_package(&package, &draft, &route)?;
        TripFolder::import(&folder, &package)?;
        Ok((trip, folder))
    }

    #[tokio::test]
    async fn desktop_service_rejects_invalid_manual_token_before_disk_write() -> std::io::Result<()>
    {
        let tmp = tempfile::tempdir()?;
        let folder = tmp.path().join("manual-invalid-token");
        let service = DesktopControlService::default();

        let error = service
            .create_manual_trip(CreateManualTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                token: "short".to_owned(),
                trip_id: "41414141414141414141414141414141".to_owned(),
                display_name: None,
                points: vec![
                    cargo_dispatch_control_ui_contracts::CreateManualTripPointInput {
                        point_id: "42424242424242424242424242424242".to_owned(),
                        title: "Manual point".to_owned(),
                        latitude: 49.8393,
                        longitude: 24.0296,
                        address: None,
                    },
                ],
            })
            .await
            .expect_err("invalid token must be rejected");

        assert!(error.contains("Session token"));
        assert!(!folder.exists());
        Ok(())
    }

    #[tokio::test]
    async fn desktop_service_rejects_duplicate_manual_trip_before_disk_write() -> std::io::Result<()>
    {
        let tmp = tempfile::tempdir()?;
        let (trip, existing_folder) = create_trip(tmp.path())?;
        let service = DesktopControlService::default();
        service
            .register_trip(RegisterTripInput {
                folder_path: existing_folder.to_string_lossy().into_owned(),
                token: "existing-trip-token-0001".to_owned(),
            })
            .await
            .expect("existing trip registers");

        let new_folder = tmp.path().join("duplicate-manual-trip");
        let error = service
            .create_manual_trip(CreateManualTripInput {
                folder_path: new_folder.to_string_lossy().into_owned(),
                token: "new-manual-token-0001".to_owned(),
                trip_id: id_hex(trip.as_bytes()),
                display_name: None,
                points: vec![
                    cargo_dispatch_control_ui_contracts::CreateManualTripPointInput {
                        point_id: "43434343434343434343434343434343".to_owned(),
                        title: "Manual point".to_owned(),
                        latitude: 49.8393,
                        longitude: 24.0296,
                        address: None,
                    },
                ],
            })
            .await
            .expect_err("duplicate trip must be rejected");

        assert!(error.contains("уже зареєстрований"));
        assert!(!new_folder.exists());
        Ok(())
    }

    #[tokio::test]
    async fn desktop_service_creates_and_registers_manual_trip_without_planning_basis()
    -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let folder = tmp.path().join("manual-control-trip");
        let service = DesktopControlService::default();
        let token = "manual-trip-token-0001";

        let status = service
            .create_manual_trip(CreateManualTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                token: token.to_owned(),
                trip_id: "51515151515151515151515151515151".to_owned(),
                display_name: Some("Ручний рейс 51".to_owned()),
                points: vec![
                    cargo_dispatch_control_ui_contracts::CreateManualTripPointInput {
                        point_id: "52525252525252525252525252525252".to_owned(),
                        title: "Warehouse".to_owned(),
                        latitude: 49.8393,
                        longitude: 24.0296,
                        address: Some("Lviv".to_owned()),
                    },
                    cargo_dispatch_control_ui_contracts::CreateManualTripPointInput {
                        point_id: "53535353535353535353535353535353".to_owned(),
                        title: "Customer".to_owned(),
                        latitude: 49.8430,
                        longitude: 24.0360,
                        address: None,
                    },
                ],
            })
            .await
            .expect("manual trip created");

        assert_eq!(status.sessions.len(), 1);
        assert_eq!(status.sessions[0].display_name, "Ручний рейс 51");
        let dashboard = service.dashboard(token).expect("manual dashboard");
        assert_eq!(dashboard.trip_version, 1);
        assert_eq!(dashboard.points.len(), 2);
        assert_eq!(dashboard.points[0].title, "Warehouse");
        assert_eq!(dashboard.points[1].title, "Customer");
        assert!(dashboard.planning_basis.is_none());

        drop(service);
        let recovered = TripFolder::open_detected(&folder)?;
        assert_eq!(recovered.state().route_points.len(), 2);
        assert!(recovered.planning_basis().is_none());
        assert_eq!(
            recovered.session_metadata().display_name.as_deref(),
            Some("Ручний рейс 51")
        );
        Ok(())
    }

    #[tokio::test]
    async fn desktop_service_keeps_trip_registered_when_listener_stops() -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let (_trip, folder) = create_trip(tmp.path())?;
        let service = DesktopControlService::default();
        let token = "desktop-trip-token-0001";

        let registered = service
            .register_trip(RegisterTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                token: token.to_owned(),
            })
            .await
            .expect("trip registers");
        assert_eq!(registered.sessions.len(), 1);
        assert!(registered.listener.is_none());

        let running = service.start_loopback().await.expect("loopback starts");
        let listener = running.listener.expect("listener status");
        assert_eq!(listener.mode, ListenerMode::Loopback);
        assert!(listener.address.starts_with("127.0.0.1:"));
        assert!(
            running.sessions[0]
                .driver_url
                .as_deref()
                .is_some_and(|url| url.contains(token))
        );

        assert!(service.start_loopback().await.is_err());
        let stopped = service.stop().await.expect("listener stops");
        assert!(stopped.listener.is_none());
        assert_eq!(stopped.sessions.len(), 1);
        assert!(stopped.sessions[0].driver_url.is_none());

        let initial = service.dashboard(token).expect("dashboard");
        assert_eq!(initial.points.len(), 1);
        assert!(initial.documents.is_empty());

        let operator_file = tmp.path().join("operator-note.pdf");
        std::fs::write(&operator_file, b"synthetic operator document")?;
        let updated = service
            .add_operator_document(OperatorDocumentInput {
                token: token.to_owned(),
                document_id: "19191919191919191919191919191919".to_owned(),
                source_path: operator_file.to_string_lossy().into_owned(),
                original_name: "operator-note.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                point_id: Some(initial.points[0].point_id.clone()),
                created_at_unix_ms: 2,
            })
            .await
            .expect("operator document");
        assert_eq!(updated.documents.len(), 1);
        assert_eq!(
            updated.documents[0].direction,
            DashboardDocumentDirection::FromOperator
        );
        let save_input = SaveDocumentInput {
            token: token.to_owned(),
            document_id: updated.documents[0].document_id.clone(),
        };
        let (verified, source) = service
            .verified_document(&save_input)
            .expect("stored document verifies before native save dialog");
        assert_eq!(verified.original_name, "operator-note.pdf");
        assert_eq!(std::fs::read(source)?, b"synthetic operator document");

        Ok(())
    }

    #[tokio::test]
    async fn desktop_session_metadata_keeps_endpoint_and_current_vehicle_separate()
    -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let (_trip, folder) = create_trip(tmp.path())?;
        let service = DesktopControlService::default();
        let token = "desktop-session-metadata-0001";

        let registered = service
            .register_trip(RegisterTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                token: token.to_owned(),
            })
            .await
            .expect("trip registers");
        assert!(registered.sessions[0].display_name.starts_with("trip-"));
        assert!(registered.sessions[0].endpoint_id.is_none());
        assert!(registered.sessions[0].vehicle_label.is_none());

        let updated = service
            .update_session_metadata(UpdateSessionMetadataInput {
                token: token.to_owned(),
                display_name: String::new(),
                endpoint_id: Some("31313131313131313131313131313131".to_owned()),
                endpoint_label: Some("Cab tablet".to_owned()),
                vehicle_label: Some("BC 1234 AB".to_owned()),
                vehicle_type: Some("tractor + semi-trailer".to_owned()),
            })
            .await
            .expect("metadata updates");
        let session = &updated.sessions[0];
        assert_eq!(session.display_name, "BC 1234 AB");
        assert_eq!(session.endpoint_label.as_deref(), Some("Cab tablet"));
        assert_eq!(session.vehicle_label.as_deref(), Some("BC 1234 AB"));
        assert_eq!(
            session.vehicle_type.as_deref(),
            Some("tractor + semi-trailer")
        );

        let dashboard = service.dashboard(token).expect("dashboard");
        let planned = dashboard.planning_basis.expect("planning basis");
        assert_eq!(planned.vehicle_type, "synthetic truck");
        assert!(planned.cargos.is_empty());
        assert_eq!(
            dashboard.actual_context.endpoint_label.as_deref(),
            Some("Cab tablet")
        );
        assert_eq!(
            dashboard.actual_context.vehicle_label.as_deref(),
            Some("BC 1234 AB")
        );
        assert_eq!(
            dashboard.actual_context.vehicle_type.as_deref(),
            Some("tractor + semi-trailer")
        );

        let renamed = service
            .update_session_metadata(UpdateSessionMetadataInput {
                token: token.to_owned(),
                display_name: "Швеція 01".to_owned(),
                endpoint_id: session.endpoint_id.clone(),
                endpoint_label: session.endpoint_label.clone(),
                vehicle_label: Some("BC 9999 AA".to_owned()),
                vehicle_type: session.vehicle_type.clone(),
            })
            .await
            .expect("operator display name updates");
        assert_eq!(renamed.sessions[0].display_name, "Швеція 01");
        assert_eq!(
            renamed.sessions[0].vehicle_label.as_deref(),
            Some("BC 9999 AA")
        );

        drop(service);
        let recovered_service = DesktopControlService::default();
        let recovered = recovered_service
            .register_trip(RegisterTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                token: "desktop-session-metadata-0002".to_owned(),
            })
            .await
            .expect("trip re-registers after restart");
        assert_eq!(recovered.sessions[0].display_name, "Швеція 01");
        assert_eq!(
            recovered.sessions[0].endpoint_label.as_deref(),
            Some("Cab tablet")
        );
        assert_eq!(
            recovered.sessions[0].vehicle_label.as_deref(),
            Some("BC 9999 AA")
        );

        Ok(())
    }

    #[tokio::test]
    async fn desktop_service_persists_current_routing_context_without_valhalla()
    -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let (_trip, folder) = create_trip(tmp.path())?;
        let service = DesktopControlService::default();
        let token = "desktop-routing-context-0001";

        service
            .register_trip(RegisterTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                token: token.to_owned(),
            })
            .await
            .expect("trip registers");

        let profile = DashboardVehicleRoutingProfile {
            height_m: Some(4.0),
            width_m: Some(2.55),
            length_m: Some(16.5),
            gross_weight_t: Some(40.0),
            axle_load_t: Some(11.5),
            axle_count: Some(5),
            hazmat: false,
            hgv: true,
            exclude_unpaved: true,
            prefer_truck_routes: true,
            toll_preference: RoutingTollPreference::Allow,
            max_speed_kmh: Some(90.0),
        };

        let dashboard = service
            .update_routing_context(UpdateRoutingContextInput {
                token: token.to_owned(),
                profile: Some(profile.clone()),
            })
            .await
            .expect("routing context updates");
        assert_eq!(
            dashboard.actual_context.routing_profile.as_ref(),
            Some(&profile)
        );
        assert!(dashboard.road_geometry.is_none());

        let mut unsupported = profile.clone();
        unsupported.max_speed_kmh = Some(300.0);
        let error = service
            .update_routing_context(UpdateRoutingContextInput {
                token: token.to_owned(),
                profile: Some(unsupported),
            })
            .await
            .expect_err("unsupported speed is refused");
        assert!(error.contains("max_speed_kmh"), "{error}");

        drop(service);
        let recovered = DesktopControlService::default();
        let status = recovered
            .register_trip(RegisterTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                token: "desktop-routing-context-0002".to_owned(),
            })
            .await
            .expect("trip re-registers");
        assert_eq!(status.sessions.len(), 1);
        let dashboard = recovered
            .dashboard("desktop-routing-context-0002")
            .expect("dashboard recovers");
        assert_eq!(dashboard.actual_context.routing_profile, Some(profile));

        Ok(())
    }

    #[test]
    fn control_ui_speed_range_matches_geo_contract() {
        assert_eq!(
            cargo_dispatch_control_ui_contracts::MIN_ROUTING_MAX_SPEED_KMH,
            cargo_dispatch_geo::MIN_MAX_SPEED_KMH
        );
        assert_eq!(
            cargo_dispatch_control_ui_contracts::MAX_ROUTING_MAX_SPEED_KMH,
            cargo_dispatch_geo::MAX_MAX_SPEED_KMH
        );
    }

    #[tokio::test]
    async fn desktop_service_relays_real_driver_page_through_gateway() -> std::io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let (_trip, folder) = create_trip(tmp.path())?;
        let service = DesktopControlService::default();
        let token = "desktop-gateway-session-0001";
        let secret = "desktop-gateway-control-secret-0001";

        service
            .register_trip(RegisterTripInput {
                folder_path: folder.to_string_lossy().into_owned(),
                token: token.to_owned(),
            })
            .await
            .expect("trip registers");

        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let gateway_address = listener.local_addr()?;
        let gateway = tokio::spawn(async move {
            axum::serve(
                listener,
                gateway_router(GatewayState::new(secret, 1024 * 1024)),
            )
            .await
            .expect("gateway server");
        });

        let running = service
            .start_gateway(StartGatewayInput {
                gateway_base_url: format!("http://{gateway_address}"),
                control_secret: secret.to_owned(),
            })
            .await
            .expect("gateway starts");

        let gateway_status = running.gateway.expect("gateway status");
        assert_eq!(gateway_status.total_sessions, 1);
        assert_eq!(gateway_status.active_sessions, 1);
        let public_url = running.sessions[0]
            .public_driver_url
            .clone()
            .expect("public Driver URL");
        assert!(public_url.contains(token));

        let response = reqwest::get(public_url).await.expect("public Driver page");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let page = response.text().await.expect("Driver page body");
        assert!(page.contains("Cargo Dispatch"));
        assert!(page.contains("Документи / фото / відео"));

        let stopped = service.stop_gateway().await.expect("gateway stops");
        assert!(stopped.gateway.is_none());
        assert!(stopped.sessions[0].public_driver_url.is_none());

        gateway.abort();
        Ok(())
    }

    #[tokio::test]
    async fn desktop_service_configures_tests_and_disables_valhalla() -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let app = axum::Router::new().route(
            "/status",
            axum::routing::get(|| async {
                r#"{"version":"3.9.2","tileset_last_modified":1795550000}"#
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("mock Valhalla");
        });

        let service = DesktopControlService::default();
        let configured = service
            .configure_routing(ConfigureRoutingInput {
                base_url: format!("http://{address}"),
                map_dataset_id: "fixture-europe-2026-11".to_owned(),
                connect_timeout_ms: 500,
                request_timeout_ms: 2_000,
            })
            .await
            .expect("routing config");
        let routing = configured.routing.expect("routing status");
        assert_eq!(routing.map_dataset_id, "fixture-europe-2026-11");
        assert_eq!(routing.connect_timeout_ms, 500);
        assert_eq!(routing.request_timeout_ms, 2_000);

        let probe = service.test_routing().await.expect("routing probe");
        assert_eq!(probe.state, RoutingAvailabilityState::Ok);
        assert_eq!(probe.engine_version.as_deref(), Some("3.9.2"));
        assert_eq!(probe.tileset_last_modified, Some(1_795_550_000));

        let disabled = service.disable_routing().await.expect("disable routing");
        assert!(disabled.routing.is_none());
        let probe = service.test_routing().await.expect("disabled probe");
        assert_eq!(probe.state, RoutingAvailabilityState::NotConfigured);

        server.abort();
        Ok(())
    }

    #[tokio::test]
    async fn failed_routing_probe_keeps_runtime_configuration() -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let address = listener.local_addr()?;
        let app = axum::Router::new().route(
            "/status",
            axum::routing::get(|| async {
                (
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    r#"{"status":"down"}"#,
                )
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("mock unavailable Valhalla");
        });

        let service = DesktopControlService::default();
        service
            .configure_routing(ConfigureRoutingInput {
                base_url: format!("http://{address}"),
                map_dataset_id: "fixture-europe-2026-11".to_owned(),
                connect_timeout_ms: 500,
                request_timeout_ms: 2_000,
            })
            .await
            .expect("routing config");

        let probe = service.test_routing().await.expect("routing probe");
        assert_eq!(probe.state, RoutingAvailabilityState::EngineUnavailable);
        let status = service.status().await.expect("status");
        assert!(
            status.routing.is_some(),
            "failed probe must not erase config"
        );

        server.abort();
        Ok(())
    }

    #[tokio::test]
    async fn desktop_service_rejects_unbounded_routing_timeouts() {
        let service = DesktopControlService::default();
        assert!(
            service
                .configure_routing(ConfigureRoutingInput {
                    base_url: "http://127.0.0.1:8002".to_owned(),
                    map_dataset_id: "fixture".to_owned(),
                    connect_timeout_ms: 99,
                    request_timeout_ms: 2_000,
                })
                .await
                .is_err()
        );
        assert!(
            service
                .configure_routing(ConfigureRoutingInput {
                    base_url: "http://127.0.0.1:8002".to_owned(),
                    map_dataset_id: "fixture".to_owned(),
                    connect_timeout_ms: 2_000,
                    request_timeout_ms: 1_000,
                })
                .await
                .is_err()
        );
        assert!(service.status().await.expect("status").routing.is_none());
    }

    #[tokio::test]
    async fn desktop_service_does_not_bypass_lan_validation() {
        let service = DesktopControlService::default();
        assert!(
            service
                .start_lan(StartLanInput {
                    ip: "0.0.0.0".to_owned(),
                    port: 8080,
                })
                .await
                .is_err()
        );
        assert!(
            service
                .start_lan(StartLanInput {
                    ip: "127.0.0.1".to_owned(),
                    port: 8080,
                })
                .await
                .is_err()
        );
        assert!(
            service
                .start_lan(StartLanInput {
                    ip: "8.8.8.8".to_owned(),
                    port: 8080,
                })
                .await
                .is_err()
        );
    }
}
