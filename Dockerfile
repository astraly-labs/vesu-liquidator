# Build stage
FROM rust:1.80 as builder
RUN apt-get update && apt-get install -y protobuf-compiler libprotobuf-dev
WORKDIR /usr/src/vesu-liquidator
COPY . .
RUN cargo build --release

# Run stage
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/vesu-liquidator/target/release/vesu-liquidator /usr/local/bin/vesu-liquidator
COPY config.yaml /usr/local/bin/config.yaml
WORKDIR /usr/local/bin
ENTRYPOINT ["vesu-liquidator"]
