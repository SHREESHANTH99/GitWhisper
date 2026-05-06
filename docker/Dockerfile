FROM rust:1.86-bullseye AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY src ./src
COPY README.md ./

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
COPY --from=builder /app/target/release/gitwhisper /usr/local/bin/gitwhisper

ENV GITWHISPER_USER=docker-admin
ENTRYPOINT ["gitwhisper"]
CMD ["--help"]
