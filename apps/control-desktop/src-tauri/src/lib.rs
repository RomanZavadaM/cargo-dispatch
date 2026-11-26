#![forbid(unsafe_code)]

use cargo_dispatch_control_gateway_client::{ControlGatewayClient, GatewayRelayConfig};
use cargo_dispatch_control_ui_contracts::{
    ConfigureRoutingInput, ControlUiStatus, DashboardDocument, DashboardDocumentDirection,
    DashboardLocation, DashboardNavigation, DashboardPoint, DashboardRoadGeometry,
    DashboardRoadGeometryPoint, DashboardRoutingIdentity, GatewayStatus, ListenerMode,
    ListenerStatus, OperatorDocumentInput, PickedFile, RegisterTripInput, ReorderRouteInput,
    RoutingAvailabilityState, RoutingProbeResult, RoutingStatus, SaveDocumentInput, SessionInput,
    SessionStatus, StartGatewayInput, StartLanInput, TripDashboard,
};
use cargo_dispatch_control_web::{
    ControlAccessMode, ControlBrowserState, DriverTripSnapshot, LoopbackControlServer,
};
use cargo_dispatch_domain::{DocumentId, PointId};
use cargo_dispatch_geo::{NormalizedStatus, valhalla_http::ValhallaRuntimeConfig};
use cargo_dispatch_trip_files::{DocumentDirection, DocumentStoreDraft, TripFolder};
use std::{
    collections::BTreeMap,
    fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tokio::{
    sync::{Mutex as AsyncMutex, watch},
    task::JoinHandle,
};

#[derive(Debug, Clone)]
struct RegisteredSession {
    trip_id: String,
    token: String,
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
}

impl Default for DesktopControlService {
    fn default() -> Self {
        Self {
            browser: ControlBrowserState::new(),
            server: AsyncMutex::new(None),
            gateway: AsyncMutex::new(None),
            sessions: Mutex::new(BTreeMap::new()),
        }
    }
}

fn driver_url(address: SocketAddr, token: &str) -> String {
    format!("http://{address}/driver/{token}")
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

fn dashboard(snapshot: DriverTripSnapshot) -> Result<TripDashboard, String> {
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
    async fn register_trip(&self, input: RegisterTripInput) -> Result<ControlUiStatus, String> {
        if self.gateway.lock().await.is_some() {
            return Err(
                "Спочатку вимкніть публічну Gateway-трансляцію, потім додайте новий рейс."
                    .to_owned(),
            );
        }

        let token = input.token.trim().to_owned();
        let folder_path = PathBuf::from(input.folder_path.trim());
        if folder_path.as_os_str().is_empty() {
            return Err("Потрібно вказати папку рейсу".to_owned());
        }

        {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?;
            if sessions.contains_key(&token) {
                return Err("Цей token уже зареєстрований".to_owned());
            }
        }

        let folder = tokio::task::spawn_blocking(move || TripFolder::open_detected(&folder_path))
            .await
            .map_err(|error| format!("Не вдалося відкрити задачу читання рейсу: {error}"))?
            .map_err(|error| format!("Не вдалося відкрити папку рейсу: {error}"))?;
        let trip_id_text = folder
            .trip_id()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "Список сесій тимчасово недоступний".to_owned())?;
            if sessions
                .values()
                .any(|session| session.trip_id == trip_id_text)
            {
                return Err("Цей рейс уже зареєстрований у Control".to_owned());
            }
        }

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
            .map(|session| SessionStatus {
                trip_id: session.trip_id.clone(),
                token: session.token.clone(),
                driver_url: address.map(|address| driver_url(address, &session.token)),
                public_driver_url: public_driver_urls.get(&session.token).cloned(),
            })
            .collect();

        Ok(ControlUiStatus {
            listener,
            gateway: gateway_status,
            routing,
            sessions,
        })
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

    fn dashboard(&self, token: &str) -> Result<TripDashboard, String> {
        let snapshot = self
            .browser
            .session_snapshot(token)
            .map_err(|error| format!("Не вдалося прочитати рейс: {error}"))?;
        dashboard(snapshot)
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
        dashboard(outcome.snapshot)
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
        let snapshot = tokio::task::spawn_blocking(move || {
            browser.store_operator_document(&token, draft, &source)
        })
        .await
        .map_err(|error| format!("Не вдалося виконати файлову операцію: {error}"))?
        .map_err(|error| format!("Control не прийняв файл: {error}"))?;
        dashboard(snapshot)
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
    state.register_trip(input).await
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
    state.configure_routing(input).await
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
    state.disable_routing().await
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
        .manage(DesktopControlService::default())
        .invoke_handler(tauri::generate_handler![
            control_status,
            register_trip,
            start_loopback,
            start_lan,
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
            send_operator_document,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Cargo Dispatch Control");
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_addressing_gateway::{GatewayState, router as gateway_router};
    use cargo_dispatch_contracts::{TripPackageHeader, TripPointKind, TripRoutePoint, TripVersion};
    use cargo_dispatch_domain::{OperatorId, PointId, TripId, WorkingRouteId};
    use cargo_dispatch_planner::PlannerTripExportDraft;
    use cargo_dispatch_trip_files::export_package;
    use tokio::net::TcpListener;

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
