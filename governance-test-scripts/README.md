# spl-governance-dev


## Preparing steps:
```
solana config set -u http://localhost:8899
solana airdrop 1000 artifacts/payer.keypair
solana program deploy -k artifacts/payer.keypair solana-program-library/target/deploy/spl_governance.so
solana program deploy -k artifacts/payer.keypair target/deploy/spl_governance_addin_fixed_weights.so
spl-token -u http://localhost:8899 create-token --decimals 6 --fee-payer artifacts/payer.keypair --mint-authority artifacts/voters/voter1.keypair governance-test-scripts/community_mint.keypair
```
