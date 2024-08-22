# ====== Build stage ======
FROM rust:1.80 AS builder

COPY . .

RUN apt-get update && \
    apt-get install -y pkg-config protobuf-compiler libprotobuf-dev libssl-dev

RUN cargo build --release

# ====== Run stage ======
FROM ubuntu:24.04 AS runner

RUN apt-get update && \
    apt-get install -y tini ca-certificates && \
    apt-get autoremove -y && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /target/release/vesu-liquidator /usr/local/bin/vesu-liquidator
COPY --from=builder /config.yaml .

ENTRYPOINT ["tini", "--", "vesu-liquidator"]
CMD ["--help"]
