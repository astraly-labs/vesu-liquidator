FROM lukemathwalker/cargo-chef:latest-rust-1.87.0 AS chef
WORKDIR /app/

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update && \
    apt-get install -y pkg-config protobuf-compiler libprotobuf-dev libssl-dev
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release

# We do not need the Rust toolchain to run the binary!
FROM ubuntu:24.04 AS runtime
RUN apt-get update && \
    apt-get install -y tini ca-certificates && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app/
COPY --from=builder /app/target/release/vesu-liquidator /usr/local/bin
COPY --from=builder /app/config.yaml .

ENTRYPOINT ["tini", "--", "vesu-liquidator"]
CMD ["--help"]
