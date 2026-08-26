# ---------- builder ----------
FROM rust:1.98 AS builder
WORKDIR /app

# No DB reachable during `docker build`, so sqlx::query! must check against the
# committed .sqlx/ offline cache instead of a live database.
ENV SQLX_OFFLINE=true

COPY Cargo.toml Cargo.lock ./

# Build deps only using dummy src — this layer is cached until Cargo.toml/Cargo.lock changes
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY migrations ./migrations
COPY .sqlx ./.sqlx

# Copy real source and touch main.rs so cargo knows to recompile
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

# ---------- runtime ----------
FROM debian:trixie-slim
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

COPY --from=builder /app/target/release/consumer /app/consumer

ENV API_PORT=2525
EXPOSE 2525
CMD ["./consumer"]
