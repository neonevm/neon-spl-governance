// =========================================================================
// Create collateral pool accounts
// =========================================================================

use crate::prelude::*;

const TREASURY_POOL_ACCOUNT_COUNT: u32 = 10;

pub fn create_collateral_pool_accounts(wallet: &Wallet, client: &Client, transaction_inserter: &mut ProposalTransactionInserter, cfg: &Configuration) -> Result<(), ScriptError> {

    let realm = Realm::new(
        cfg.client,
        &wallet.governance_program_id,
        REALM_NAME,
        &wallet.community_pubkey,
    );
    let maintenance_governance = realm.governance(&wallet.neon_evm_program_id);
    let maintenance_governance_pubkey: Pubkey = maintenance_governance.governance_address;
    
    let (maintenance_record_pubkey,_): (Pubkey,u8) =
        get_maintenance_record_address(&wallet.maintenance_program_id, &wallet.neon_evm_program_id);
    
    println!("Maintenance DAO Address: {:?}", maintenance_governance_pubkey);
    println!("Maintenance Record Address: {:?}", maintenance_record_pubkey);
    

    let minimum_balance_for_rent_exemption = cfg.client.get_minimum_balance_for_rent_exemption(0).unwrap();

    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    for index in 0u32..TREASURY_POOL_ACCOUNT_COUNT {
        let seed: String = format!("collateral_seed_{}", index);
        println!("\nCollateral Poool Seed: {}", seed);

        let collateral_pool_account: Pubkey =  Pubkey::create_with_seed(&maintenance_governance_pubkey, &seed, &cfg.wallet.neon_evm_program_id).unwrap();

        executor.check_and_create_object(
            "Proposal Internal Payer",
            client.get_account(&collateral_pool_account)?,
            |_| {
                Ok(None)
            },
            || {
                let transaction = client.create_transaction_with_payer_only(
                    &[
                        system_instruction::transfer(
                            &client.payer.pubkey(),             // from
                            &collateral_pool_account,          // to
                            minimum_balance_for_rent_exemption, // lamports
                        ),
                    ],
                )?;
                Ok(Some(transaction))
            },
        )?;

        transaction_inserter.insert_transaction_checked(
                &format!("Create collateral pool account [{}] - {}", index, collateral_pool_account),
                vec![
                    system_instruction::assign_with_seed(
                        &collateral_pool_account,
                        &maintenance_governance_pubkey,
                        seed.as_str(),
                        &cfg.wallet.neon_evm_program_id,
                    ).into(),
                ],
            )?;
    }

    Ok(())
}
