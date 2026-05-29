FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

ENV TERM=xterm-256color

EXPOSE 1883/tcp
EXPOSE 9001/tcp

COPY --from=builder /app/target/release/rust_mqtt_client_tui /usr/local/bin/rust-mqtt-client-tui

ENTRYPOINT ["rust-mqtt-client-tui"]
