FROM rust:1.87-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY common/Cargo.toml common/Cargo.toml
COPY backend/Cargo.toml backend/Cargo.toml
COPY frontend/Cargo.toml frontend/Cargo.toml

RUN mkdir -p common/src backend/src frontend/src \
 && echo "pub fn _stub() {}" > common/src/lib.rs \
 && echo "fn main() {}" > backend/src/main.rs \
 && echo "fn main() {}" > frontend/src/main.rs

RUN cargo fetch

COPY common/ common/
COPY backend/ backend/
RUN cargo build --release --package server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/server ./server
COPY database/init.sql ./database/init.sql

RUN mkdir -p /var/lib/fund_transparency data static

EXPOSE 3000
CMD ["./server"]
