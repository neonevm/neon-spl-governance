// =========================================================================
// Deploy evm_loader from buffer account
// =========================================================================

use crate::prelude::*;

pub fn deploy_evm_loader_from_buffer(_wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        cfg: &Configuration,
        evm_loader_pubkey: Pubkey,
        buffer_pubkey: Pubkey,
        upgrade_authority_pubkey: Pubkey,
) -> Result<(), ScriptError> {

    let buffer_account_opt: Option<Account> = client.get_account(&buffer_pubkey)?;
    if let Some(buffer_account) = buffer_account_opt {
        let data_len: usize = buffer_account.data.len();
        let minimum_balance_for_rent_exemption = cfg.client.get_minimum_balance_for_rent_exemption(data_len).unwrap();

        let instructions: Vec<InstructionData> =
            bpf_loader_upgradeable::deploy_with_max_program_len(
                &client.payer.pubkey(),
                &evm_loader_pubkey,
                &buffer_pubkey,
                &upgrade_authority_pubkey,
                minimum_balance_for_rent_exemption,
                data_len * 2,
            )?
            .drain(..)
            .map(|instruction| instruction.into() )
            .collect();

        transaction_inserter.insert_transaction_checked(
                &format!("Deploy evm_loader from buffer at address {}", buffer_pubkey),
                instructions,
            )?;
    }

    Ok(())
}
