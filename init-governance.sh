#!/bin/bash
solana -v --url http://localhost:8899 --keypair /opt/artifacts/creator.keypair airdrop 100
solana -v --url http://localhost:8899 --keypair /opt/artifacts/payer.keypair airdrop 100

solana -v --url http://localhost:8899 --keypair /opt/artifacts/voters/voter1.keypair airdrop 100
solana -v --url http://localhost:8899 --keypair /opt/artifacts/voters/voter2.keypair airdrop 100
solana -v --url http://localhost:8899 --keypair /opt/artifacts/voters/voter3.keypair airdrop 100
solana -v --url http://localhost:8899 --keypair /opt/artifacts/voters/voter4.keypair airdrop 100
solana -v --url http://localhost:8899 --keypair /opt/artifacts/voters/voter5.keypair airdrop 100

spl-token --url http://localhost:8899 create-token --decimals 6 --fee-payer /opt/artifacts/payer.keypair /opt/artifacts/community-mint.keypair

solana program deploy --url http://localhost:8899 --program-id /opt/artifacts/spl-governance.keypair -v /opt/spl_governance.so

solana program deploy --url http://localhost:8899 --program-id /opt/artifacts/addin-fixed-weights.keypair -v /opt/spl_governance_addin_fixed_weights.so

solana program deploy --url http://localhost:8899 --program-id /opt/artifacts/addin-vesting.keypair -v /opt/spl_governance_addin_vesting.so

solana program deploy --url http://localhost:8899 --program-id /opt/artifacts/maintenance.keypair -v /opt/maintenance.so
