use crate::prelude::*;
use maintenance::state::MaintenanceRecord;

fn check_neon_evm(
    wallet: &Wallet,
    client: &Client,
    cfg: &Configuration,
) -> Result<(), ScriptError> {
    let scope_name = "neon-evm";

    let maintenance_program_id = &wallet.maintenance_program_id;
    let neon_evm_program_id = &wallet.neon_evm_program_id;
    let governance_program_id = &wallet.governance_program_id;
    let community_pubkey = &wallet.community_pubkey;
    let creator_pubkey = &wallet.creator_pubkey;
    let payer = &wallet.payer_keypair.pubkey();

    let executor = TransactionExecutor {
        client,
        setup: cfg.send_trx,
        verbose: cfg.verbose,
    };
    let mut collector = TransactionCollector::new(
        client,
        cfg.send_trx,
        cfg.verbose,
        "Preparing environment EVM",
    );

    //---- 1. Check Maintenance DAO for exists -----------------------------------------------------
    let realm = Realm::new(client, governance_program_id, REALM_NAME, community_pubkey);
    let maintenance_dao = realm.governance(neon_evm_program_id);
    maintenance_dao
        .get_data()?
        .ok_or(StateError::MissingGovernance(*neon_evm_program_id))?;

    //---- 2. Check and create MaintenanceRecord for neon_evm --------------------------------------
    let maintenance_record_pubkey =
        get_maintenance_record_address(maintenance_program_id, neon_evm_program_id).0;

    executor.check_and_create_object(
        &format!("MaintenanceRecord for {}: {:?}", scope_name, maintenance_record_pubkey),
        client.get_account_data_borsh::<MaintenanceRecord>(
            maintenance_program_id,
            &maintenance_record_pubkey,
        )?,
        |account_data| {
            if (account_data.authority == maintenance_dao.governance_address
                || account_data.authority == *creator_pubkey)
                && account_data.maintained_address == *neon_evm_program_id
            {
                Ok(None)
            } else {
                Err(StateError::InvalidMaintenanceRecord(
                    *maintenance_program_id,
                    *neon_evm_program_id,
                )
                .into())
            }
        },
        || {
            let transaction = client.create_transaction_with_payer_only(
                &[create_maintenance(
                    maintenance_program_id,
                    neon_evm_program_id,
                    &maintenance_dao.governance_address,
                    payer,
                )],
            )?;
            Ok(Some(transaction))
        },
    )?;

    //---- 3. Pass neon-evm upgrade authority under MaintenanceRecord ------------------------------
    collector.check_and_create_object(
        &format!("{} upgrade-authority", scope_name),
        Some(client.get_program_upgrade_authority(neon_evm_program_id)?),
        |&upgrade_authority| {
            if upgrade_authority == Some(*creator_pubkey) {
                let instructions = [client.set_program_upgrade_authority_instruction(
                    neon_evm_program_id,
                    creator_pubkey,
                    Some(&maintenance_record_pubkey),
                )?]
                .to_vec();
                let signers = [wallet.get_creator_keypair()?].to_vec();
                Ok(Some((instructions, signers)))
            } else if upgrade_authority == Some(maintenance_record_pubkey) {
                Ok(None)
            } else {
                Err(StateError::InvalidProgramUpgradeAuthority(
                    *neon_evm_program_id,
                    upgrade_authority,
                )
                .into())
            }
        },
        || Err(StateError::InvalidProgram(*neon_evm_program_id).into()),
    )?;
    collector.execute_transaction()?;

    Ok(())
}

fn create_neon_balance_token(
    wallet: &Wallet,
    client: &Client,
    cfg: &Configuration,
) -> Result<(), ScriptError> {
    let scope_name = "neon-evm";

    let neon_evm_program_id = &wallet.neon_evm_program_id;
    let governance_program_id = &wallet.governance_program_id;
    let community_pubkey = &wallet.community_pubkey;
    let payer = &wallet.payer_keypair.pubkey();

    let realm = Realm::new(client, governance_program_id, REALM_NAME, community_pubkey);
    let maintenance_dao = realm.governance(neon_evm_program_id);

    let neon_token_program_id = Pubkey::find_program_address(&[b"Deposit"], neon_evm_program_id).0;
    let neon_evm_token_account = spl_associated_token_account::get_associated_token_address(
        &neon_token_program_id,
        community_pubkey,
    );
    println!(
        "Neon-EVM governance address: {}",
        maintenance_dao.governance_address
    );
    println!("Neon-EVM token account: {}", neon_evm_token_account);

    let executor = TransactionExecutor {
        client,
        setup: cfg.send_trx,
        verbose: cfg.verbose,
    };

    executor.check_and_create_object(
        &format!("NEON-token {} account", scope_name),
        get_account_data(client, &neon_evm_token_account)?,
        |d| {
            assert_is_valid_account_data(
                d,
                &neon_evm_token_account,
                community_pubkey,
                &maintenance_dao.governance_address,
            )?;
            Ok(None)
        },
        || {
            let transaction = client.create_transaction_with_payer_only(&[
                spl_associated_token_account::instruction::create_associated_token_account(
                    payer,
                    &maintenance_dao.governance_address,
                    community_pubkey,
                    &spl_token::id(),
                ),
            ])?;
            Ok(Some(transaction))
        },
    )?;

    Ok(())
}

pub fn process_environment_evm(
    wallet: &Wallet,
    client: &Client,
    cfg: &Configuration,
) -> Result<(), ScriptError> {
    // Check that Neon-evm program exists and owned by creator or MaintenanceRecord
    check_neon_evm(wallet, client, cfg)?;

    // Create NEON associated token account
    create_neon_balance_token(wallet, client, cfg)?;

    Ok(())
}
