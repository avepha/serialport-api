# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libudev-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libudev1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/serialport-api /usr/local/bin/serialport-api
EXPOSE 4002
ENTRYPOINT ["/usr/local/bin/serialport-api"]
CMD ["serve", "--host", "0.0.0.0", "--port", "4002"]
