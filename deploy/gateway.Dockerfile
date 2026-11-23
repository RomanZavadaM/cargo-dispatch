FROM rust:1.89-bookworm AS build

WORKDIR /src
COPY Cargo.toml ./
COPY crates ./crates
COPY web ./web
RUN cargo build --release -p cargo-connectivity-gateway

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home --uid 10001 cargo-gateway

COPY --from=build /src/target/release/cargo-connectivity-gateway /usr/local/bin/cargo-connectivity-gateway

USER cargo-gateway
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/cargo-connectivity-gateway"]
