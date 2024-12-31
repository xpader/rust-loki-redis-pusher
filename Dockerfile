FROM clux/muslrust:1.83.0-stable as builder

ENV RUSTUP_DIST_SERVER=https://mirrors.ustc.edu.cn/rust-static RUSTUP_UPDATE_ROOT=https://mirrors.ustc.edu.cn/rust-static/rustup
COPY docker/config.toml /root/.cargo/

WORKDIR /app

# Cache build stage
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p /app/src && echo 'fn main() { println!("Dummy") }' > ./src/main.rs
RUN cargo build --release
RUN rm -r target/x86_64-unknown-linux-musl/release/.fingerprint/rust-loki-redis-pusher-*

# Build app
COPY . .
RUN cargo build --release

FROM alpine:3.18.4

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rust-loki-redis-pusher /usr/bin/loki-redis-pusher

ENTRYPOINT ["/usr/bin/loki-redis-pusher"]