# Install BPF SDK
FROM solanalabs/rust:1.62.0 AS builder
# Use hardcoded solana revision for install SDK to prevent long rebuild when use other SOLANA_REVISION
RUN sh -c "$(curl -sSfL https://release.solana.com/v1.10.29/install)" && \
    /root/.local/share/solana/install/active_release/bin/sdk/bpf/scripts/install.sh
ENV PATH=/root/.local/share/solana/install/active_release/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# Build governance
# Note: create stub Cargo.toml to speedup build
FROM builder AS governance-builder
COPY ./ /opt/neon-governance/

WORKDIR /opt/neon-governance
RUN cargo clippy && cargo build-bpf && cargo test-bpf && cargo build --release

WORKDIR /opt/neon-governance/solana-program-library/governance/program
RUN cargo build-bpf

# Build target image
FROM ubuntu:20.04 AS base
WORKDIR /opt

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get -y install libssl1.1 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=governance-builder /opt/neon-governance/solana-program-library/target/deploy/*.so /opt/deploy/
COPY --from=governance-builder /opt/neon-governance/target/deploy/*.so /opt/deploy/
COPY --from=governance-builder /opt/neon-governance/target/release/vesting-contract-cli /opt/

ENV PATH=/opt/solana/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/opt
