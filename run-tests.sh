#!/bin/bash
set -euo pipefail

solana config -u ${SOLANA_URL:-http://localhost:8899}

./init-governance.sh