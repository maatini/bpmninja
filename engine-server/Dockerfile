# syntax=docker/dockerfile:1
FROM rust:1.99 AS builder
WORKDIR /app
COPY . .
# Registry/git/target bleiben im BuildKit-Cache (GHA mode=max).
# Das Binary liegt im Cache-Mount, deshalb nach /tmp kopieren.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p engine-server \
    && cp /app/target/release/engine-server /tmp/engine-server

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /tmp/engine-server /usr/local/bin/engine-server
ENV PORT=8081
ENV NATS_URL=nats://nats:4222
EXPOSE 8081
CMD ["engine-server"]
