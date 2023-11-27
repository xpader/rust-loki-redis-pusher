FROM alpine:3.18.4

WORKDIR /loki-redis-pusher/
COPY target/x86_64-unknown-linux-musl/release/rust-loki-redis-pusher /loki-redis-pusher/loki-redis-pusher

ENTRYPOINT ["/loki-redis-pusher/loki-redis-pusher"]