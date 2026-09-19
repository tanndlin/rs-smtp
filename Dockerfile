# Single Dockerfile for every service. The shared `builder` stage compiles the
# whole workspace once; each service is a small runtime stage selected with
# `target:` in docker-compose.yml.

# ---------- builder ----------
FROM rust:1.98 AS builder
WORKDIR /app

# No DB reachable during `docker build`, so sqlx::query! must check against the
# committed .sqlx/ offline cache instead of a live database.
ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY .sqlx ./.sqlx
COPY util ./util
COPY consumer ./consumer
COPY imap ./imap
COPY smtp ./smtp

# The cargo registry and target/ live in BuildKit cache mounts, so deps and
# unchanged crates are reused across builds (and across services) instead of
# being recompiled whenever this layer reruns. Building --workspace (not -p)
# unifies features so every service links against the same dep artifacts.
# Mounts aren't part of the image, so the binaries are copied out to /out.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target,sharing=locked \
    cargo build --release --workspace \
    && mkdir -p /out \
    && cp target/release/consumer target/release/imap target/release/smtp /out/

# ---------- runtime ----------
FROM debian:trixie-slim AS runtime
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

FROM runtime AS consumer
COPY --from=builder /out/consumer /app/consumer
ENV API_PORT=2525
EXPOSE 2525
CMD ["./consumer"]

FROM runtime AS imap
COPY --from=builder /out/imap /app/imap
EXPOSE 143
CMD ["./imap"]

FROM runtime AS smtp
COPY --from=builder /out/smtp /app/smtp
ENV API_PORT=2525
EXPOSE 2525
CMD ["./smtp"]
