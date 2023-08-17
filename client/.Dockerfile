FROM debian:bookworm-slim
WORKDIR /app

COPY ./target/release/lf-client /app/lf-client
CMD ["/app/lf-client", "/app/config.json"]
