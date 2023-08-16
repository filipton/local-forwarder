FROM debian:bookworm-slim
WORKDIR /app

COPY ./target/release/local-forwarder /app/local-forwarder
CMD ["/app/local-forwarder", "/app/config.json"]
