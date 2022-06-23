ARG SOLANA_REVISION=v1.9.12
# Install BPF SDK
FROM solanalabs/rust:latest AS builder
RUN rustup toolchain install stable
RUN rustup component add clippy --toolchain stable
RUN cargo install spl-token-cli
WORKDIR /opt
RUN sh -c "$(curl -sSfL https://release.solana.com/stable/install)" && \
    /root/.local/share/solana/install/active_release/bin/sdk/bpf/scripts/install.sh
ENV PATH=/root/.local/share/solana/install/active_release/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# Build governance
# Note: create stub Cargo.toml to speedup build
FROM builder AS governance-builder
COPY ./ /opt/neon-governance/

WORKDIR /opt/neon-governance
RUN cargo clippy && cargo test-bpf && cargo build-bpf && cargo build --release

WORKDIR /opt/neon-governance/solana-program-library/governance/program
RUN cargo build-bpf

# Define solana-image that contains utility
FROM solanalabs/solana:${SOLANA_REVISION} AS solana

# Build target image
FROM ubuntu:20.04 AS base
WORKDIR /opt

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get -y install libssl1.1 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=solana /usr/bin/solana /usr/bin/solana-keygen /opt/solana/bin/
COPY --from=governance-builder /usr/local/cargo/bin/spl-token /opt/solana/bin/
COPY --from=governance-builder /opt/neon-governance/solana-program-library/target/deploy/*.so /opt/deploy/
COPY --from=governance-builder /opt/neon-governance/target/deploy/*.so /opt/deploy/
COPY --from=governance-builder /opt/neon-governance/target/release/launch-scrupt /opt/
COPY artifacts/creator.keypair /root/.config/solana/id.json
COPY artifacts/*.keypair /opt/artifacts/
COPY artifacts/voters/*.keypair /opt/artifacts/voters/
COPY init-governance.sh /opt/
COPY run-tests.sh /opt/

ENV PATH=/opt/solana/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/opt
