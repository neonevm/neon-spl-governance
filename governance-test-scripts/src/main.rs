mod errors;
mod tokens;
mod wallet;
mod helpers;

use crate::{
    tokens::{get_mint_data, get_account_data, create_mint_instructions},
    errors::{StateError, ScriptError},
    wallet::Wallet,
    helpers::{
        TransactionExecutor,
        TransactionCollector,
        ProposalTransactionInserter,
    },
};
use solana_sdk::{
    pubkey::Pubkey,
    signer::{
        Signer,
        keypair::Keypair,
    },
    system_instruction,
    rent::Rent,
};

use spl_governance::{
    state::{
        enums::{
            MintMaxVoteWeightSource,
            VoteThresholdPercentage,
            VoteTipping,
            ProposalState,
        },
        governance::GovernanceConfig,
        realm::SetRealmAuthorityAction,
    },
};
use clap::{
    crate_description, crate_name, crate_version,
    App, AppSettings, Arg, SubCommand,
};

use spl_governance_addin_vesting::state::VestingSchedule;

use governance_lib::{
    client::Client,
    realm::{RealmConfig, Realm},
    proposal::Proposal,
    addin_fixed_weights::AddinFixedWeights,
    addin_vesting::AddinVesting,
};
use solana_sdk::pubkey;

const REALM_NAME: &str = "Test_Realm_9";

enum ExtraTokenAccountOwner {
    MainGovernance,
    EmergencyGovernance,
    Key(Pubkey),
}

struct ExtraTokenAccount {
    pub owner: ExtraTokenAccountOwner,
    pub amount: u64,
    pub name: &'static str,
}

const EXTRA_TOKEN_ACCOUNTS: [ExtraTokenAccount;3] = [
    ExtraTokenAccount {owner: ExtraTokenAccountOwner::MainGovernance, amount: 5_000_000, name: "Tresuary"},
    ExtraTokenAccount {owner: ExtraTokenAccountOwner::MainGovernance, amount: 5_000_000, name: "Some funds"},
    ExtraTokenAccount {owner: ExtraTokenAccountOwner::Key(pubkey!("rDeo4nZPE2aWpBkqFXBH8ygh1cD63nEKZPiDrpmQad6")), amount: 1_000_000, name: "IDO pool"},
];



fn process_environment(wallet: &Wallet, client: &Client, setup: bool, verbose: bool) -> Result<(), ScriptError> {

    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(&client, wallet.vesting_addin_id);
    let governance = realm.governance(&wallet.governed_account_pubkey);

    let executor = TransactionExecutor {client, setup, verbose};
    // ----------- Check or create community mint ----------------------
    executor.check_and_create_object("Mint", get_mint_data(client, &wallet.community_pubkey)?,
        |d| {
            if !d.mint_authority.contains(&wallet.creator_keypair.pubkey()) &&
                    !d.mint_authority.contains(&governance.governance_address) {
                return Err(StateError::InvalidMintAuthority(wallet.community_pubkey, d.mint_authority).into());
            }
            Ok(None)
        },
        || {
            let transaction = client.create_transaction(
                &create_mint_instructions(
                        &client,
                        &wallet.community_keypair.pubkey(),
                        &wallet.creator_keypair.pubkey(),
                        None,
                        6,
                    )?,
                &[&wallet.community_keypair],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // -------------- Check or create Realm ---------------------------
    executor.check_and_create_object("Realm", realm.get_data()?,
        |d| {
            if d.community_mint != realm.community_mint {
                return Err(StateError::InvalidRealmCommunityMint(realm.realm_address, d.community_mint).into());
            }
            if d.authority != Some(wallet.creator_keypair.pubkey()) &&
                    d.authority != Some(governance.governance_address) {
                return Err(StateError::InvalidRealmAuthority(realm.realm_address, d.authority).into());
            }
            Ok(None)
        },
        || {
            let transaction = client.create_transaction_with_payer_only(
                &[
                    realm.create_realm_instruction(
                        &wallet.creator_keypair.pubkey(),
                        Some(wallet.fixed_weight_addin_id),
                        Some(wallet.fixed_weight_addin_id),
                    ),
                ],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // ------------ Setup and configure max_voter_weight_record ----------------
    // TODO check max_voter_weight_record_address created correctly
    let max_voter_weight_record_address = fixed_weight_addin.setup_max_voter_weight_record(&realm).unwrap();
    realm.settings_mut().max_voter_weight_record_address = Some(max_voter_weight_record_address);

    // -------------------- Create accounts for token_owner --------------------
    let voter_list = fixed_weight_addin.get_voter_list()?;
    for (i, voter_weight) in voter_list.iter().enumerate() {
        let token_owner_record = realm.token_owner_record(&voter_weight.voter);
        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;

        executor.check_and_create_object(&seed, token_owner_record.get_data()?,
            |_| {
                // TODO check that all accounts needed to this owner created correctly
                Ok(None)
            },
            || {
                let transaction = client.create_transaction(
                    &[
                        token_owner_record.create_token_owner_record_instruction(),
                        fixed_weight_addin.setup_voter_weight_record_instruction(
                                &realm, &voter_weight.voter),
                        system_instruction::create_account_with_seed(
                            &wallet.payer_keypair.pubkey(),       // from
                            &vesting_token_account,               // to
                            &wallet.creator_keypair.pubkey(),     // base
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
                        ).unwrap(),
                    ],
                    &[&wallet.creator_keypair]
                )?;
                Ok(Some(transaction))
            }
        )?;
    }

    // -------------------- Create extra token accounts ------------------------
    for token_account in EXTRA_TOKEN_ACCOUNTS.iter() {
        let seed: String = format!("{}_account_{}", REALM_NAME, token_account.name);
        let token_account_address = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;
        let token_account_owner = match token_account.owner {
            ExtraTokenAccountOwner::MainGovernance => {governance.governance_address},
            ExtraTokenAccountOwner::EmergencyGovernance => {governance.governance_address},
            ExtraTokenAccountOwner::Key(pubkey) => {pubkey},
        };
        println!("Extra token account '{}' {}", token_account.name, token_account_address);

        executor.check_and_create_object(&seed, get_account_data(client, &token_account_address)?,
            |d| {
                if d.mint != wallet.community_pubkey {
                    return Err(StateError::InvalidTokenAccountMint(token_account_address, d.mint).into());
                }
                if d.owner != token_account_owner {
                    return Err(StateError::InvalidTokenAccountOwner(token_account_address, d.owner).into());
                }
                Ok(None)
            },
            || {
                let transaction = client.create_transaction(
                    &[
                        system_instruction::create_account_with_seed(
                            &wallet.payer_keypair.pubkey(),       // from
                            &token_account_address,               // to
                            &wallet.creator_keypair.pubkey(),     // base
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
                        ).unwrap(),
                    ],
                    &[&wallet.creator_keypair]
                )?;
                Ok(Some(transaction))
            }
        )?;
    }

    // ----------- Build creator_token_owner record ---------------
    let creator_token_owner: &Keypair = &wallet.creator_token_owner_keypair;
    let creator_token_owner_record = realm.token_owner_record(&creator_token_owner.pubkey());
    creator_token_owner_record.update_voter_weight_record_address()?;

    // TODO setup delegate through multisig
    executor.check_and_create_object("Delegate for creator_token_owner",
        creator_token_owner_record.get_data()?,
        |d| {
            if let Some(delegate) = d.governance_delegate {
                if delegate == wallet.creator_keypair.pubkey() {
                    return Ok(None);
                } else {
                    return Err(StateError::InvalidDelegate(creator_token_owner.pubkey(), Some(delegate)).into());
                }
            } else {
                let transaction = client.create_transaction(
                    &[
                        creator_token_owner_record.set_delegate_instruction(
                            &creator_token_owner.pubkey(),
                            &Some(wallet.creator_keypair.pubkey()),
                        ),
                    ],
                    &[creator_token_owner],
                )?;
                Ok(Some(transaction))
            }
        },
        || {return Err(StateError::MissingTokenOwnerRecord(creator_token_owner.pubkey()).into());}
    )?;

    // ------------- Setup main governance ------------------------
    let gov_config: GovernanceConfig =
        GovernanceConfig {
            vote_threshold_percentage: VoteThresholdPercentage::YesVote(2),
            min_community_weight_to_create_proposal: 10,
            min_transaction_hold_up_time: 0,
            max_voting_time: 78200,
            vote_tipping: VoteTipping::Strict,
            proposal_cool_off_time: 0,
            min_council_weight_to_create_proposal: 0,
        };

    executor.check_and_create_object("Governance", governance.get_data()?,
        |_| {Ok(None)},
        || {
            let transaction = client.create_transaction(
                &[
                    governance.create_governance_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &creator_token_owner_record,
                        gov_config
                    ),
                ],
                &[&wallet.creator_keypair]
            )?;
            Ok(Some(transaction))
        }
    )?;

    // --------------- Pass token and programs to governance ------
    let mut collector = TransactionCollector::new(client, setup, verbose, "Pass under governance");
    // 1. Mint
    collector.check_and_create_object("NEON-token mint-authority",
        get_mint_data(client, &wallet.community_pubkey)?,
        |d| {
            if d.mint_authority.contains(&wallet.creator_keypair.pubkey()) {
                let instructions = [
                        spl_token::instruction::set_authority(
                            &spl_token::id(),
                            &wallet.community_pubkey,
                            Some(&governance.governance_address),
                            spl_token::instruction::AuthorityType::MintTokens,
                            &wallet.creator_keypair.pubkey(),
                            &[],
                        ).unwrap()
                    ].to_vec();
                let signers = [&wallet.creator_keypair].to_vec();
                Ok(Some((instructions, signers,)))
            } else if d.mint_authority.contains(&governance.governance_address) {
                Ok(None)
            } else {
                Err(StateError::InvalidMintAuthority(wallet.community_pubkey, d.mint_authority).into())
            }
        },
        || {Err(StateError::MissingMint(wallet.community_pubkey).into())},
    )?;

    // 2. Realm
    collector.check_and_create_object("Realm authority", realm.get_data()?,
        |d| {
            if d.authority == Some(wallet.creator_keypair.pubkey()) {
                let instructions = [
                        realm.set_realm_authority_instruction(
                            &wallet.creator_keypair.pubkey(),
                            Some(&governance.governance_address),
                            SetRealmAuthorityAction::SetChecked,
                        )
                    ].to_vec();
                let signers = [&wallet.creator_keypair].to_vec();
                Ok(Some((instructions, signers,)))
            } else if d.authority == Some(governance.governance_address) {
                Ok(None)
            } else {
                Err(StateError::InvalidRealmAuthority(realm.realm_address, d.authority).into())
            }
        },
        || {Err(StateError::MissingRealm(realm.realm_address).into())}
    )?;

    // 3. Programs...
    for (name,program) in [
            ("spl-governance", &wallet.governance_program_id),
            ("fixed-weight-addin", &wallet.fixed_weight_addin_id),
            ("vesting-addin", &wallet.vesting_addin_id),
        ]
    {
        collector.check_and_create_object(&format!("{} upgrade-authority", name),
            Some(client.get_program_upgrade_authority(program)?),
            |&upgrade_authority| {
                if upgrade_authority == Some(wallet.creator_keypair.pubkey()) {
                    let instructions = [
                            client.set_program_upgrade_authority_instruction(
                                program,
                                &wallet.creator_keypair.pubkey(),
                                Some(&governance.governance_address),
                            )?
                        ].to_vec();
                    let signers = [&wallet.creator_keypair].to_vec();
                    Ok(Some((instructions, signers,)))
                } else if upgrade_authority == Some(governance.governance_address) {
                    Ok(None)
                } else {
                    Err(StateError::InvalidProgramUpgradeAuthority(*program, upgrade_authority).into())
                }
            },
            || {unreachable!()},
        )?;
    }

    collector.execute_transaction()?;

    Ok(())
}



// =========================================================================
// Create TGE proposal (Token Genesis Event)
// =========================================================================
fn setup_proposal_tge(wallet: &Wallet, client: &Client, proposal_index: Option<u32>, setup: bool, verbose: bool) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {client, setup, verbose};

    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    realm.update_max_voter_weight_record_address()?;

    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(&client, wallet.vesting_addin_id);
    let governance = realm.governance(&wallet.governed_account_pubkey);

    let creator_token_owner = realm.token_owner_record(&wallet.creator_token_owner_keypair.pubkey());
    creator_token_owner.update_voter_weight_record_address()?;

    let governance_proposal_count = governance.get_proposals_count();
    let proposal_number = proposal_index.unwrap_or(governance_proposal_count);
    if proposal_number > governance_proposal_count {return Err(StateError::InvalidProposalIndex.into());}
    println!("Use {} for proposal_index", proposal_number);

    let proposal: Proposal = governance.proposal(proposal_number);
    
    executor.check_and_create_object("Proposal TGE", proposal.get_data()?,
        |_| {Ok(None)},
        || {
            let transaction = client.create_transaction(
                &[
                    proposal.create_proposal_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &creator_token_owner,
                        &format!("{} {}", "Token Genesis Event", proposal_number),
                        "Proposal for Token Genesis Event (mint tokens and distribute it)",
                    ),
                ],
                &[&wallet.creator_keypair],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // let result = client.add_signatory(&realm, &governance, &proposal, &token_owner);
    // println!("Add signatory {:?}", result);
    
    let mut transaction_inserter = ProposalTransactionInserter {
        proposal: &proposal,
        creator_keypair: &wallet.creator_keypair,
        creator_token_owner: &creator_token_owner,
        hold_up_time: 0,
        setup: setup,
        verbose: verbose,
        proposal_transaction_index: 0,
    };

    let governance_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &governance.governance_address, &wallet.community_pubkey, &spl_token::id());
    println!("Governance address: {}", governance.governance_address);
    println!("Governance token account: {}", governance_token_account);

    if !client.account_exists(&governance_token_account) {
        transaction_inserter.insert_transaction_checked(
                "Create associated NEON-token for governance",
                vec![
                    spl_associated_token_account::create_associated_token_account(
                        &wallet.payer_keypair.pubkey(),
                        &governance.governance_address,
                        &wallet.community_pubkey,
                    ).into(),
                ],
            )?;
    }

    let voter_list = fixed_weight_addin.get_voter_list()?;
    let total_amount = voter_list.iter().map(|v| v.weight).sum::<u64>() +
                       EXTRA_TOKEN_ACCOUNTS.iter().map(|v| v.amount).sum::<u64>();

    transaction_inserter.insert_transaction_checked(
            "Mint tokens",
            vec![
                spl_token::instruction::mint_to(
                    &spl_token::id(),
                    &wallet.community_pubkey,
                    &governance_token_account,
                    &governance.governance_address, &[],
                    total_amount,
                )?.into(),
            ],
        )?;

    for (i, voter) in voter_list.iter().enumerate() {
        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id()).unwrap();
        // TODO Calculate schedule
        let schedule = vec!(VestingSchedule { release_time: 0, amount: voter.weight });

        transaction_inserter.insert_transaction_checked(
                &format!("Deposit {} to {} on token account {}",
                        voter.weight, voter.voter, vesting_token_account),
                vec![
                    vesting_addin.deposit_with_realm_instruction(
                        &governance.governance_address,          // source_token_authority
                        &governance_token_account,    // source_token_account
                        &voter.voter,                 // vesting_owner
                        &vesting_token_account,       // vesting_token_account
                        schedule,                     // schedule
                        &realm,                       // realm
                    )?.into(),
                ],
            )?;
    }

    for token_account in EXTRA_TOKEN_ACCOUNTS.iter() {
        let seed: String = format!("{}_account_{}", REALM_NAME, token_account.name);
        let token_account_address = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;

        transaction_inserter.insert_transaction_checked(
                &format!("Transfer {} to {} ({})", token_account.amount, token_account_address, token_account.name),
                vec![
                    spl_token::instruction::transfer(
                        &spl_token::id(),
                        &governance_token_account,
                        &token_account_address,
                        &governance.governance_address, &[],
                        token_account.amount,
                    )?.into(),
                ],
            )?;
    };

    transaction_inserter.insert_transaction_checked(
            "Change to Vesting addin",
            vec![
                realm.set_realm_config_instruction(
                    &governance.governance_address,       // we already passed realm under governance
                    &RealmConfig {
                        council_token_mint: None,
                        community_voter_weight_addin: Some(wallet.vesting_addin_id),
                        max_community_voter_weight_addin: None,
                        min_community_weight_to_create_governance: 1,            // TODO Verify parameters!
                        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                    }
                ).into(),
            ],
        )?;

    transaction_inserter.insert_transaction_checked(
            "Change Governance config",
            vec![
                governance.set_governance_config_instruction(
                    GovernanceConfig {
                        vote_threshold_percentage: VoteThresholdPercentage::YesVote(2),
                        min_community_weight_to_create_proposal: 3*1000_000,
                        min_transaction_hold_up_time: 0,
                        max_voting_time: 1*60, // 3*24*3600,
                        vote_tipping: VoteTipping::Disabled,
                        proposal_cool_off_time: 0,                 // not implemented in the current version
                        min_council_weight_to_create_proposal: 0,  // council token does not used
                    },
                ).into(),
            ],
        )?;

    Ok(())
}

fn finalize_vote_proposal(wallet: &Wallet, client: &Client, proposal_index: Option<u32>, verbose: bool) -> Result<(), ScriptError> {
    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    realm.update_max_voter_weight_record_address()?;
    let governance = realm.governance(&wallet.governed_account_pubkey);

    let creator_token_owner = realm.token_owner_record(&wallet.creator_token_owner_keypair.pubkey());
    creator_token_owner.update_voter_weight_record_address()?;

    let governance_proposal_count = governance.get_proposals_count();
    let proposal_number = proposal_index.unwrap_or(governance_proposal_count);
    if proposal_number > governance_proposal_count {return Err(StateError::InvalidProposalIndex.into());}
    println!("Use {} for proposal_index", proposal_number);

    let proposal: Proposal = governance.proposal(proposal_number);
    if let None = proposal.get_data()? {
        return Err(StateError::InvalidProposalIndex.into());
    }

    proposal.finalize_vote(&creator_token_owner)?;

    Ok(())
}

fn sign_off_proposal(wallet: &Wallet, client: &Client, proposal_index: Option<u32>, verbose: bool) -> Result<(), ScriptError> {
    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    realm.update_max_voter_weight_record_address()?;
    let governance = realm.governance(&wallet.governed_account_pubkey);

    let creator_token_owner = realm.token_owner_record(&wallet.creator_token_owner_keypair.pubkey());
    creator_token_owner.update_voter_weight_record_address()?;

    let governance_proposal_count = governance.get_proposals_count();
    let proposal_number = proposal_index.unwrap_or(governance_proposal_count);
    if proposal_number > governance_proposal_count {return Err(StateError::InvalidProposalIndex.into());}
    println!("Use {} for proposal_index", proposal_number);

    let proposal: Proposal = governance.proposal(proposal_number);
    if let None = proposal.get_data()? {
        return Err(StateError::InvalidProposalIndex.into());
    }

    if proposal.get_state()? == ProposalState::Draft {
        proposal.sign_off_proposal(&wallet.creator_keypair, &creator_token_owner)?;
    }

    Ok(())
}

fn approve_proposal(wallet: &Wallet, client: &Client, proposal_index: Option<u32>, verbose: bool) -> Result<(), ScriptError> {
    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    realm.update_max_voter_weight_record_address()?;

    let creator_token_owner = realm.token_owner_record(&wallet.creator_token_owner_keypair.pubkey());
    creator_token_owner.update_voter_weight_record_address()?;

    let governance = realm.governance(&wallet.governed_account_pubkey);
    let governance_proposal_count = governance.get_proposals_count();
    let proposal_number = proposal_index.unwrap_or(governance_proposal_count);
    if proposal_number > governance_proposal_count {return Err(StateError::InvalidProposalIndex.into());}
    println!("Use {} for proposal_index", proposal_number);

    let proposal: Proposal = governance.proposal(proposal_number);
    if let None = proposal.get_data()? {
        return Err(StateError::InvalidProposalIndex.into());
    }

    for voter in wallet.voter_keypairs.iter() {
        let token_owner = realm.token_owner_record(&voter.pubkey());
        if let Some(_) = token_owner.get_data()? {
            token_owner.update_voter_weight_record_address()?;

            let signature = proposal.cast_vote(&creator_token_owner, voter, &token_owner, true)?;
            println!("CastVote {} {:?}", voter.pubkey(), signature);
        }
    }

    Ok(())
}

fn execute_proposal(wallet: &Wallet, client: &Client, proposal_index: Option<u32>, verbose: bool) -> Result<(), ScriptError> {
    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    let governance = realm.governance(&wallet.governed_account_pubkey);

    let governance_proposal_count = governance.get_proposals_count();
    let proposal_number = proposal_index.unwrap_or(governance_proposal_count);
    if proposal_number > governance_proposal_count {return Err(StateError::InvalidProposalIndex.into());}
    println!("Use {} for proposal_index", proposal_number);

    let proposal: Proposal = governance.proposal(proposal_number);
    if let None = proposal.get_data()? {
        return Err(StateError::InvalidProposalIndex.into());
    }

    let result = proposal.execute_transactions(0)?;
    println!("Execute transactions from proposal option 0: {:?}", result);

    Ok(())
}

fn main() {
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .takes_value(false)
                .global(true)
                .help("Show additional information")
        )
        .arg(
            Arg::with_name("send_trx")
                .long("send-trx")
                .takes_value(false)
                .help("Send transactions to blockchain")
        )
        .subcommand(SubCommand::with_name("environment")
            .about("Prepare environment for launching")
        )
        .subcommand(SubCommand::with_name("proposal")
            .about("Prepare and execute proposal")
            .arg(
                Arg::with_name("index")
                    .long("index")
                    .short("i")
                    .takes_value(true)
                    .value_name("PROPOSAL_INDEX")
                    .help("Proposal index")
            )
            .subcommand(SubCommand::with_name("create-tge")
                .about("Create Token Genesis Event proposal")
            )
            .subcommand(SubCommand::with_name("sign-off")
                .about("Sign Off proposal")
            )
            .subcommand(SubCommand::with_name("approve")
                .about("Approve proposal")
            )
            .subcommand(SubCommand::with_name("finalize-vote")
                .about("Finalize vote for proposal")
            )
            .subcommand(SubCommand::with_name("execute")
                .about("Execute proposal (after approve)")
            )
        ).get_matches();

    let wallet = Wallet::new().unwrap();
    wallet.display();

    let client = Client::new("http://localhost:8899", &wallet.payer_keypair);

    let send_trx: bool = matches.is_present("send_trx");
    let verbose: bool = matches.is_present("verbose");
    match matches.subcommand() {
        ("environment", Some(arg_matches)) => {
            process_environment(&wallet, &client, send_trx, verbose).unwrap()
        },
        ("proposal", Some(arg_matches)) => {
            let proposal_index = arg_matches.value_of("index").map(|v| v.parse::<u32>().unwrap());
            match arg_matches.subcommand() {
                ("create-tge", Some(arg_matches)) => {
                    setup_proposal_tge(&wallet, &client, proposal_index, send_trx, verbose).unwrap()
                },
                ("sign-off", Some(arg_matches)) => {
                    sign_off_proposal(&wallet, &client, proposal_index, verbose).unwrap()
                },
                ("approve", Some(arg_matches)) => {
                    approve_proposal(&wallet, &client, proposal_index, verbose).unwrap()
                },
                ("finalize-vote", Some(arg_matches)) => {
                    finalize_vote_proposal(&wallet, &client, proposal_index, verbose).unwrap()
                },
                ("execute", Some(arg_matches)) => {
                    execute_proposal(&wallet, &client, proposal_index, verbose).unwrap()
                },
                _ => unreachable!(),
            }
        },
        _ => unreachable!(),
    }
}
