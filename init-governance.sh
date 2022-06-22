#!/bin/bash
set -euo pipefail

solana -v --keypair artifacts/payer.keypair airdrop 100

solana -v --keypair artifacts/voters/voter1.keypair airdrop 100
solana -v --keypair artifacts/voters/voter2.keypair airdrop 100
solana -v --keypair artifacts/voters/voter3.keypair airdrop 100
solana -v --keypair artifacts/voters/voter4.keypair airdrop 100
solana -v --keypair artifacts/voters/voter5.keypair airdrop 100

spl-token create-token --decimals 9 --fee-payer artifacts/payer.keypair --mint-authority artifacts/creator.keypair -- artifacts/community-mint.keypair

solana program deploy -v --keypair artifacts/payer.keypair --upgrade-authority artifacts/creator.keypair --program-id artifacts/spl-governance.keypair       deploy/spl_governance.so
solana program deploy -v --keypair artifacts/payer.keypair --upgrade-authority artifacts/creator.keypair --program-id artifacts/addin-fixed-weights.keypair  deploy/spl_governance_addin_fixed_weights.so
solana program deploy -v --keypair artifacts/payer.keypair --upgrade-authority artifacts/creator.keypair --program-id artifacts/addin-vesting.keypair        deploy/spl_governance_addin_vesting.so
solana program deploy -v --keypair artifacts/payer.keypair --upgrade-authority artifacts/creator.keypair --program-id artifacts/maintenance.keypair          deploy/maintenance.so
