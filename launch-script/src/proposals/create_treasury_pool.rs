// =========================================================================
// Create collateral pool accounts
// =========================================================================

use crate::prelude::*;

pub fn create_collateral_pool_accounts(wallet: &Wallet, transaction_inserter: &mut ProposalTransactionInserter, cfg: &Configuration) -> Result<(), ScriptError> {

    let minimum_balance_for_rent_exemption = cfg.client.get_minimum_balance_for_rent_exemption(0).unwrap();

    for index in 0u32..10 {
        let seed: String = format!("collateral_seed_{}", index.to_string().as_str());
        println!("\nCollateral Poool Seed: {}", seed);

        let collateral_pool_account: Pubkey =  Pubkey::create_with_seed(&wallet.maintenance_program_id, &seed, &cfg.wallet.neon_evm_program_id).unwrap();

        transaction_inserter.insert_transaction_checked(
                &format!("Create collateral pool account [{}] - {}", index, collateral_pool_account),
                vec![
                    system_instruction::create_account_with_seed(
                        &wallet.maintenance_program_id,
                        &collateral_pool_account,
                        &wallet.maintenance_program_id,
                        seed.as_str(),
                        minimum_balance_for_rent_exemption,
                        0,
                        &cfg.wallet.neon_evm_program_id,
                    ).into(),
                ],
            )?;
    }

    Ok(())
}
