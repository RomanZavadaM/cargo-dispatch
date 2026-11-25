#![forbid(unsafe_code)]

use cargo_dispatch_addressing_gateway::{GatewayState, router};
use std::{env, io, net::SocketAddr};
use tokio::net::TcpListener;

const DEFAULT_BIND: &str = "0.0.0.0:8080";
const DEFAULT_MAX_RELAY_BYTES: usize = 64 * 1024 * 1024;
const MAX_CONFIGURED_RELAY_BYTES: usize = 512 * 1024 * 1024;

#[tokio::main]
async fn main() -> io::Result<()> {
    let bind = env::var("CARGO_DISPATCH_GATEWAY_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let address: SocketAddr = bind.parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "CARGO_DISPATCH_GATEWAY_BIND must be an IP:port address",
        )
    })?;

    let control_token = env::var("CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN").map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN is required",
        )
    })?;
    if control_token.len() < 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN must contain at least 32 characters",
        ));
    }

    let max_relay_bytes = env::var("CARGO_DISPATCH_GATEWAY_MAX_RELAY_BYTES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_RELAY_BYTES);
    if max_relay_bytes == 0 || max_relay_bytes > MAX_CONFIGURED_RELAY_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "CARGO_DISPATCH_GATEWAY_MAX_RELAY_BYTES must be between 1 and 536870912",
        ));
    }

    let listener = TcpListener::bind(address).await?;
    let actual = listener.local_addr()?;
    println!("Cargo Dispatch Addressing Gateway listening on {actual}");

    axum::serve(
        listener,
        router(GatewayState::new(control_token, max_relay_bytes)),
    )
    .await
}
