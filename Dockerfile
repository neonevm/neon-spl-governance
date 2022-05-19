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
WORKDIR /opt/neon-governance/addin-fixed-weights
RUN cd program
# RUN cd program && /opt/evm_loader/ci_checks.sh
# ARG REVISION
# ENV NEON_REVISION=${REVISION}
RUN cargo clippy && \
    # cargo build --release && \
    cargo build-bpf
    # cargo build-bpf --features no-logs,testnet && cp target/deploy/evm_loader.so target/deploy/evm_loader-testnet.so && \
    # cargo build-bpf --features no-logs,alpha && cp target/deploy/evm_loader.so target/deploy/evm_loader-alpha.so && \
    # cargo build-bpf --features no-logs,mainnet && cp target/deploy/evm_loader.so target/deploy/evm_loader-mainnet.so && \
    # cargo build-bpf --features no-logs
