#!/bin/bash
set -euo pipefail

echo "Neon Governance revision = ${BUILDKITE_COMMIT}"

set ${SOLANA_REVISION:=v1.9.12-testnet-with_trx_cap}

docker pull solanalabs/solana:${SOLANA_REVISION}
echo "SOLANA_REVISION=$SOLANA_REVISION"

docker build --build-arg REVISION=${BUILDKITE_COMMIT} --build-arg SOLANA_REVISION=$SOLANA_REVISION -t neonlabsorg/neon-governance:${BUILDKITE_COMMIT} .
