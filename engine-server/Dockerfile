# syntax=docker/dockerfile:1

# Base-Images auf konkrete Minor-Tags gepinnt (im Repo sind keine Digests üblich).
# Builder und Runtime bleiben auf derselben Debian-Basis (bookworm), damit die
# glibc des Release-Builds zum Runtime-Image passt.
FROM rust:1.99-bookworm AS builder
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

# curl wird ausschließlich vom HEALTHCHECK gebraucht: debian-slim bringt weder
# curl noch wget mit, und curl prüft den Statuscode ohne Body (-f) und ohne
# Shell-Tricks. --no-install-recommends hält das Image schlank; die
# apt-Listen werden im selben Layer entfernt.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

# Non-Root-User mit fester UID/GID 10001, ohne Home und ohne Login-Shell.
# Bewusst ohne `--system`: System-UIDs enden bei 999, sonst warnt useradd.
RUN groupadd --gid 10001 bpmninja \
    && useradd --uid 10001 --gid bpmninja --no-create-home \
        --shell /usr/sbin/nologin bpmninja

WORKDIR /app
# /app muss für den Non-Root-User schreibbar bleiben: LOG_FILE (Default
# engine_logs.jsonl) liegt relativ zum Arbeitsverzeichnis.
RUN install -d -o bpmninja -g bpmninja -m 0755 /app

COPY --from=builder /tmp/engine-server /usr/local/bin/engine-server
ENV PORT=8081
ENV NATS_URL=nats://nats:4222
EXPOSE 8081

# /api/health ist auth-frei und braucht weder NATS noch sonstigen State.
HEALTHCHECK --interval=30s --timeout=3s --start-period=15s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${PORT}/api/health" || exit 1

USER 10001:10001
CMD ["engine-server"]
