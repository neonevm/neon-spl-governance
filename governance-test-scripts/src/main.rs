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
        proposal_transaction::InstructionData,
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

    // -------------------- Create extra token accounts ------------------------
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
    let creator_voter_weight_record = fixed_weight_addin.get_voter_weight_record_address(&realm, &voter_list[0].voter);
    creator_token_owner.set_voter_weight_record_address(Some(creator_voter_weight_record));


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
    }

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



struct ProposalTransactionInserter<'a> {
    pub proposal: &'a Proposal<'a>,
    pub creator_keypair: &'a Keypair,
    pub creator_token_owner: &'a TokenOwner<'a>,
    pub hold_up_time: u32,
    pub setup: bool,

    pub proposal_transaction_index: u16,
}

impl<'a> ProposalTransactionInserter<'a> {
    pub fn insert_transaction_checked(&mut self, name: &str, instructions: Vec<InstructionData>) -> Result<(), ScriptError> {
        if let Some(transaction_data) = self.proposal.get_proposal_transaction_data(0, self.proposal_transaction_index)? {
            println_item!("Proposal transaction '{}'/{}: {:?}", name, self.proposal_transaction_index, transaction_data);
            if transaction_data.instructions != instructions {
                let error = StateError::InvalidProposalTransaction(self.proposal_transaction_index);
                if self.setup {return Err(error.into())} else {println_error!("{:?}", error);}
            } else {
                println!("Proposal transaction '{}'/{} correct", name, self.proposal_transaction_index);
            }
        } else if self.setup {
            let signature = self.proposal.insert_transaction(
                    &self.creator_keypair,
                    &self.creator_token_owner,
                    0, self.proposal_transaction_index, self.hold_up_time,
                    instructions
                )?;
            println!("Proposal transaction '{}'/{} was inserted in trx: {}", name, self.proposal_transaction_index, signature);
        } else {
            println!("Proposal transaction '{}'/{} will be inserted", name, self.proposal_transaction_index);
        }
        self.proposal_transaction_index += 1;
        Ok(())
    }
}

// =========================================================================
// Create TGE proposal (Token Genesis Event)
// =========================================================================
fn setup_proposal_tge(wallet: &Wallet, client: &Client, proposal_index: Option<u32>, setup: bool) -> Result<(), ScriptError> {

    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(&client, wallet.vesting_addin_id);
    let governance = realm.governance(&wallet.governed_account_pubkey);

    // TODO it is correct for fixed_weight_addin only!
    let max_voter_weight_record_address = fixed_weight_addin.setup_max_voter_weight_record(&realm).unwrap();
    realm.settings_mut().max_voter_weight_record_address = Some(max_voter_weight_record_address);

    // TODO is is correct for fixed_weight_addin only!
    let creator_token_owner = {
        let voter_list = fixed_weight_addin.get_voter_list()?;
        let mut creator_token_owner = realm.token_owner_record(&voter_list[0].voter);
        let creator_voter_weight_record = fixed_weight_addin.get_voter_weight_record_address(&realm, &voter_list[0].voter);
        creator_token_owner.set_voter_weight_record_address(Some(creator_voter_weight_record));
        creator_token_owner
    };

    let governance_proposal_count = governance.get_proposals_count();
    let proposal_number = proposal_index.unwrap_or(governance_proposal_count);
    if proposal_number > governance_proposal_count {return Err(StateError::InvalidProposalIndex.into());}
    println!("Use {} for proposal_index", proposal_number);

    let mut proposal_transaction_count: u16 = 0;
    let hold_up_time = 0;
    let proposal: Proposal = governance.proposal(proposal_number);
    if let Some(proposal_data) = proposal.get_data()? {
        // TODO check proposal data (and state)!
        println_item!("TGE: {:?}", proposal_data);
        println!("Proposal TGE exists");
    } else if setup {
        let signature = client.send_and_confirm_transaction(
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
        println!("Proposal TGE created in trx: {}", signature);
    } else {
        println!("Proposal TGE will be created");
    }

    // let result = client.add_signatory(&realm, &governance, &proposal, &token_owner);
    // println!("Add signatory {:?}", result);
    
    let mut transaction_inserter = ProposalTransactionInserter {
        proposal: &proposal,
        creator_keypair: &wallet.creator_keypair,
        creator_token_owner: &creator_token_owner,
        hold_up_time: 0,
        setup: setup,
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
                       extra_token_accounts.iter().map(|v| v.amount).sum::<u64>();

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

    for token_account in extra_token_accounts.iter() {
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
            
    if proposal.get_state()? == ProposalState::Draft && setup {
        proposal.sign_off_proposal(&wallet.creator_keypair, &creator_token_owner)?;
    }

    Ok(())
}

fn approve_proposal(wallet: &Wallet, client: &Client, proposal_index: Option<u32>) -> Result<(), ScriptError> {
    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(&client, wallet.vesting_addin_id);
    let governance = realm.governance(&wallet.governed_account_pubkey);

    // TODO it is correct for fixed_weight_addin only!
    let max_voter_weight_record_address = fixed_weight_addin.setup_max_voter_weight_record(&realm).unwrap();
    realm.settings_mut().max_voter_weight_record_address = Some(max_voter_weight_record_address);

    // TODO is is correct for fixed_weight_addin only!
    let creator_token_owner = {
        let voter_list = fixed_weight_addin.get_voter_list()?;
        let mut creator_token_owner = realm.token_owner_record(&voter_list[0].voter);
        let creator_voter_weight_record = fixed_weight_addin.get_voter_weight_record_address(&realm, &voter_list[0].voter);
        creator_token_owner.set_voter_weight_record_address(Some(creator_voter_weight_record));
        creator_token_owner
    };

    let governance_proposal_count = governance.get_proposals_count();
    let proposal_number = proposal_index.unwrap_or(governance_proposal_count);
    if proposal_number > governance_proposal_count {return Err(StateError::InvalidProposalIndex.into());}
    println!("Use {} for proposal_index", proposal_number);

    let proposal: Proposal = governance.proposal(proposal_number);
    if let None = proposal.get_data()? {
        return Err(StateError::InvalidProposalIndex.into());
    }

    let voter_list = fixed_weight_addin.get_voter_list()?;
    for (i, voter_weight) in voter_list.iter().enumerate() {
        let mut token_owner = realm.token_owner_record(&voter_weight.voter);
        // TODO Valid for fixed_weight addin
        let voter_weight_record = fixed_weight_addin.get_voter_weight_record_address(&realm, &voter_weight.voter);
        token_owner.set_voter_weight_record_address(Some(voter_weight_record));

        let signature = proposal.cast_vote(&creator_token_owner, &wallet.voter_keypairs[i], &token_owner, true)?;
        println!("CastVote {} {:?}", i, signature);
    }

    Ok(())
}

fn execute_proposal(wallet: &Wallet, client: &Client, proposal_index: Option<u32>) -> Result<(), ScriptError> {
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
            .arg(
                Arg::with_name("send_trx")
                    .long("send-trx")
                    .takes_value(false)
                    .help("Send transactions to blockchain")
            )
            .subcommand(SubCommand::with_name("create-tge")
                .about("Create Token Genesis Event proposal")
            )
            .subcommand(SubCommand::with_name("approve")
                .about("Approve proposal")
            )
            .subcommand(SubCommand::with_name("execute")
                .about("Execute proposal (after approve)")
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
        ("proposal", Some(arg_matches)) => {
            let proposal_index = arg_matches.value_of("index").map(|v| v.parse::<u32>().unwrap());
            let send_trx: bool = arg_matches.is_present("send_trx");
            match arg_matches.subcommand() {
                ("create-tge", Some(arg_matches)) => {
                    setup_proposal_tge(&wallet, &client, proposal_index, send_trx).unwrap()
                },
                ("approve", Some(arg_matches)) => {
                    approve_proposal(&wallet, &client, proposal_index).unwrap()
                },
                ("execute", Some(arg_matches)) => {
                    execute_proposal(&wallet, &client, proposal_index).unwrap()
                },
                _ => unreachable!(),
            }
        },
        _ => unreachable!(),
    }
}
