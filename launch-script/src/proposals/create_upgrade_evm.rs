// =========================================================================
// Deploy evm_loader from buffer account
// =========================================================================
use solana_sdk::{
    bpf_loader_upgradeable,
};
use crate::prelude::*;

pub fn create_upgrade_evm(client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        cfg: &Configuration,
        buffer_pubkey: Pubkey,
) -> Result<(), ScriptError> {

    transaction_inserter.insert_transaction_checked(
            &format!("Upgrade evm_loader from buffer at address {}", buffer_pubkey),
            vec![
                bpf_loader_upgradeable::upgrade(
                    &cfg.wallet.neon_evm_program_id,
                    &buffer_pubkey,
                    &cfg.maintenance_program_address,
                    &client.payer.pubkey(),
                ).into(),
            ],
        )

}
