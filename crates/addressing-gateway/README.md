# Cargo Dispatch Addressing Gateway

Public server-side relay for connecting a specific **Control** instance with Driver Browser / Android / iOS clients.

## Product boundary

The gateway is deliberately thin:

- keeps only an in-memory session → Control relay map;
- accepts an outbound poll channel from Control, so Control does not need a public inbound port;
- relays the existing Control HTTP paths unchanged: `/driver/{token}` and `/api/trip/{token}/...`;
- does not host the Driver Browser bundle itself — HTML/JS responses still come from Control;
- does not persist trip state, documents, media, locations, or history;
- forgets all sessions on restart.

This r1 transport buffers one request/response in RAM and defaults to 64 MiB. It is suitable for page/API/photo/document testing. Streaming/chunked relay for large video is the next transport step.

## Server environment

- `CARGO_DISPATCH_GATEWAY_BIND` — default `0.0.0.0:8080`.
- `CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN` — required server secret, at least 32 characters. It is used only by Control, never placed in a Driver URL.
- `CARGO_DISPATCH_GATEWAY_MAX_RELAY_BYTES` — optional per-direction RAM limit, default 67108864, hard maximum 536870912.

TLS is expected to terminate at a normal reverse proxy (Caddy/nginx/Traefik). The public Driver URL should therefore be HTTPS.

## Control relay protocol

Control authenticates with:

`Authorization: Bearer <CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN>`

For each active Driver session token:

1. `PUT /v1/control/{session}` — register/replace the in-memory session.
2. Long-poll `GET /v1/control/{session}/next`.
3. A pending public HTTP request arrives as raw body plus:
   - `x-cargo-relay-request-id`
   - `x-cargo-relay-method`
   - `x-cargo-relay-path`
   - selected original headers under `x-cargo-relay-header-...`
4. Control executes that request against its own Control Browser state.
5. Control sends raw response body to `POST /v1/control/{session}/response/{request_id}` with:
   - `x-cargo-relay-status`
   - optional response headers under `x-cargo-relay-response-...`
6. `DELETE /v1/control/{session}` removes the mapping.

Public clients keep using the normal paths on the gateway host. They never receive the Control registration secret.

## Health

- `GET /healthz`
- `GET /v1/session/{session}` — reports whether that Control session is currently polling.

## Deployment

See `deploy/addressing-gateway/`.

The container listens on plain HTTP internally. Put it behind HTTPS and do not publish the Control registration token in client configuration.
