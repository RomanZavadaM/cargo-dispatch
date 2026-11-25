#![forbid(unsafe_code)]

use cargo_dispatch_control_web::{ControlBrowserState, LoopbackControlServer};
use cargo_dispatch_trip_files::TripFolder;
use std::{
    env, future,
    io::{self, Write},
    path::PathBuf,
};

const DEFAULT_TOKEN: &str = "native-smoke-session-00000001";

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

    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: native_smoke_server <trip-package.zip> [session-token]",
        ));
    }

    let workspace = tempfile::tempdir()?;
    let active_trip = workspace.path().join("active-trip");
    let trip = TripFolder::import(&active_trip, &package)?;

    let state = ControlBrowserState::new();
    state
        .register_trip(token.clone(), trip)
        .map_err(|error| io::Error::other(format!("register smoke trip: {error:?}")))?;

    let server = LoopbackControlServer::start(state).await?;
    let address = server.address();
    let local_url = format!("http://{address}/driver/{token}");
    let android_url = format!("http://10.0.2.2:{}/driver/{token}", address.port());

    println!("CARGO_DISPATCH_SMOKE_LOCAL_URL={local_url}");
    println!("CARGO_DISPATCH_SMOKE_ANDROID_URL={android_url}");
    println!("CARGO_DISPATCH_SMOKE_ACTIVE_TRIP={}", active_trip.display());
    io::stdout().flush()?;

    // The CI harness owns the process lifetime and terminates this server
    // after the virtual-device assertions and screenshot are complete.
    future::pending::<()>().await;
    Ok(())
}
