#!/bin/bash
set -euo pipefail

solana -v --keypair artifacts/creator.keypair airdrop 100
solana -v --keypair artifacts/payer.keypair airdrop 100

solana -v --keypair artifacts/voters/voter1.keypair airdrop 100
solana -v --keypair artifacts/voters/voter2.keypair airdrop 100
solana -v --keypair artifacts/voters/voter3.keypair airdrop 100
solana -v --keypair artifacts/voters/voter4.keypair airdrop 100
solana -v --keypair artifacts/voters/voter5.keypair airdrop 100

spl-token --url create-token --decimals 6 --fee-payer artifacts/payer.keypair artifacts/community-mint.keypair

solana program deploy --url --program-id artifacts/spl-governance.keypair -v deploy/spl_governance.so

solana program deploy --url --program-id artifacts/addin-fixed-weights.keypair -v deploy/spl_governance_addin_fixed_weights.so

solana program deploy --url --program-id artifacts/addin-vesting.keypair -v deploy/spl_governance_addin_vesting.so

solana program deploy --url --program-id artifacts/maintenance.keypair -v deploy/maintenance.so
