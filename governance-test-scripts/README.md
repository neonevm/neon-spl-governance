# spl-governance-dev


## Preparing steps:
```
solana-keygen new -f -o governance-test-scripts/community_mint.keypair --no-bip39-passphrase

solana config set -u http://localhost:8899
solana airdrop 1000 artifacts/payer.keypair
solana program deploy -k artifacts/payer.keypair --upgrade-authority artifacts/creator.keypair artifacts/spl_governance.so
solana program deploy -k artifacts/payer.keypair --upgrade-authority artifacts/creator.keypair artifacts/spl_governance_addin_fixed_weights.so
solana program deploy -k artifacts/payer.keypair --upgrade-authority artifacts/creator.keypair artifacts/spl_governance_addin_vesting.so
```




Snippets:
1. Forced send transaction (in case of fail you can see it on Solana blockexplorer):
```
        //self.get_interactor().solana_client.send_and_confirm_transaction(&transaction)
        self.get_interactor().solana_client.send_and_confirm_transaction_with_spinner_and_config(&transaction, 
                self.get_interactor().solana_client.commitment(),
                RpcSendTransactionConfig {skip_preflight: true, ..RpcSendTransactionConfig::default()})
```
