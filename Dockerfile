FROM alpine
COPY target/x86_64-unknown-linux-musl/release/traefik-dsd /
ENTRYPOINT ["/traefik-dsd"]
