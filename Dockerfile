FROM alpine:3.18.4

COPY target/x86_64-unknown-linux-musl/release/rust-loki-redis-pusher /usr/bin/loki-redis-pusher

ENTRYPOINT ["/usr/bin/loki-redis-pusher"]