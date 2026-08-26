# ---------- builder ----------
FROM rust:1.98 AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Build deps only using dummy src — this layer is cached until Cargo.toml/Cargo.lock changes
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy real source and touch main.rs so cargo knows to recompile
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

# ---------- runtime ----------
FROM debian:trixie-slim
WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates && update-ca-certificates

COPY --from=builder /app/target/release/smtp /app/smtp

ENV API_PORT=2525
EXPOSE 2525
CMD ["./smtp"]
