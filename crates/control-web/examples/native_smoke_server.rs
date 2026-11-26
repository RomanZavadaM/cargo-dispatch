#![forbid(unsafe_code)]

use cargo_dispatch_control_gateway_client::{ControlGatewayClient, GatewayRelayConfig};
use cargo_dispatch_control_web::{ControlBrowserState, LoopbackControlServer};
use cargo_dispatch_domain::DocumentId;
use cargo_dispatch_trip_files::{DocumentDirection, DocumentStoreDraft, TripFolder};
use std::{
    env, future,
    io::{self, Write},
    path::PathBuf,
};
use tokio::sync::watch;

const DEFAULT_TOKEN: &str = "native-smoke-session-00000001";
const GATEWAY_BASE_URL_ENV: &str = "CARGO_DISPATCH_SMOKE_GATEWAY_BASE_URL";
const GATEWAY_CONTROL_SECRET_ENV: &str = "CARGO_DISPATCH_SMOKE_GATEWAY_CONTROL_SECRET";

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut args = env::args_os().skip(1);
    let package = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "trip package path required"))?;
    let token = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| DEFAULT_TOKEN.to_owned());

    let operator_document = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: native_smoke_server <trip-package.zip> [session-token] [operator-document]",
        ));
    }

    let workspace = tempfile::tempdir()?;
    let active_trip = workspace.path().join("active-trip");
    let trip = TripFolder::import(&active_trip, &package)?;

    let state = ControlBrowserState::new();
    state
        .register_trip(token.clone(), trip)
        .map_err(|error| io::Error::other(format!("register smoke trip: {error:?}")))?;

    if let Some(source) = operator_document {
        let original_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("operator-document.bin")
            .to_owned();
        state.store_operator_document(
            &token,
            DocumentStoreDraft {
                document_id: DocumentId::from_bytes([0x91; 16])
                    .map_err(|error| io::Error::other(format!("document id: {error:?}")))?,
                direction: DocumentDirection::FromOperator,
                original_name,
                mime_type: "application/pdf".to_owned(),
                created_at_unix_ms: 1,
                point_id: None,
            },
            &source,
        )?;
    }

    let lan_bind = env::var("CARGO_DISPATCH_SMOKE_LAN_BIND")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let lan_mode = lan_bind.is_some();
    let server = if let Some(value) = lan_bind {
        let address = value.parse().map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid CARGO_DISPATCH_SMOKE_LAN_BIND: {error}"),
            )
        })?;
        LoopbackControlServer::start_lan(state, address).await?
    } else {
        LoopbackControlServer::start(state).await?
    };
    let address = server.address();
    let local_base_url = format!("http://{address}");
    let local_url = format!("{local_base_url}/driver/{token}");
    let android_url = format!("http://10.0.2.2:{}/driver/{token}", address.port());

    let gateway_base_url = env::var(GATEWAY_BASE_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let _gateway_relay = if let Some(gateway_base_url) = gateway_base_url {
        let control_secret = env::var(GATEWAY_CONTROL_SECRET_ENV).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{GATEWAY_CONTROL_SECRET_ENV} is required with {GATEWAY_BASE_URL_ENV}"),
            )
        })?;
        let config = GatewayRelayConfig::new(
            &gateway_base_url,
            &local_base_url,
            control_secret,
            token.clone(),
        )
        .map_err(|error| io::Error::other(format!("gateway relay config: {error}")))?;
        let public_url = config
            .public_driver_url()
            .map_err(|error| io::Error::other(format!("public Driver URL: {error}")))?;
        let relay = ControlGatewayClient::new(config)
            .map_err(|error| io::Error::other(format!("gateway relay client: {error}")))?;
        let (shutdown, shutdown_rx) = watch::channel(false);
        let relay_task = tokio::spawn(async move {
            if let Err(error) = relay.run(shutdown_rx).await {
                eprintln!("CARGO_DISPATCH_SMOKE_GATEWAY_ERROR={error}");
            }
        });
        println!("CARGO_DISPATCH_SMOKE_PUBLIC_URL={public_url}");
        Some((shutdown, relay_task))
    } else {
        None
    };

    println!("CARGO_DISPATCH_SMOKE_LOCAL_URL={local_url}");
    if lan_mode {
        println!("CARGO_DISPATCH_SMOKE_LAN_URL={local_url}");
    }
    println!("CARGO_DISPATCH_SMOKE_ANDROID_URL={android_url}");
    println!("CARGO_DISPATCH_SMOKE_ACTIVE_TRIP={}", active_trip.display());
    io::stdout().flush()?;

    // The CI harness owns the process lifetime and terminates this server
    // after the virtual-device or public-Gateway assertions are complete.
    future::pending::<()>().await;
    Ok(())
}
