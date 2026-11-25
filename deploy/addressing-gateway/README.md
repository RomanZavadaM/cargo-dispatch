# Addressing Gateway deployment

The product gateway listens on plain HTTP inside the host/container and is intended to sit behind a public HTTPS reverse proxy.

## Docker

```bash
export CARGO_DISPATCH_GATEWAY_CONTROL_TOKEN='replace-with-a-long-random-secret'
docker compose -f deploy/addressing-gateway/docker-compose.example.yml up -d --build
```

The example binds the container only to host loopback at `127.0.0.1:8080`. Publish HTTPS with Caddy/nginx/Traefik on the same server.

## Reverse proxy requirements

- HTTPS only on the public side.
- Preserve method, path, query string, request body, and response body.
- Do not cache `/driver/*`, `/api/trip/*`, or `/v1/*`.
- Allow long-poll requests to `/v1/control/*/next` for at least 30 seconds.
- Keep the Control registration token only in server/Control configuration; never expose it in Driver URLs.

## Persistence

None. The container can be replaced or restarted safely. Active Control sessions reconnect and register again. The gateway must not mount a trip/document/media volume.
