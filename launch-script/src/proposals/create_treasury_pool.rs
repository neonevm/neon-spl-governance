// =========================================================================
// Create collateral pool accounts
// =========================================================================

use crate::prelude::*;

pub fn create_collateral_pool_accounts(_wallet: &Wallet, _client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        cfg: &Configuration,
        evm_loader_pubkey: Pubkey,
        collateral_pool_base_pubkey: Pubkey,
) -> Result<(), ScriptError> {

    let mut seed: String = String::with_capacity(25);
    let minimum_balance_for_rent_exemption = cfg.client.get_minimum_balance_for_rent_exemption(0).unwrap();

    for index in 0u32..10 {
        seed.push_str("collateral_seed_");
        seed.push_str(index.to_string().as_str());
        println!("\nCollateral Poool Seed: {}", seed);

        let collateral_pool_account: Pubkey =  Pubkey::create_with_seed(&collateral_pool_base_pubkey, &seed, &evm_loader_pubkey).unwrap();

        transaction_inserter.insert_transaction_checked(
                &format!("Create collateral pool account [{}] - {}", index, collateral_pool_account),
                vec![
                    system_instruction::create_account_with_seed(
                        &collateral_pool_base_pubkey,
                        &collateral_pool_account,
                        &collateral_pool_base_pubkey,
                        seed.as_str(),
                        minimum_balance_for_rent_exemption,
                        0,
                        &evm_loader_pubkey,
                    ).into(),
                ],
            )?;
    }

    Ok(())
}
