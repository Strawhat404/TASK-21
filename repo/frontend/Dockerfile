FROM rust:1.87-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

# Install trunk
RUN curl -L --retry 5 --retry-delay 3 \
    https://github.com/trunk-rs/trunk/releases/download/v0.21.4/trunk-x86_64-unknown-linux-gnu.tar.gz \
    | tar -xz -C /usr/local/bin

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
COPY frontend/ frontend/

# Build frontend — output goes to static/dist per Trunk.toml
RUN cd frontend && trunk build --release

FROM nginx:alpine
COPY --from=builder /app/static/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
