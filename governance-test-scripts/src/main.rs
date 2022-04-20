mod errors;
mod tokens;
mod wallet;
mod token_distribution;

use crate::{
    tokens::{get_mint_data, get_account_data, create_mint},
    errors::{StateError, ScriptError},
    wallet::Wallet,
};
use colored::*;
use solana_sdk::{
    pubkey::Pubkey,
    signer::{
        Signer,
        keypair::{Keypair, read_keypair_file},
    },
    transaction::Transaction,
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
        governance::{
            GovernanceConfig,
            GovernanceV2,
        },
        realm::SetRealmAuthorityAction,
    },
};
use clap::{
    crate_description, crate_name, crate_version, value_t, 
    App, AppSettings, Arg, SubCommand,
};

//use spl_governance_addin_fixed_weights::{
//    instruction::{
//        get_max_voter_weight_address,
//        get_voter_weight_address,
//    }
//};

use spl_governance_addin_vesting::state::VestingSchedule;

// mod tokens;

use governance_lib::{
    client::{Client, ClientResult},
    realm::{RealmConfig, Realm},
    governance::Governance,
    proposal::Proposal,
    token_owner::TokenOwner,
    addin_fixed_weights::AddinFixedWeights,
    addin_vesting::AddinVesting,
};
use question::{Answer, Question};
use solana_sdk::pubkey;

use crate::token_distribution::DISTRIBUTION_LIST;
const ADDITIONAL_SUPPLY: u64 = 10_000_000;

// const REALM_NAME: &str = "Test Realm";
const REALM_NAME: &str = "Test_Realm_9";
// const REALM_NAME: &str = "Test Realm 6";
//const PROPOSAL_NAME: &str = "Token Genesis Event";
//const PROPOSAL_DESCRIPTION: &str = "proposal_description";

macro_rules! println_item {
    ($format:literal, $($item:expr),*) => {
        println!(concat!("\x1b[34m", $format, "\x1b[0m"), $($item),*);
    }
}

macro_rules! println_error {
    ($format:literal, $($item:expr),*) => {
        println!(concat!("\x1b[31m", $format, "\x1b[0m"), $($item),*);
    }
}

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

const extra_token_accounts: [ExtraTokenAccount;3] = [
    ExtraTokenAccount {owner: ExtraTokenAccountOwner::MainGovernance, amount: 5_000_000, name: "Tresuary"},
    ExtraTokenAccount {owner: ExtraTokenAccountOwner::MainGovernance, amount: 5_000_000, name: "Some funds"},
    ExtraTokenAccount {owner: ExtraTokenAccountOwner::Key(pubkey!("rDeo4nZPE2aWpBkqFXBH8ygh1cD63nEKZPiDrpmQad6")), amount: 1_000_000, name: "IDO pool"},
];

fn process_environment(wallet: &Wallet, client: &Client, setup: bool) -> Result<(), ScriptError> {

    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(&client, wallet.vesting_addin_id);
    let governance = realm.governance(&wallet.governed_account_pubkey);

    // ------------ Check programs authority ----------------------
    // TODO: check programs authority!

    // ----------- Check or create community mint ----------------------
    let mint_data = get_mint_data(client, &wallet.community_pubkey)?;
    if let Some(mint_info) = mint_data {
        println_item!("Mint: {:?}", mint_info);
        if mint_info.mint_authority.contains(&wallet.creator_keypair.pubkey()) {
            // All ok: mint exist and creator can mint
            println!("Mint {} exists with mint-authority belongs to creator {}",
                    &wallet.community_pubkey, &wallet.creator_keypair.pubkey());

        } else if mint_info.mint_authority.contains(&governance.governance_address) {
            // All ok: mint exist but governance can mint
            // It seems like environment was already setup
            println!("Mint {} exists with mint-authority belongs to governance {}",
                    &wallet.community_pubkey, &governance.governance_address);
        } else {
            // Error: mint authority doesn't belong to creator or governance
            let error = StateError::InvalidMintAuthority(wallet.community_pubkey, mint_info.mint_authority);
            if setup {return Err(error.into())} else {println_error!("{:?}", error);};
        }

        if mint_info.freeze_authority.is_some() {
            // Error: freeze authority should be None
            let error = StateError::InvalidFreezeAuthority(wallet.community_pubkey, mint_info.freeze_authority);
            if setup {return Err(error.into())} else {println_error!("{:?}", error);};
        }
    } else {
        if setup {
            let signature = create_mint(
                    &client,
                    &wallet.community_keypair,
                    &wallet.creator_keypair.pubkey(),
                    None,
                    6,
                )?;
            println!("Mint {} created in trx: {}", &wallet.community_pubkey, signature);
        } else {
            println!("Mint {} missed", &wallet.community_pubkey);
        }
    }

    // -------------- Check or create Realm ---------------------------
    if let Some(realm_data) = realm.get_data()? {
        println_item!("Realm: {:?}", realm_data);
        if realm_data.community_mint != realm.community_mint {
            let error = StateError::InvalidRealmCommunityMint(realm.realm_address, realm_data.community_mint);
            if setup {return Err(error.into())} else {println_error!("{:?}", error);};
        }
        if realm_data.authority == Some(wallet.creator_keypair.pubkey()) {
            println!("Realm {} exists with authority belongs to creator {}",
                    realm.realm_address, &wallet.creator_keypair.pubkey());
        } else if realm_data.authority == Some(governance.governance_address) {
            println!("Realm {} exists with authority belongs to governance {}",
                    realm.realm_address, &governance.governance_address);
        } else {
            let error = StateError::InvalidRealmAuthority(realm.realm_address, realm_data.authority);
            if setup {return Err(error.into())} else {println_error!("{:?}", error);};
        }
        // TODO Check realm config structure (for right addin)!
    } else {
        if setup {
            let signature = realm.create_realm(
                &wallet.creator_keypair,
                Some(wallet.fixed_weight_addin_id),
                Some(wallet.fixed_weight_addin_id),
            )?;
            println!("Realm {} created in trx: {}", realm.realm_address, signature);
        } else {
            println!("Realm {} missed", &realm.realm_address);
        }
    }

    // ------------ Setup and configure max_voter_weight_record ----------------
    // TODO check max_voter_weight_record_address created correctly
    let max_voter_weight_record_address = fixed_weight_addin.setup_max_voter_weight_record(&realm).unwrap();
    realm.settings_mut().max_voter_weight_record_address = Some(max_voter_weight_record_address);

    // -------------------- Create accounts for token_owner --------------------
    let voter_list = fixed_weight_addin.get_voter_list()?;
    let total_voter_weight = voter_list.iter().map(|item| item.weight).sum::<u64>();
    for (i, voter_weight) in voter_list.iter().enumerate() {
        let token_owner_record = realm.token_owner_record(&voter_weight.voter);
        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;

        if let Some(token_owner_record) = token_owner_record.get_data()? {
            // TODO check that all accounts needed to this owner created correctly
            println!("token_owner_record {} exists", voter_weight.voter);
        } else {
            if setup {
                let signature = client.send_and_confirm_transaction(
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
                println!("token_owner_record {} created in trx: {}", voter_weight.voter, signature);
            } else {
                println!("Missing token_owner_record for {}", voter_weight.voter);
            }
        }
    }

    // -------------------- Create accounts for token_owner --------------------
    for token_account in extra_token_accounts.iter() {
        let seed: String = format!("{}_account_{}", REALM_NAME, token_account.name);
        let token_account_address = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;
        let token_account_owner = match token_account.owner {
            ExtraTokenAccountOwner::MainGovernance => {governance.governance_address},
            ExtraTokenAccountOwner::EmergencyGovernance => {governance.governance_address},
            ExtraTokenAccountOwner::Key(pubkey) => {pubkey},
        };
        println!("Extra token account '{}' {}", token_account.name, token_account_address);

        if let Some(token_account_data) = get_account_data(client, &token_account_address)? {
            if token_account_data.mint != wallet.community_pubkey {
                let error = StateError::InvalidTokenAccountMint(token_account_address, token_account_data.mint);
                if setup {return Err(error.into())} else {println_error!("{:?}", error);};
            }
            if token_account_data.owner != token_account_owner {
                let error = StateError::InvalidTokenAccountOwner(token_account_address, token_account_data.owner);
                if setup {return Err(error.into())} else {println_error!("{:?}", error);};
            }
        } else if setup {
            let signature = client.send_and_confirm_transaction(
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
            println!("Extra token account '{}' {} created in trx: {}", token_account.name, token_account_address, signature);
        } else {
            println!("Extra token account '{}' {} missed", token_account.name, token_account_address);
        }
    }

    // ----------- Build creator_token_owner record ---------------
    // TODO it is correct for fixed_weight_addin!
    let mut creator_token_owner = realm.token_owner_record(&voter_list[0].voter);
    //let creator_voter_weight_record = fixed_weight_addin.get_voter_weight_record_address(&realm, &voter_list[0].voter);
    //creator_token_owner.set_voter_weight_record_address(Some(creator_voter_weight_record));

/*
    // TODO setup delegate through multisig
    let creator_token_owner_data = creator_token_owner.get_data()?;
    println_item!("Creator token_owner_record: {:?}", creator_token_owner_data);
    if let Some(creator_token_owner_data) = creator_token_owner_data {
        if let Some(delegate) = creator_token_owner_data.governance_delegate {
            if delegate == wallet.creator_keypair.pubkey() {
                println!("Creator token_owner has correct delegate {}", delegate);
            } else {
                let error = StateError::InvalidDelegate(voter_list[0].voter, Some(delegate));
                if setup {return Err(error.into())} else {println_error!("{:?}", error);};
            }
        } else {
            if setup {
                let signature = creator_token_owner.set_delegate(&wallet.voter_keypairs[0], &Some(wallet.creator_keypair.pubkey()))?;
                println!("Creator token_owner delegate set in trx: {}", signature);
            } else {
                println!("Creator token_owner delegate doesn't installed");
            }
        }
    } else {
        let error = StateError::MissingTokenOwnerRecord(voter_list[0].voter);
        if setup {return Err(error.into())} else {println_error!("{:?}", error);};
    }*/

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

    if let Some(governance_data) = governance.get_data()? {
        // TODO check governance config
        println_item!("Governance: {:?}", governance_data);
        println!("Governance {} exists", governance.governance_address);
    } else {
        if setup {
            let signature = client.send_and_confirm_transaction(
                &[
                    governance.create_governance_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &creator_token_owner,
                        gov_config
                    ),
                ],
                &[&wallet.creator_keypair]
            )?;
            println!("Governance {} created in trx: {}", governance.governance_address, signature);
        } else {
            println!("Missing governance {}", governance.governance_address);
        }
    }

    // --------------- Pass token and programs to governance ------
    // 1. Mint
    let mut instructions = vec!();
    let mint_data = get_mint_data(client, &wallet.community_pubkey)?;
    if let Some(mint_data) = mint_data {
        if mint_data.mint_authority.contains(&wallet.creator_keypair.pubkey()) {
            instructions.push(
                    spl_token::instruction::set_authority(
                        &spl_token::id(),
                        &wallet.community_pubkey,
                        Some(&governance.governance_address),
                        spl_token::instruction::AuthorityType::MintTokens,
                        &wallet.creator_keypair.pubkey(),
                        &[],
                    ).unwrap()
                );
        } else if mint_data.mint_authority.contains(&governance.governance_address) {
            // Ok, mint already under governance authority
        } else {
            let error = StateError::InvalidMintAuthority(wallet.community_pubkey, mint_data.mint_authority);
            if setup {return Err(error.into())} else {println_error!("{:?}", error);}
        }
    } else {
        let error = StateError::MissingMint(wallet.community_pubkey);
        if setup {return Err(error.into())} else {println_error!("{:?}", error);}
    }

    // 2. Realm
    let realm_data = realm.get_data()?;
    if let Some(realm_data) = realm_data {
        if realm_data.authority == Some(wallet.creator_keypair.pubkey()) {
            instructions.push(
                    realm.set_realm_authority_instruction(
                        &wallet.creator_keypair.pubkey(),
                        Some(&governance.governance_address),
                        SetRealmAuthorityAction::SetChecked,
                    )
                );
        } else if realm_data.authority == Some(governance.governance_address) {
            // Ok, realm already under governance authority
        } else {
            let error = StateError::InvalidRealmAuthority(realm.realm_address, realm_data.authority);
            if setup {return Err(error.into())} else {println_error!("{:?}", error);}
        }
    } else {
        let error = StateError::MissingRealm(realm.realm_address);
        if setup {return Err(error.into())} else {println_error!("{:?}", error);}
    }

    // 3. Programs...
    for program in [&wallet.governance_program_id, &wallet.fixed_weight_addin_id, &wallet.vesting_addin_id,] {
        let upgrade_authority = client.get_program_upgrade_authority(program)?;
        if upgrade_authority == Some(wallet.creator_keypair.pubkey()) {
            println!("Program upgrade-authority for {} will be changed to {}", program, governance.governance_address);
            instructions.push(
                    client.set_program_upgrade_authority_instruction(
                        program,
                        &wallet.creator_keypair.pubkey(),
                        Some(&governance.governance_address),
                    )?
                );
        } else if upgrade_authority == Some(governance.governance_address) {
            // Ok, program already under governance authority
            println!("Program upgrade-authority for {} belongs to governance {}", program, governance.governance_address);
        } else {
            let error = StateError::InvalidProgramUpgradeAuthority(*program, upgrade_authority);
            if setup {return Err(error.into())} else {println_error!("{:?}", error);}
        }
    }
    if setup && !instructions.is_empty() {
        client.send_and_confirm_transaction(
                &instructions,
                &[&wallet.creator_keypair],
            ).unwrap();
    }

    Ok(())
}

fn _main() {
    //Question::new("Do you want to continue").show_defaults().confirm();
    //return;

    let wallet = Wallet::new().unwrap();
    wallet.display();

    let client = Client::new("http://localhost:8899", &wallet.payer_keypair);

    // let client = Client::new("https://api.devnet.solana.com", program_id, voter_weight_addin_pubkey);

//    let fixed_weight_addin = AddinFixedWeights::new(&client, voter_weight_addin_pubkey);
//    let voter_list = fixed_weight_addin.get_voter_list().unwrap();
//    println!("Fixed weight addin: voter list {:?}", voter_list);
//    println!("Total weight: {}", voter_list.iter().map(|item| item.weight).sum::<u64>());
//    return;

    let mint = get_mint_data(&client, &wallet.community_pubkey).unwrap(); //client.get_account_data_pack::<spl_token::state::Mint>(&spl_token::id(), &wallet.community_pubkey).unwrap();
    if let Some(_mint) = mint {
//        if !mint.mint_authority.contains(&payer_keypair.pubkey()) {
//            panic!("Invalid mint authority: actual {:?}, expected {}", mint.mint_authority, creator_keypair.pubkey());
//        }
    } else {
        let result = create_mint(
                &client,
                &wallet.community_keypair,
                &wallet.creator_keypair.pubkey(),
                None,
                6,
            ).unwrap();
        println!("Created community mint: {}", result);
    }

    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    if let Some(realm_data) = realm.get_data().unwrap() {
        if realm_data.community_mint != realm.community_mint {
            panic!("Invalid Realm community mint: expected {}, actual {}",
                    realm.community_mint, realm_data.community_mint);
        }
    } else {
        realm.create_realm(
                &wallet.creator_keypair,
                Some(wallet.fixed_weight_addin_id),
                Some(wallet.fixed_weight_addin_id),
            ).unwrap();
    }
    println!("{:?}", realm);
    println!("Realm Pubkey: {}", realm.realm_address);

    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let result = fixed_weight_addin.setup_max_voter_weight_record(&realm);
    println!("VoterWeightAddin.setup_max_voter_weight_record = {:?}", result);
    realm.settings_mut().max_voter_weight_record_address = Some(result.unwrap());

    let vesting_addin = AddinVesting::new(&client, wallet.vesting_addin_id);

    //let mut creator_token_owner: TokenOwner = realm.create_token_owner_record(&creator_keypair.pubkey()).unwrap();
    //let creator_voter_weight = fixed_weight_addin.setup_voter_weight_record(&realm, &creator_keypair.pubkey()).unwrap();
    //creator_token_owner.set_voter_weight_record_address(Some(creator_voter_weight));
    //creator_token_owner.set_voter_weight_record_address(Some(creator_keypair.pubkey()));

    let mut token_owners = vec!();
    for (i, keypair) in wallet.voter_keypairs.iter().enumerate() {
        let mut token_owner: TokenOwner = realm.token_owner_record(&keypair.pubkey());
        if let None = token_owner.get_data().unwrap() {
            token_owner.create_token_owner_record().unwrap();
        }
        let voter_weight_record = fixed_weight_addin.setup_voter_weight_record(&realm, &keypair.pubkey()).unwrap();
        token_owner.set_voter_weight_record_address(Some(voter_weight_record));
        println!("Token Owner {} \n{:?}, voter_weight_record: {}", i, token_owner, voter_weight_record);
        token_owners.push(token_owner);
    }

    let result = token_owners[0].set_delegate(&wallet.voter_keypairs[0], &Some(wallet.creator_keypair.pubkey())).unwrap();
    println!("Set delegate for voter[0]: {:?}", result);

    let gov_config: GovernanceConfig =
        GovernanceConfig {
            vote_threshold_percentage: VoteThresholdPercentage::YesVote(60),
            min_community_weight_to_create_proposal: 10,
            min_transaction_hold_up_time: 0,
            max_voting_time: 78200,
            vote_tipping: VoteTipping::Strict,
            proposal_cool_off_time: 0,
            min_council_weight_to_create_proposal: 0,
        };

    let governance = realm.governance(&wallet.governed_account_pubkey);
    if let None = governance.get_data().unwrap() {
        governance.create_governance(
                &wallet.creator_keypair,
                &token_owners[0],
                gov_config,
            ).unwrap();
    }
    println!("{}", governance);
    println!("{:?}", governance);

    // STEP 2: Pass Token and Realm under governance
    // transaction if already correct authority)
    let mut instructions = vec!();
    let mint = client.get_account_data_pack::<spl_token::state::Mint>(&spl_token::id(), &wallet.community_pubkey).unwrap().unwrap();
    if mint.mint_authority.contains(&wallet.creator_keypair.pubkey()) {
        instructions.push(
                spl_token::instruction::set_authority(
                    &spl_token::id(),
                    &wallet.community_pubkey,
                    Some(&governance.governance_address),
                    spl_token::instruction::AuthorityType::MintTokens,
                    &wallet.creator_keypair.pubkey(),
                    &[],
                ).unwrap()
            );
    }
    let realm_data = realm.get_data().unwrap().unwrap(); //client.get_account_data_borsh::<spl_governance::state::realm::RealmV2>(&program_id, &realm.realm_address).unwrap().unwrap();
    if realm_data.authority == Some(wallet.creator_keypair.pubkey()) {
        instructions.push(
                realm.set_realm_authority_instruction(
                    &wallet.creator_keypair.pubkey(),
                    Some(&governance.governance_address),
                    SetRealmAuthorityAction::SetChecked,
                )
            );
    }
    if !instructions.is_empty() {
        client.send_and_confirm_transaction(
                &instructions,
                &[&wallet.creator_keypair],
            ).unwrap();
    }

    // =========================================================================
    // Create TGE proposal (Token Genesis Event)
    // =========================================================================

    let proposal_number = governance.get_proposals_count();
    let proposal: Proposal = governance.proposal(proposal_number);
    proposal.create_proposal(
            &wallet.creator_keypair,
            &token_owners[0],
            &format!("{} {}", "Token Genesis Event", proposal_number),
            "Proposal for Token Genesis Event (mint tokens and distribute it)",
        ).unwrap();
    println!("{:?}", proposal);

    // let result = client.add_signatory(&realm, &governance, &proposal, &token_owner);
    // println!("Add signatory {:?}", result);
    
    let governance_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
                        &governance.governance_address,
                        &wallet.community_pubkey,
                        &spl_token::id(),
                    );
    println!("Governance address: {}", governance.governance_address);
    println!("Governance token account: {}", governance_token_account);
    if !client.account_exists(&governance_token_account) {
        let signature = client.send_and_confirm_transaction_with_payer_only(
                &[
                    spl_associated_token_account::create_associated_token_account(
                        &wallet.payer_keypair.pubkey(),
                        &governance.governance_address,
                        &wallet.community_pubkey,
                    ),
                ],
            ).unwrap();
        println!("Create associated token account {}", signature);
    }

    let total_amount = DISTRIBUTION_LIST.iter().map(|(_, amount)| amount).sum::<u64>() + ADDITIONAL_SUPPLY;
    proposal.insert_transaction(
            &wallet.creator_keypair,
            &token_owners[0],
            0, 0, 0,
            vec![
                spl_token::instruction::mint_to(
                    &spl_token::id(),
                    &wallet.community_pubkey,
                    &governance_token_account,
                    &governance.governance_address, &[],
                    total_amount,
                ).unwrap().into(),
            ],
        ).unwrap();

    for (i, (owner, amount)) in DISTRIBUTION_LIST.iter().enumerate() {
        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id()).unwrap();
        // TODO Calculate schedule
        let schedule = vec!(VestingSchedule { release_time: 0, amount: *amount });
        println!("{}, Voter {}, amount {}, token_account {}", i, owner, amount, vesting_token_account);

        let mut instructions = vec!();
        if !client.account_exists(&vesting_token_account) {
            instructions.extend([
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
            ]);
        }
        instructions.push(
                proposal.insert_transaction_instruction(
                    &wallet.creator_keypair.pubkey(),
                    &token_owners[0],
                    0, (i+1).try_into().unwrap(), 0,
                    vec![
                        vesting_addin.deposit_with_realm_instruction(
                            &governance.governance_address,          // source_token_authority
                            &governance_token_account,    // source_token_account
                            &owner,                       // vesting_owner
                            &vesting_token_account,       // vesting_token_account
                            schedule,                     // schedule
                            &realm,                       // realm
                        ).unwrap().into(),
                    ],
                ),
            );
            
        let result = client.send_and_confirm_transaction(&instructions, &[&wallet.creator_keypair]).unwrap();
        println!("   created: {}", result);
    }

    // Change to other VoterWeight addin
    proposal.insert_transaction(
        &wallet.creator_keypair,
        &token_owners[0],
        0, (DISTRIBUTION_LIST.len()+1).try_into().unwrap(), 0,
        vec![
            realm.set_realm_config_instruction(
                &governance.governance_address,       // we already passed realm under governance
                &RealmConfig {
                    council_token_mint: None,
                    community_voter_weight_addin: Some(wallet.vesting_addin_id),
                    max_community_voter_weight_addin: None,
                    min_community_weight_to_create_governance: 1,
                    community_mint_max_vote_weight_source: MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                }
            ).into(),
        ],
    ).unwrap();

    // Change Governance config
    proposal.insert_transaction(
        &wallet.creator_keypair,
        &token_owners[0],
        0, (DISTRIBUTION_LIST.len()+2).try_into().unwrap(), 0,
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
    ).unwrap();

    if proposal.get_state().unwrap() == ProposalState::Draft {
        proposal.sign_off_proposal(&wallet.creator_keypair, &token_owners[0]).unwrap();
    }

    for (i, owner) in token_owners.iter().enumerate() {
        let yes_no = i == 0 || i == 3 || i == 4;
        let result = proposal.cast_vote(&token_owners[0], &wallet.voter_keypairs[i], owner, yes_no);
        println!("CastVote {} {:?}", i, result);
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    let result = proposal.execute_transactions(0).unwrap();
    println!("Execute transactions from proposal option 0: {:?}", result);


    // ===================================================================================
    // Check correctly operation after switching to vesting-addin
    // ===================================================================================
    realm.settings_mut().max_voter_weight_record_address = None;
    for (ref mut token_owner) in token_owners.iter_mut() {
        let token_owner_pubkey = token_owner.token_owner_address;
        let voter_weight_record = vesting_addin.get_voter_weight_record_address(&token_owner_pubkey, &realm);
        token_owner.set_voter_weight_record_address(Some(voter_weight_record));
    }

    // ===================================================================================
    // Create proposal
    // ===================================================================================
    let proposal: Proposal = governance.proposal(governance.get_proposals_count());
    proposal.create_proposal(
            &wallet.creator_keypair,
            &token_owners[0],
            "Deploy EVM",
            "Deploy EVM and configure governance to control it",
        ).unwrap();
    println!("{:?}", proposal);

    if proposal.get_state().unwrap() == ProposalState::Draft {
        proposal.sign_off_proposal(&wallet.creator_keypair, &token_owners[0]).unwrap();
    }

    for (i, owner) in token_owners.iter().enumerate() {
        let yes_no = i == 0 || i == 3 || i == 4;
        let result = proposal.cast_vote(&token_owners[0], &wallet.voter_keypairs[i], owner, yes_no);
        println!("CastVote {} {:?}", i, result);
    }
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
            Arg::with_name("force")
                .long("force")
                .short("f")
                .takes_value(false)
                .global(true)
                .help("Force execution dangerous actions")
        )
        .subcommand(SubCommand::with_name("environment")
            .about("Prepare environment for launching")
            .subcommand(SubCommand::with_name("check")
                .about("Check environemnt")
            )
            .subcommand(SubCommand::with_name("setup")
                .about("Setup environment")
            )
        )
        .subcommand(SubCommand::with_name("proposal-tge")
            .about("Prepare and execute proposal for Token Genesis Event")
            .subcommand(SubCommand::with_name("check")
                .about("Check TGE proposal")
            )
            .subcommand(SubCommand::with_name("setup")
                .about("Setup TGE proposal")
            )
            .subcommand(SubCommand::with_name("execute")
                .about("Execute TGE proposal (after approve)")
            )
        )
        .subcommand(SubCommand::with_name("proposal-evm")
            .about("Prepare and execute proposal for launch NeonEVM")
            .subcommand(SubCommand::with_name("check")
                .about("Check Launch NeonEVM proposal")
            )
            .subcommand(SubCommand::with_name("setup")
                .about("Setup Launch NeonEVM proposal")
            )
            .subcommand(SubCommand::with_name("execute")
                .about("Execute Launch NeonEVM proposal")
            )
        ).get_matches();

    let wallet = Wallet::new().unwrap();
    wallet.display();

    let client = Client::new("http://localhost:8899", &wallet.payer_keypair);

    match matches.subcommand() {
        ("environment", Some(arg_matches)) => {
            match arg_matches.subcommand() {
                ("check", Some(arg_matches)) => {
                    process_environment(&wallet, &client, false).unwrap()
                },
                ("setup", Some(arg_matches)) => {
                    process_environment(&wallet, &client, true).unwrap()
                },
                _ => unreachable!(),
            }
        },
        ("proposal-tge", Some(arg_matches)) => {
            match arg_matches.subcommand() {
                ("check", Some(arg_matches)) => {
                },
                ("setup", Some(arg_matches)) => {
                },
                ("execute", Some(arg_matches)) => {
                },
                _ => unreachable!(),
            }
        },
        ("proposal_evm", Some(arg_matches)) => {
            match arg_matches.subcommand() {
                ("check", Some(arg_matches)) => {
                },
                ("setup", Some(arg_matches)) => {
                },
                ("execute", Some(arg_matches)) => {
                },
                _ => unreachable!(),
            }
        },
        _ => unreachable!(),
    }
}
