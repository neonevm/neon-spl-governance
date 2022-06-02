#!/bin/bash
set -euo pipefail

solana config set --url ${SOLANA_URL:-http://localhost:8899}

./init-governance.sh