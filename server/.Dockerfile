FROM debian:bookworm-slim
WORKDIR /app

COPY ./target/release/lf-server /app/lf-server
CMD ["/app/lf-server", "/app/config.json"]
