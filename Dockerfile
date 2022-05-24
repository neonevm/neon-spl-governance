ARG SOLANA_REVISION=v1.9.12-testnet-with_trx_cap
# Install BPF SDK
FROM solanalabs/rust:latest AS builder
RUN rustup toolchain install stable
RUN rustup component add clippy --toolchain stable
WORKDIR /opt
RUN sh -c "$(curl -sSfL https://release.solana.com/stable/install)" && \
    /root/.local/share/solana/install/active_release/bin/sdk/bpf/scripts/install.sh
ENV PATH=/root/.local/share/solana/install/active_release/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# Build governance
# Note: create stub Cargo.toml to speedup build
FROM builder AS governance-builder
COPY ./ /opt/neon-governance/

WORKDIR /opt/neon-governance
RUN cargo clippy && cargo build-bpf

WORKDIR /opt/neon-governance/solana-program-library/governance/program
RUN cargo build-bpf

# Define solana-image that contains utility
FROM neonlabsorg/solana:${SOLANA_REVISION} AS solana

# Build target image
FROM ubuntu:20.04 AS base
WORKDIR /opt

COPY --from=solana /opt/solana/bin/solana /opt/solana/bin/solana-keygen /opt/solana/bin/
COPY --from=governance-builder /opt/neon-governance/solana-program-library/target/deploy/*.so /opt/
COPY --from=governance-builder /opt/neon-governance/target/deploy/*.so /opt/
COPY context/spl-token /opt/solana/bin/
COPY context/libssl.so.1.1 context/libcrypto.so.1.1 /usr/lib/x86_64-linux-gnu/
COPY artifacts/creator.keypair /root/.config/solana/id.json
COPY artifacts/*.keypair /opt/artifacts/
COPY artifacts/voters/*.keypair /opt/artifacts/voters/
COPY init-governance.sh /opt/

ENV PATH=/opt/solana/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/opt
