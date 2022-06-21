use crate::prelude::*;

pub fn process_environment_dao(
    wallet: &Wallet,
    client: &Client,
    cfg: &Configuration,
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {
        client,
        setup: cfg.send_trx,
        verbose: cfg.verbose,
    };

    let realm = Realm::new(
        client,
        &wallet.governance_program_id,
        REALM_NAME,
        &wallet.community_pubkey,
    );
    let fixed_weight_addin = AddinFixedWeights::new(client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(client, wallet.vesting_addin_id);
    let main_governance = realm.governance(&wallet.community_pubkey);
    let emergency_governance = realm.governance(&wallet.governance_program_id);
    let maintenance_governance = realm.governance(&wallet.neon_evm_program_id);
    let neon_multisig = cfg.neon_multisig_address();
    let token_distribution = cfg.get_token_distribution()?;

    println!("Install MultiSig accounts");
    for msig in &cfg.multi_sigs {
        setup_msig(wallet, client, &executor, msig)?;
    }

    // ----------- Check or create community mint ----------------------
    executor.check_and_create_object(
        "Mint",
        get_mint_data(client, &wallet.community_pubkey)?,
        |d| {
            if !d.mint_authority.contains(&wallet.creator_pubkey)
                && !d
                    .mint_authority
                    .contains(&main_governance.governance_address)
                && !d.mint_authority.contains(&neon_multisig)
            {
                return Err(StateError::InvalidMintAuthority(
                    wallet.community_pubkey,
                    d.mint_authority,
                )
                .into());
            }
            if d.decimals != 9 {
                return Err(StateError::InvalidMintPrecision(wallet.community_pubkey).into());
            }
            Ok(None)
        },
        || Err(StateError::MissingMint(wallet.community_pubkey).into()),
    )?;

    // -------------- Check or create Realm ---------------------------
    executor.check_and_create_object(
        "Realm",
        realm.get_data()?,
        |d| {
            if d.community_mint != realm.community_mint {
                return Err(StateError::InvalidRealmCommunityMint(
                    realm.realm_address,
                    d.community_mint,
                )
                .into());
            }
            if d.authority != Some(wallet.creator_pubkey)
                && d.authority != Some(main_governance.governance_address)
            {
                return Err(
                    StateError::InvalidRealmAuthority(realm.realm_address, d.authority).into(),
                );
            }
            Ok(None)
        },
        || {
            let transaction =
                client.create_transaction_with_payer_only(&[realm.create_realm_instruction(
                    &wallet.creator_pubkey,
                    &cfg.startup_realm_config,
                )])?;
            Ok(Some(transaction))
        },
    )?;

    // ------------ Setup and configure max_voter_weight_record ----------------
    // TODO check max_voter_weight_record_address created correctly
    let max_voter_weight_record_address = fixed_weight_addin
        .setup_max_voter_weight_record(&realm)
        .unwrap();
    realm.settings_mut().max_voter_weight_record_address = Some(max_voter_weight_record_address);

    // ------------ Transfer tokens to Vesting-addin MaxVoterWeightRecord ------
    {
        let record_address = vesting_addin.get_max_voter_weight_record_address(&realm);
        let record_length = vesting_addin.get_max_voter_weight_account_size();
        let record_lamports = Rent::default().minimum_balance(record_length);
        executor.check_and_create_object(
            "Vesting max_voter_weight_record",
            client.get_account(&record_address)?,
            |v| {
                if v.lamports < record_lamports {
                    let transaction = client.create_transaction_with_payer_only(&[
                        system_instruction::transfer(
                            &wallet.payer_keypair.pubkey(),
                            &record_address,
                            record_lamports - v.lamports,
                        ),
                    ])?;
                    return Ok(Some(transaction));
                }
                Ok(None)
            },
            || {
                let transaction =
                    client.create_transaction_with_payer_only(&[system_instruction::transfer(
                        &wallet.payer_keypair.pubkey(),
                        &record_address,
                        record_lamports,
                    )])?;
                Ok(Some(transaction))
            },
        )?;
    }

    // -------------------- Setup multisig record --------- --------------------
    executor.check_and_create_object(
        &format!("Governance multisig for spl_token {}", neon_multisig),
        get_multisig_data(client, &neon_multisig)?,
        |_d| {
            //assert_is_valid_account_data(d, &token_account_address,
            //        &wallet.community_pubkey, &token_account_owner)?;
            Ok(None)
        },
        || {
            let transaction = client.create_transaction(
                &[
                    system_instruction::create_account_with_seed(
                        &wallet.payer_keypair.pubkey(),       // from
                        &neon_multisig,                       // to
                        &wallet.creator_pubkey,               // base
                        &format!("{}_multisig", REALM_NAME),  // seed
                        Rent::default().minimum_balance(355), // lamports
                        355,                                  // space
                        &spl_token::id(),                     // owner
                    ),
                    spl_token::instruction::initialize_multisig(
                        &spl_token::id(),
                        &neon_multisig,
                        &[
                            &main_governance.governance_address,
                            &emergency_governance.governance_address,
                        ],
                        1,
                    )
                    .unwrap(),
                ],
                &[wallet.get_creator_keypair()?],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // -------------------- Create accounts for token_owner --------------------
    let special_accounts = token_distribution.get_special_accounts();
    for (i, voter_weight) in token_distribution.voter_list.iter().enumerate() {
        let token_owner_record = realm.token_owner_record(&voter_weight.voter);
        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = cfg.account_by_seed(&seed, &spl_token::id());
        let lockup = Lockup::default();

        executor.check_and_create_object(
            &format!("{} <- {}", seed, voter_weight.voter),
            token_owner_record.get_data()?,
            |_| {
                // TODO check that all accounts needed to this owner created correctly
                let fixed_weight_record_address =
                    fixed_weight_addin.get_voter_weight_record_address(&realm, &voter_weight.voter);
                let vesting_weight_record_address =
                    vesting_addin.get_voter_weight_record_address(&voter_weight.voter, &realm);
                println!(
                    "VoterWeightRecords: fixed {}, vesting {}",
                    fixed_weight_record_address, vesting_weight_record_address
                );
                Ok(None)
            },
            || {
                let mut instructions = vec![
                    token_owner_record.create_token_owner_record_instruction(),
                    fixed_weight_addin
                        .setup_voter_weight_record_instruction(&realm, &voter_weight.voter),
                    system_instruction::transfer(
                        // Charge VestingAddin::VoterWeightRecord
                        &wallet.payer_keypair.pubkey(),
                        &vesting_addin.get_voter_weight_record_address(&voter_weight.voter, &realm),
                        Rent::default()
                            .minimum_balance(vesting_addin.get_voter_weight_account_size()),
                    ),
                ];
                if !special_accounts.contains(&voter_weight.voter) {
                    instructions.extend(vec![
                        system_instruction::create_account_with_seed(
                            &wallet.payer_keypair.pubkey(),       // from
                            &vesting_token_account,               // to
                            &wallet.creator_pubkey,               // base
                            &seed,                                // seed
                            Rent::default().minimum_balance(165), // lamports
                            165,                                  // space
                            &spl_token::id(),                     // owner
                        ),
                        spl_token::instruction::initialize_account(
                            &spl_token::id(),
                            &vesting_token_account,
                            &wallet.community_pubkey,
                            &vesting_addin.find_vesting_account(&vesting_token_account),
                        )
                        .unwrap(),
                        system_instruction::transfer(
                            // Charge VestingRecord
                            &wallet.payer_keypair.pubkey(),
                            &vesting_addin.find_vesting_account(&vesting_token_account),
                            Rent::default().minimum_balance(
                                vesting_addin
                                    .get_vesting_account_size(cfg.get_schedule_size(&lockup), true),
                            ),
                        ),
                    ]);
                    let transaction = client
                        .create_transaction(&instructions, &[wallet.get_creator_keypair()?])?;
                    Ok(Some(transaction))
                } else {
                    let transaction = client.create_transaction_with_payer_only(&instructions)?;
                    Ok(Some(transaction))
                }
            },
        )?;
    }

    // -------------------- Create extra token accounts ------------------------
    for (i, token_account) in token_distribution.extra_token_accounts().iter().enumerate() {
        let seed: String = format!("{}_account_{}", REALM_NAME, i);
        let token_account_address = cfg.account_by_seed(&seed, &spl_token::id());
        let token_account_owner = if token_account.lockup.is_locked() {
            vesting_addin.find_vesting_account(&token_account_address)
        } else {
            cfg.get_owner_address(&token_account.owner)?
        };
        println!(
            "Extra token account '{}' {} owned by {}",
            seed, token_account_address, token_account_owner
        );

        executor.check_and_create_object(
            &seed,
            get_account_data(client, &token_account_address)?,
            |d| {
                assert_is_valid_account_data(
                    d,
                    &token_account_address,
                    &wallet.community_pubkey,
                    &token_account_owner,
                )?;
                Ok(None)
            },
            || {
                let mut instructions = vec![
                    system_instruction::create_account_with_seed(
                        &wallet.payer_keypair.pubkey(),       // from
                        &token_account_address,               // to
                        &wallet.creator_pubkey,               // base
                        &seed,                                // seed
                        Rent::default().minimum_balance(165), // lamports
                        165,                                  // space
                        &spl_token::id(),                     // owner
                    ),
                    spl_token::instruction::initialize_account(
                        &spl_token::id(),
                        &token_account_address,
                        &wallet.community_pubkey,
                        &token_account_owner,
                    )
                    .unwrap(),
                ];
                if token_account.lockup.is_locked() {
                    instructions.extend(vec![system_instruction::transfer(
                        // Charge VestingRecord
                        &wallet.payer_keypair.pubkey(),
                        &vesting_addin.find_vesting_account(&token_account_address),
                        Rent::default().minimum_balance(vesting_addin.get_vesting_account_size(
                            cfg.get_schedule_size(&token_account.lockup),
                            true,
                        )),
                    )]);
                }
                let transaction =
                    client.create_transaction(&instructions, &[wallet.get_creator_keypair()?])?;
                Ok(Some(transaction))
            },
        )?;
    }

    // ----------- Fill creator_token_owner record ---------------
    let creator_token_owner = wallet.payer_keypair.pubkey();
    let creator_token_owner_record = realm.token_owner_record(&creator_token_owner);

    // ------------- Setup main governance ------------------------
    executor.check_and_create_object(
        "Main governance",
        main_governance.get_data()?,
        |_| Ok(None),
        || {
            let transaction = client.create_transaction(
                &[main_governance.create_governance_instruction(
                    &wallet.creator_pubkey,
                    &creator_token_owner_record,
                    cfg.community_governance_config.clone(),
                )],
                &[wallet.get_creator_keypair()?],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // ------------- Setup emergency governance ------------------------
    executor.check_and_create_object(
        "Emergency governance",
        emergency_governance.get_data()?,
        |_| Ok(None),
        || {
            let transaction = client.create_transaction(
                &[emergency_governance.create_governance_instruction(
                    &wallet.creator_pubkey,
                    &creator_token_owner_record,
                    cfg.emergency_governance_config.clone(),
                )],
                &[wallet.get_creator_keypair()?],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // ------------- Setup emergency governance ------------------------
    executor.check_and_create_object(
        "Emergency governance",
        emergency_governance.get_data()?,
        |_| Ok(None),
        || {
            let transaction = client.create_transaction(
                &[emergency_governance.create_governance_instruction(
                    &wallet.creator_pubkey,
                    &creator_token_owner_record,
                    cfg.emergency_governance_config.clone(),
                )],
                &[wallet.get_creator_keypair()?],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // ------------- Setup maintenance governance ------------------------
    executor.check_and_create_object(
        "Maintenance governance",
        maintenance_governance.get_data()?,
        |_| Ok(None),
        || {
            let transaction = client.create_transaction(
                &[maintenance_governance.create_governance_instruction(
                    &wallet.creator_pubkey,
                    &creator_token_owner_record,
                    cfg.maintenance_governance_config.clone(),
                )],
                &[wallet.get_creator_keypair()?],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // --------- Create NEON associated token account -------------
    let governance_token_account =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &main_governance.governance_address,
            &wallet.community_pubkey,
            &spl_token::id(),
        );
    println!(
        "Main governance address: {}",
        main_governance.governance_address
    );
    println!(
        "Main governance token account: {}",
        governance_token_account
    );

    executor.check_and_create_object(
        "NEON-token main-governance account",
        get_account_data(client, &governance_token_account)?,
        |d| {
            assert_is_valid_account_data(
                d,
                &governance_token_account,
                &wallet.community_pubkey,
                &main_governance.governance_address,
            )?;
            Ok(None)
        },
        || {
            let transaction = client.create_transaction_with_payer_only(&[
                spl_associated_token_account::instruction::create_associated_token_account(
                    &wallet.payer_keypair.pubkey(),
                    &main_governance.governance_address,
                    &wallet.community_pubkey,
                    &spl_token::id(),
                ),
            ])?;
            Ok(Some(transaction))
        },
    )?;

    // --------------- Pass token and programs to governance ------
    let mut collector =
        TransactionCollector::new(client, cfg.send_trx, cfg.verbose, "Pass under governance");
    // 1. Mint
    collector.check_and_create_object(
        "NEON-token mint-authority",
        get_mint_data(client, &wallet.community_pubkey)?,
        |d| {
            if d.mint_authority.contains(&wallet.creator_pubkey) {
                let instructions = [spl_token::instruction::set_authority(
                    &spl_token::id(),
                    &wallet.community_pubkey,
                    Some(&neon_multisig),
                    spl_token::instruction::AuthorityType::MintTokens,
                    &wallet.creator_pubkey,
                    &[],
                )
                .unwrap()]
                .to_vec();
                let signers = [wallet.get_creator_keypair()?].to_vec();
                Ok(Some((instructions, signers)))
            } else if d.mint_authority.contains(&neon_multisig) {
                Ok(None)
            } else {
                Err(
                    StateError::InvalidMintAuthority(wallet.community_pubkey, d.mint_authority)
                        .into(),
                )
            }
        },
        || {
            if cfg.send_trx {
                Err(StateError::MissingMint(wallet.community_pubkey).into())
            } else {
                Ok(None)
            }
        },
    )?;

    // 2. Realm
    collector.check_and_create_object(
        "Realm authority",
        realm.get_data()?,
        |d| {
            if d.authority == Some(wallet.creator_pubkey) {
                let instructions = [realm.set_realm_authority_instruction(
                    &wallet.creator_pubkey,
                    Some(&main_governance.governance_address),
                    SetRealmAuthorityAction::SetChecked,
                )]
                .to_vec();
                let signers = [wallet.get_creator_keypair()?].to_vec();
                Ok(Some((instructions, signers)))
            } else if d.authority == Some(main_governance.governance_address)
                || d.authority == Some(emergency_governance.governance_address)
            {
                Ok(None)
            } else {
                Err(StateError::InvalidRealmAuthority(realm.realm_address, d.authority).into())
            }
        },
        || {
            if cfg.send_trx {
                Err(StateError::MissingRealm(realm.realm_address).into())
            } else {
                Ok(None)
            }
        },
    )?;

    // 3. Programs...
    for (name, program) in [
        ("spl-governance", &wallet.governance_program_id),
        ("fixed-weight-addin", &wallet.fixed_weight_addin_id),
        ("vesting-addin", &wallet.vesting_addin_id),
    ] {
        collector.check_and_create_object(
            &format!("{} upgrade-authority", name),
            Some(client.get_program_upgrade_authority(program)?),
            |&upgrade_authority| {
                if upgrade_authority == Some(wallet.creator_pubkey) {
                    let instructions = [client.set_program_upgrade_authority_instruction(
                        program,
                        &wallet.creator_pubkey,
                        Some(&emergency_governance.governance_address),
                    )?]
                    .to_vec();
                    let signers = [wallet.get_creator_keypair()?].to_vec();
                    Ok(Some((instructions, signers)))
                } else if upgrade_authority == Some(emergency_governance.governance_address) {
                    Ok(None)
                } else {
                    Err(
                        StateError::InvalidProgramUpgradeAuthority(*program, upgrade_authority)
                            .into(),
                    )
                }
            },
            || unreachable!(),
        )?;
    }

    collector.execute_transaction()?;

    Ok(())
}
