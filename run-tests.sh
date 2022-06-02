#!/bin/bash
set -euo pipefail

solana set config -u ${SOLANA_URL:-http://localhost:8899}

./init-governance.sh