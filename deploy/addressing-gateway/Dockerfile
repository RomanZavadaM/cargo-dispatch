FROM rust:1.91-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --locked -p cargo-dispatch-addressing-gateway --release

FROM debian:bookworm-slim
RUN useradd --create-home --uid 10001 cargo-gateway
COPY --from=builder /src/target/release/cargo-dispatch-addressing-gateway /usr/local/bin/cargo-dispatch-addressing-gateway
USER cargo-gateway
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/cargo-dispatch-addressing-gateway"]
