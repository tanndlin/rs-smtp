# ---------- builder ----------
# Build context is the repo root (see docker-compose.yml) since this crate
# depends on the ../util workspace member.
FROM rust:1.98 AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY util/Cargo.toml util/Cargo.toml
COPY consumer/Cargo.toml consumer/Cargo.toml
COPY imap/Cargo.toml imap/Cargo.toml
COPY smtp/Cargo.toml smtp/Cargo.toml

# Build deps only using dummy sources for every workspace member — this layer
# is cached until any Cargo.toml/Cargo.lock changes
RUN mkdir -p util/src consumer/src imap/src smtp/src \
    && echo "" > util/src/lib.rs \
    && echo "fn main() {}" > consumer/src/main.rs \
    && echo "fn main() {}" > imap/src/main.rs \
    && echo "fn main() {}" > smtp/src/main.rs
RUN cargo build --release -p smtp
RUN rm -rf util/src smtp/src

# Copy real source and touch main.rs so cargo knows to recompile
COPY util/src ./util/src
COPY smtp/src ./smtp/src
RUN touch util/src/lib.rs smtp/src/main.rs
RUN cargo build --release -p smtp

# ---------- runtime ----------
FROM debian:trixie-slim
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

COPY --from=builder /app/target/release/smtp /app/smtp

ENV API_PORT=2525
EXPOSE 2525
CMD ["./smtp"]
