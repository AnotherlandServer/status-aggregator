# 1. Build stage
FROM rust:latest as builder

WORKDIR /app

# Build actual project
COPY . .
RUN cargo build --release

# 2. Runtime stage
FROM debian:stable-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/status-aggregator /usr/local/bin/status-aggregator

ENTRYPOINT ["status-aggregator"]
