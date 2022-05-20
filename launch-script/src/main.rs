mod errors;
mod tokens;
mod wallet;
#[macro_use]
mod helpers;
mod msig;
mod token_distribution;
mod schedule_creator;

use crate::{
    tokens::{
        get_mint_data,
        get_account_data,
        create_mint_instructions,
        assert_is_valid_account_data,
    },
    errors::{StateError, ScriptError},
    wallet::Wallet,
    msig::MultiSig,
    helpers::{
        TransactionExecutor,
        TransactionCollector,
        ProposalTransactionInserter,
    },
    token_distribution::{
        TokenDistribution,
    },
    schedule_creator::ScheduleCreator,
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
use solana_clap_utils::{
    input_parsers::{pubkey_of},
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
    governance::Governance,
    proposal::Proposal,
    addin_fixed_weights::{VoterWeight, AddinFixedWeights},
    addin_vesting::AddinVesting,
    token_owner::TokenOwner,
};
use solana_sdk::pubkey;

const REALM_NAME: &str = "NEON";
const NEON_SUPPLY_FRACTION: MintMaxVoteWeightSource = MintMaxVoteWeightSource::SupplyFraction(
        MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE/10);

#[derive(Debug,PartialEq,Copy,Clone)]
pub enum AccountOwner {
    MainGovernance,
    EmergencyGovernance,
    MultiSig(&'static str),
    Key(Pubkey),
}

#[derive(Debug,PartialEq,Copy,Clone)]
pub enum Lockup {
    NoLockup,
    For4Years,
    For1Year_1YearLinear,
}

impl Lockup {
    pub fn default() -> Self {Lockup::For1Year_1YearLinear}

    pub fn is_locked(&self) -> bool {*self != Lockup::NoLockup}

    pub fn get_schedule_size(&self) -> u32 {
        match *self {
            Lockup::NoLockup => 1,
            Lockup::For4Years => 1,
            Lockup::For1Year_1YearLinear => 12,
        }
    }
}

pub struct ExtraTokenAccount {
    pub owner: AccountOwner,
    pub amount: u64,
    pub name: &'static str,
    pub lockup: Lockup,
}

const TOKEN_MULT:u64 = u64::pow(10, 9);

const EXTRA_TOKEN_ACCOUNTS: &[ExtraTokenAccount] = &[
    ExtraTokenAccount {amount:   1_000_000 * TOKEN_MULT, name: "",         lockup: Lockup::For1Year_1YearLinear, owner: AccountOwner::MultiSig("5")},
    ExtraTokenAccount {amount: 142_700_000 * TOKEN_MULT, name: "",         lockup: Lockup::For1Year_1YearLinear, owner: AccountOwner::MultiSig("5")},
    ExtraTokenAccount {amount:   7_500_000 * TOKEN_MULT, name: "",         lockup: Lockup::For1Year_1YearLinear, owner: AccountOwner::MultiSig("4")},
    ExtraTokenAccount {amount:   3_750_000 * TOKEN_MULT, name: "",         lockup: Lockup::For1Year_1YearLinear, owner: AccountOwner::MultiSig("4")},
    ExtraTokenAccount {amount:  60_000_000 * TOKEN_MULT, name: "",         lockup: Lockup::For4Years,            owner: AccountOwner::MultiSig("2")},
    ExtraTokenAccount {amount: 188_762_400 * TOKEN_MULT, name: "",         lockup: Lockup::NoLockup,             owner: AccountOwner::MultiSig("1")},
    ExtraTokenAccount {amount: 210_000_000 * TOKEN_MULT, name: "Treasury", lockup: Lockup::NoLockup,             owner: AccountOwner::MainGovernance},
    ExtraTokenAccount {amount:  80_000_000 * TOKEN_MULT, name: "IDO pool", lockup: Lockup::NoLockup,             owner: AccountOwner::Key(pubkey!("tstzQJwDhrPNSmqtV5rmC26xbbeBf56xFz9wpyTV7tW"))},
];

const MULTI_SIGS: &[MultiSig] = &[
    MultiSig {name: "1", threshold: 2,
        signers: &[
            pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
            pubkey!("6tAoNNAB6sXMbt8phMjr46noQ5T18GnnkBftWcw1HfCW"),
            pubkey!("EsyJ9wzg2VTCCfHmnyi7ePE9LU368iVCrEd4LZeDYMzJ"),
        ],
    },
    MultiSig {name: "2", threshold: 2,
        signers: &[
            pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
            pubkey!("H3cAYot4UJuY1jQhn8FtpeP4fHia3SXtvuKYaov7KMA9"),
            pubkey!("8ZjncH1eKhJMmwqymWwPEAEaPjTSt91R1gMwx2bMyZqC"),
        ],
    },
    MultiSig {name: "4", threshold: 2,
        signers: &[
            pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
            pubkey!("2Smf7Kyskf3VXUKUB16GVgCizW4qDhvRREGCLcHt7bJV"),
            pubkey!("EwNeN5ixjqNmBNGbVKDHd1iipStGhMC9u5yGsq7zsw6L"),
        ],
    },
    MultiSig {name: "5", threshold: 2,
        signers: &[
            pubkey!("tstUPDM1tDgRgC8KALbXQ3hJeKQQTxDywyDVvxv51Lu"),
            pubkey!("tstTLYLzy9Q5meFUmhhiXfnaGai96hc7Ludu3gQz8nh"),
            pubkey!("tstVEdAx9DpjzGefNDMEYV6fxasM5QFBZsssadZn3SB"),
        ],
    },
];

pub struct AccountOwnerResolver<'a> {
    wallet: &'a Wallet,
    client: &'a Client<'a>,
    multi_sigs: &'a [MultiSig],
}

impl<'a> AccountOwnerResolver<'a> {
    pub fn new(wallet: &'a Wallet, client: &'a Client, multi_sigs: &'a [MultiSig]) -> Self {
        Self {wallet, client, multi_sigs}
    }

    pub fn get_owner_pubkey(&self, account_owner: &AccountOwner) -> Result<Pubkey, ScriptError> {
        match account_owner {
            AccountOwner::MainGovernance => {
                let realm = Realm::new(self.client, &self.wallet.governance_program_id, REALM_NAME, &self.wallet.community_pubkey);
                let governance = realm.governance(&self.wallet.community_pubkey);
                Ok(governance.governance_address)
            },
            AccountOwner::EmergencyGovernance => unreachable!(),
            AccountOwner::MultiSig(msig_name) => {
                if self.multi_sigs.iter().find(|v| v.name == *msig_name).is_none() {
                    return Err(StateError::UnknownMultiSig(msig_name.to_string()).into());
                };
                let seed: String = format!("MSIG_{}", msig_name);
                let msig_mint = Pubkey::create_with_seed(&self.wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;
                let msig_realm = Realm::new(self.client, &self.wallet.governance_program_id, &seed, &msig_mint);
                let msig_governance = msig_realm.governance(&msig_mint);
                Ok(msig_governance.governance_address)
            },
            AccountOwner::Key(pubkey) => {
                Ok(*pubkey)
            }
        }
    }
}

fn process_environment(wallet: &Wallet, client: &Client, setup: bool, verbose: bool) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {client, setup, verbose};

    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(&client, wallet.vesting_addin_id);
    let main_governance = realm.governance(&wallet.community_pubkey);
    let emergency_governance = realm.governance(&wallet.governance_program_id);

    let account_owner_resolver = AccountOwnerResolver::new(wallet, client, MULTI_SIGS);
    let token_distribution = TokenDistribution::new(&fixed_weight_addin, &account_owner_resolver, EXTRA_TOKEN_ACCOUNTS)?;
    token_distribution.validate()?;

    let msig_total_amounts = 0; //MULTI_SIGS.iter().map(|v| v.amounts.iter().map(|u| u.1).sum::<u64>()).sum::<u64>();
    println!("MULTI_SIGS total amount: {}", msig_total_amounts);
    for msig in MULTI_SIGS {
        msig::setup_msig(wallet, client, &executor, msig)?; 
    }

    // ----------- Check or create community mint ----------------------
    executor.check_and_create_object("Mint", get_mint_data(client, &wallet.community_pubkey)?,
        |d| {
            if !d.mint_authority.contains(&wallet.creator_keypair.pubkey()) &&
                    !d.mint_authority.contains(&main_governance.governance_address) {
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
                        9,
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
                    d.authority != Some(main_governance.governance_address) {
                return Err(StateError::InvalidRealmAuthority(realm.realm_address, d.authority).into());
            }
            Ok(None)
        },
        || {
            let transaction = client.create_transaction_with_payer_only(
                &[
                    realm.create_realm_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &RealmConfig {
                            council_token_mint: None,
                            community_voter_weight_addin: Some(wallet.fixed_weight_addin_id),
                            max_community_voter_weight_addin: Some(wallet.fixed_weight_addin_id),
                            min_community_weight_to_create_governance: 1,            // TODO Verify parameters!
                            community_mint_max_vote_weight_source: NEON_SUPPLY_FRACTION,
                        }
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

    // ------------ Transfer tokens to Vesting-addin MaxVoterWeightRecord ------
    {
        let record_address = vesting_addin.get_max_voter_weight_record_address(&realm);
        let record_length = vesting_addin.get_max_voter_weight_account_size();
        let record_lamports = Rent::default().minimum_balance(record_length);
        executor.check_and_create_object("Vesting max_voter_weight_record",
            client.get_account(&record_address)?,
            |v| {
                if v.lamports < record_lamports {
                    let transaction = client.create_transaction_with_payer_only(
                        &[
                            system_instruction::transfer(
                                &wallet.payer_keypair.pubkey(),
                                &record_address,
                                record_lamports - v.lamports,
                            ),
                        ],
                    )?;
                    return Ok(Some(transaction))
                }
                Ok(None)
            },
            || {
                let transaction = client.create_transaction_with_payer_only(
                    &[
                        system_instruction::transfer(
                            &wallet.payer_keypair.pubkey(),
                            &record_address,
                            record_lamports,
                        ),
                    ],
                )?;
                Ok(Some(transaction))
            }
        )?;
    }

    // -------------------- Create accounts for token_owner --------------------
    let special_accounts = token_distribution.get_special_accounts();
    for (i, voter_weight) in token_distribution.voter_list.iter().enumerate() {

        let token_owner_record = realm.token_owner_record(&voter_weight.voter);
        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;

        executor.check_and_create_object(&format!("{} <- {}", seed, voter_weight.voter), token_owner_record.get_data()?,
            |_| {
                // TODO check that all accounts needed to this owner created correctly
                let fixed_weight_record_address = fixed_weight_addin.get_voter_weight_record_address(&realm, &voter_weight.voter);
                let vesting_weight_record_address = vesting_addin.get_voter_weight_record_address(&voter_weight.voter, &realm);
                println!("VoterWeightRecords: fixed {}, vesting {}", fixed_weight_record_address, vesting_weight_record_address);
                Ok(None)
            },
            || {
                let mut instructions = vec![
                    token_owner_record.create_token_owner_record_instruction(),
                    fixed_weight_addin.setup_voter_weight_record_instruction(
                            &realm, &voter_weight.voter),
                    system_instruction::transfer(        // Charge VestingAddin::VoterWeightRecord
                        &wallet.payer_keypair.pubkey(),
                        &vesting_addin.get_voter_weight_record_address(&voter_weight.voter, &realm),
                        Rent::default().minimum_balance(vesting_addin.get_voter_weight_account_size()),
                    ),
                ];
                if !special_accounts.contains(&voter_weight.voter) {
                    instructions.extend(vec![
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
                        system_instruction::transfer(         // Charge VestingRecord
                            &wallet.payer_keypair.pubkey(),
                            &vesting_addin.find_vesting_account(&vesting_token_account),
                            Rent::default().minimum_balance(vesting_addin.get_vesting_account_size(Lockup::default().get_schedule_size(), true)),
                        ),
                    ]);
                    let transaction = client.create_transaction(
                        &instructions,
                        &[&wallet.creator_keypair]
                    )?;
                    Ok(Some(transaction))
                } else {
                    let transaction = client.create_transaction_with_payer_only(
                        &instructions,
                    )?;
                    Ok(Some(transaction))
                }
            }
        )?;
    }

    // -------------------- Create extra token accounts ------------------------
    for (i,token_account) in token_distribution.extra_token_accounts.iter().enumerate() {
        let seed: String = format!("{}_account_{}", REALM_NAME, i);
        let token_account_address = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;
        let token_account_owner = if token_account.lockup.is_locked() {
            vesting_addin.find_vesting_account(&token_account_address)
        } else {
            account_owner_resolver.get_owner_pubkey(&token_account.owner)?
        };
        println!("Extra token account '{}' {} owner by {}", seed, token_account_address, token_account_owner);

        executor.check_and_create_object(&seed, get_account_data(client, &token_account_address)?,
            |d| {
                assert_is_valid_account_data(d, &token_account_address,
                        &wallet.community_pubkey, &token_account_owner)?;
                Ok(None)
            },
            || {
                let mut instructions = vec![
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
                    ];
                if token_account.lockup.is_locked() {
                    instructions.extend(vec![
                        system_instruction::transfer(         // Charge VestingRecord
                            &wallet.payer_keypair.pubkey(),
                            &vesting_addin.find_vesting_account(&token_account_address),
                            Rent::default().minimum_balance(vesting_addin.get_vesting_account_size(token_account.lockup.get_schedule_size(), true)),
                        ),
                    ]);
                }
                let transaction = client.create_transaction(
                    &instructions,
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
    executor.check_and_create_object("Main governance", main_governance.get_data()?,
        |_| {Ok(None)},
        || {
            let transaction = client.create_transaction(
                &[
                    main_governance.create_governance_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &creator_token_owner_record,
                        GovernanceConfig {
                            vote_threshold_percentage: VoteThresholdPercentage::YesVote(16),
                            min_community_weight_to_create_proposal: 3_000,
                            min_transaction_hold_up_time: 0,
                            max_voting_time: 3*60, //78200,
                            vote_tipping: VoteTipping::Disabled,
                            proposal_cool_off_time: 0,
                            min_council_weight_to_create_proposal: 0,
                        },
                    ),
                ],
                &[&wallet.creator_keypair]
            )?;
            Ok(Some(transaction))
        }
    )?;

    // ------------- Setup emergency governance ------------------------
    executor.check_and_create_object("Emergency governance", emergency_governance.get_data()?,
        |_| {Ok(None)},
        || {
            let transaction = client.create_transaction(
                &[
                    emergency_governance.create_governance_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &creator_token_owner_record,
                        GovernanceConfig {
                            vote_threshold_percentage: VoteThresholdPercentage::YesVote(90),
                            min_community_weight_to_create_proposal: 1_000_000,
                            min_transaction_hold_up_time: 0,
                            max_voting_time: 3*60, //78200,
                            vote_tipping: VoteTipping::Disabled,
                            proposal_cool_off_time: 0,
                            min_council_weight_to_create_proposal: 0,
                        },
                    ),
                ],
                &[&wallet.creator_keypair]
            )?;
            Ok(Some(transaction))
        }
    )?;

    // --------- Create NEON associated token account -------------
    let governance_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &main_governance.governance_address, &wallet.community_pubkey, &spl_token::id());
    println!("Main governance address: {}", main_governance.governance_address);
    println!("Main governance token account: {}", governance_token_account);

    executor.check_and_create_object("NEON-token main-governance account",
        get_account_data(client, &governance_token_account)?,
        |d| {
            assert_is_valid_account_data(d, &governance_token_account,
                    &wallet.community_pubkey, &main_governance.governance_address)?;
            Ok(None)
        },
        || {
            let transaction = client.create_transaction_with_payer_only(
                &[
                    spl_associated_token_account::create_associated_token_account(
                        &wallet.payer_keypair.pubkey(),
                        &main_governance.governance_address,
                        &wallet.community_pubkey,
                    ).into(),
                ],
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
                            Some(&main_governance.governance_address),
                            spl_token::instruction::AuthorityType::MintTokens,
                            &wallet.creator_keypair.pubkey(),
                            &[],
                        ).unwrap()
                    ].to_vec();
                let signers = [&wallet.creator_keypair].to_vec();
                Ok(Some((instructions, signers,)))
            } else if d.mint_authority.contains(&main_governance.governance_address) {
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
                            Some(&main_governance.governance_address),
                            SetRealmAuthorityAction::SetChecked,
                        )
                    ].to_vec();
                let signers = [&wallet.creator_keypair].to_vec();
                Ok(Some((instructions, signers,)))
            } else if d.authority == Some(main_governance.governance_address) {
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
                                Some(&emergency_governance.governance_address),
                            )?
                        ].to_vec();
                    let signers = [&wallet.creator_keypair].to_vec();
                    Ok(Some((instructions, signers,)))
                } else if upgrade_authority == Some(emergency_governance.governance_address) {
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
// Upgrade program
// =========================================================================
fn setup_proposal_upgrade_program(wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        program: &Pubkey, buffer: &Pubkey
) -> Result<(), ScriptError> {
    use borsh::ser::BorshSerialize;
    use spl_governance::{
        state::{
            proposal_transaction::InstructionData,
            vote_record::{Vote, VoteChoice, get_vote_record_address},
        },
        instruction::cast_vote,
    };

    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    executor.check_and_create_object("Program", client.get_program_upgrade_authority(program)?,
        |authority| {
            if *authority != transaction_inserter.proposal.governance.governance_address {
                Err(StateError::InvalidProgramUpgradeAuthority(*program, Some(*authority)).into())
            } else {
                Ok(None)
            }
        },
        || {Err(StateError::InvalidProgramUpgradeAuthority(*program, None).into())},
    )?;

    transaction_inserter.insert_transaction_checked(
            &format!("Upgrade program {}", program),
            vec![
                solana_sdk::bpf_loader_upgradeable::upgrade(
                    program,
                    buffer,
                    &transaction_inserter.proposal.governance.governance_address,
                    &client.payer.pubkey(),
                ).into(),
            ],
        )?;

    Ok(())
}


// =========================================================================
// Vote for proposal
// =========================================================================
fn setup_proposal_vote_proposal(wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        proposal_address: &Pubkey,
) -> Result<(), ScriptError> {
    use borsh::BorshSerialize;
    use spl_governance::{
        instruction::cast_vote,
        state::{
            governance::GovernanceV2,
            realm::RealmV2,
            proposal::ProposalV2,
            vote_record::{Vote, VoteChoice, get_vote_record_address},
            proposal_transaction::InstructionData,
        },
    };

    let voter = transaction_inserter.proposal.governance.governance_address;
    let program_id = transaction_inserter.proposal.governance.realm.program_id;

    let proposal_data = client.get_account_data_borsh::<ProposalV2>(&program_id, &proposal_address)?
            .ok_or(StateError::InvalidProposal)?;
    let governance_data = client.get_account_data_borsh::<GovernanceV2>(&program_id, &proposal_data.governance)?
            .ok_or(StateError::InvalidProposal)?;
    let realm_data = client.get_account_data_borsh::<RealmV2>(&program_id, &governance_data.realm)?
            .ok_or(StateError::InvalidProposal)?;

    let voted_realm = Realm::new(client, &program_id, &realm_data.name, &realm_data.community_mint);
    voted_realm.update_max_voter_weight_record_address()?;
    let voted_governance = voted_realm.governance(&governance_data.governed_account);
    let voted_proposal = voted_governance.proposal(&proposal_address);

    let voter_token_owner = voted_realm.token_owner_record(&voter);
    voter_token_owner.update_voter_weight_record_address()?;

    let vote_record_address = get_vote_record_address(
        &program_id,
        &voted_proposal.proposal_address,
        &voter_token_owner.token_owner_record_address);

    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    executor.check_and_create_object("VoteRecord for voter", client.get_account(&vote_record_address)?,
        |v| {
            println!("Vote record: {:?}", v);
            Ok(None)
        },
        || {
            let lamports = Rent::default().minimum_balance(4+32+32+1+8+(4+4+1+1)+8);
            println_bold!("Charge {} account with {}.{:09} lamports",
                    vote_record_address, lamports/1_000_000_000, lamports%1_000_000_000);
            let transaction = client.create_transaction_with_payer_only(
                &[
                    system_instruction::transfer(
                        &wallet.payer_keypair.pubkey(),
                        &vote_record_address,
                        lamports,
                    ),
                ],
            )?;
            Ok(Some(transaction))
        },
    )?;

    let instruction: InstructionData = cast_vote(
        &program_id,
        &voted_realm.realm_address,
        &voted_governance.governance_address,
        &voted_proposal.proposal_address,
        &proposal_data.token_owner_record,
        &voter_token_owner.token_owner_record_address,
        &voter,
        &voted_realm.community_mint,
        &voter_token_owner.token_owner_address,     // as payer
        voter_token_owner.get_voter_weight_record_address(),
        voted_realm.settings().max_voter_weight_record_address,
        Vote::Approve(vec![VoteChoice {rank: 0, weight_percentage: 100}]),
    ).into();

    println_bold!("Add instruction to proposal: {}", base64::encode(instruction.try_to_vec()?));

    transaction_inserter.insert_transaction_checked(
            &format!("Vote proposal {}", proposal_address),
            vec![instruction],
        )?;

    Ok(())
}


// =========================================================================
// Create TGE proposal (Token Genesis Event)
// =========================================================================
fn setup_proposal_tge(wallet: &Wallet, client: &Client, transaction_inserter: &mut ProposalTransactionInserter, testing: bool) -> Result<(), ScriptError> {
    //let executor = TransactionExecutor {client, setup, verbose};
    let schedule_creator = ScheduleCreator::new(testing);

    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    realm.update_max_voter_weight_record_address()?;

    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(&client, wallet.vesting_addin_id);
    let governance = realm.governance(&wallet.community_pubkey);
    let emergency_governance = realm.governance(&wallet.governance_program_id);

    let account_owner_resolver = AccountOwnerResolver::new(wallet, client, MULTI_SIGS);
    let token_distribution = TokenDistribution::new(&fixed_weight_addin, &account_owner_resolver, EXTRA_TOKEN_ACCOUNTS)?;
    token_distribution.validate()?;

    let governance_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &governance.governance_address, &wallet.community_pubkey, &spl_token::id());
    println!("Governance address: {}", governance.governance_address);
    println!("Governance token account: {}", governance_token_account);

    transaction_inserter.insert_transaction_checked(
            "Mint tokens",
            vec![
                spl_token::instruction::mint_to(
                    &spl_token::id(),
                    &wallet.community_pubkey,
                    &governance_token_account,
                    &governance.governance_address, &[],
                    token_distribution.info.total_amount,
                )?.into(),
            ],
        )?;

    let special_accounts = token_distribution.get_special_accounts();
    for (i, voter) in token_distribution.voter_list.iter().enumerate() {
        if special_accounts.contains(&voter.voter) {continue;}

        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id()).unwrap();
        let schedule = schedule_creator.get_schedule(voter.weight, Lockup::default());

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
                        Some(governance.governance_address),  // payer
                    )?.into(),
                ],
            )?;
    }

    for (i, token_account) in token_distribution.extra_token_accounts.iter().enumerate() {
        let seed: String = format!("{}_account_{}", REALM_NAME, i);
        let token_account_address = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;

        if token_account.lockup.is_locked() {
            let token_account_owner = account_owner_resolver.get_owner_pubkey(&token_account.owner)?;
            let schedule = schedule_creator.get_schedule(token_account.amount, token_account.lockup);

            transaction_inserter.insert_transaction_checked(
                &format!("Deposit {} to {} on token account {}",
                        token_account.amount, token_account_owner, token_account_address),
                vec![
                    vesting_addin.deposit_with_realm_instruction(
                        &governance.governance_address,          // source_token_authority
                        &governance_token_account,    // source_token_account
                        &token_account_owner,         // vesting_owner
                        &token_account_address,       // vesting_token_account
                        schedule,                     // schedule
                        &realm,                       // realm
                        Some(governance.governance_address),  // payer
                    )?.into(),
                ],
            )?;
        } else {
            transaction_inserter.insert_transaction_checked(
                &format!("Transfer {} to {} ({})", token_account.amount, token_account_address, seed),
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
        }
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
                        community_mint_max_vote_weight_source: NEON_SUPPLY_FRACTION,
                    },
                    Some(governance.governance_address),  // payer
                ).into(),
            ],
        )?;

    transaction_inserter.insert_transaction_checked(
            "Pass Realm under Emergency-governance",
            vec![
                realm.set_realm_authority_instruction(
                    &governance.governance_address,
                    Some(&emergency_governance.governance_address),
                    SetRealmAuthorityAction::SetChecked,
                ).into(),
            ],
        )?;

/*    transaction_inserter.insert_transaction_checked(
            "Change Governance config",
            vec![
                governance.set_governance_config_instruction(
                    GovernanceConfig {
                        vote_threshold_percentage: VoteThresholdPercentage::YesVote(16),
                        min_community_weight_to_create_proposal: 3_000,
                        min_transaction_hold_up_time: 0,
                        max_voting_time: 1*60, // 3*24*3600,
                        vote_tipping: VoteTipping::Disabled,
                        proposal_cool_off_time: 0,                 // not implemented in the current version
                        min_council_weight_to_create_proposal: 0,  // council token does not used
                    },
                ).into(),
            ],
        )?;*/

    Ok(())
}

fn finalize_vote_proposal(wallet: &Wallet, client: &Client, proposal: &Proposal, verbose: bool) -> Result<(), ScriptError> {
    let proposal_data = proposal.get_data()?.ok_or(StateError::InvalidProposalIndex)?;
    proposal.finalize_vote(&proposal_data.token_owner_record)?;

    Ok(())
}

fn sign_off_proposal(wallet: &Wallet, client: &Client, proposal: &Proposal, verbose: bool) -> Result<(), ScriptError> {
    let creator_token_owner = proposal.governance.realm.token_owner_record(&wallet.creator_token_owner_keypair.pubkey());
    creator_token_owner.update_voter_weight_record_address()?;

    let proposal_data = proposal.get_data()?.ok_or(StateError::InvalidProposalIndex)?;
    if proposal_data.state == ProposalState::Draft {
        proposal.sign_off_proposal(&wallet.creator_keypair, &creator_token_owner)?;
    }

    Ok(())
}

fn approve_proposal(wallet: &Wallet, client: &Client, proposal: &Proposal, verbose: bool) -> Result<(), ScriptError> {
    let proposal_data = proposal.get_data()?.ok_or(StateError::InvalidProposalIndex)?;

    for voter in wallet.voter_keypairs.iter() {
        let token_owner = proposal.governance.realm.token_owner_record(&voter.pubkey());
        if let Some(_) = token_owner.get_data()? {
            token_owner.update_voter_weight_record_address()?;

            let signature = proposal.cast_vote(&proposal_data.token_owner_record, voter, &token_owner, true)?;
            println!("CastVote {} {:?}", voter.pubkey(), signature);
        }
    }

    Ok(())
}

fn execute_proposal(wallet: &Wallet, client: &Client, proposal: &Proposal, verbose: bool) -> Result<(), ScriptError> {
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
        .arg(
            Arg::with_name("testing")
                .long("testing")
                .takes_value(false)
                .help("Configure testing environment")
        )
        .subcommand(SubCommand::with_name("environment")
            .about("Prepare environment for launching")
        )
        .subcommand(SubCommand::with_name("proposal")
            .about("Prepare and execute proposal")
            .arg(
                Arg::with_name("name")
                    .long("name")
                    .short("n")
                    .takes_value(true)
                    .value_name("PROPOSAL_NAME")
                    .help("Proposal name")
            )
            .arg(
                Arg::with_name("description")
                    .long("description")
                    .short("d")
                    .takes_value(true)
                    .value_name("PROPOSAL_DESCRIPTION")
                    .help("Proposal description")
            )
            .arg(
                Arg::with_name("proposal")
                    .long("proposal")
                    .short("p")
                    .takes_value(true)
                    .value_name("PROPOSAL_ADDRESS")
                    .help("Proposal address")
            )
            .arg(
                Arg::with_name("governance")
                    .long("governance")
                    .short("g")
                    .takes_value(true)
                    .value_name("GOVERNANCE")
                    .help("Governance name (COMMUNITY, EMERGENCY, MSIG_#, or <address>)")
            )
            .subcommand(SubCommand::with_name("create-tge")
                .about("Create Token Genesis Event proposal")
            )
            .subcommand(SubCommand::with_name("create-empty")
                .about("Create Empty proposal")
            )
            .subcommand(SubCommand::with_name("create-upgrade-program")
                .about("Create proposal for upgrade program")
                .arg(
                    Arg::with_name("program")
                        .long("program")
                        .takes_value(true)
                        .value_name("PROGRAM")
                        .help("Program address")
                )
                .arg(
                    Arg::with_name("buffer")
                        .long("buffer")
                        .short("b")
                        .takes_value(true)
                        .value_name("BUFFER")
                        .help("Buffer with new program")
                )
            )
            .subcommand(SubCommand::with_name("create-vote-proposal")
                .about("Create proposal for CastVote")
                .arg(
                    Arg::with_name("vote-proposal")
                        .long("vote-proposal")
                        .takes_value(true)
                        .value_name("VOTE_PROPOSAL")
                        .help("Proposal for vote")
                )
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
    let testing: bool = matches.is_present("testing");
    match matches.subcommand() {
        ("environment", Some(arg_matches)) => {
            process_environment(&wallet, &client, send_trx, verbose).unwrap()
        },
        ("proposal", Some(arg_matches)) => {
            let governance_name = arg_matches.value_of("governance").unwrap_or("COMMUNITY");
            let (realm_name, realm_mint, governed_address) = match governance_name {
                "COMMUNITY" => (REALM_NAME, wallet.community_pubkey, wallet.community_pubkey),
                "EMERGENCY" => (REALM_NAME, wallet.community_pubkey, wallet.governance_program_id),
                name if name.starts_with("MSIG_") => {
                    let msig_mint = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), name, &spl_token::id()).unwrap();
                    (name, msig_mint, msig_mint)
                },
                _ => unreachable!(),
            };
            let realm = Realm::new(&client, &wallet.governance_program_id, realm_name, &realm_mint);
            realm.update_max_voter_weight_record_address().unwrap();
            let governance = realm.governance(&governed_address);

            match arg_matches.subcommand() {
                (cmd, Some(cmd_matches)) if cmd.starts_with("create-") => {
                    let executor = TransactionExecutor {client: &client, setup: send_trx, verbose};
                    let creator_owner_record = {
                        let creator = wallet.creator_keypair.pubkey();
                        let owner_record = realm.find_owner_or_delegate_record(&creator).unwrap();
                        executor.check_and_create_object("Creator owner record", owner_record.as_ref(),
                            |record| {
                                record.update_voter_weight_record_address()?;
                                Ok(None)
                            },
                            || {Err(StateError::MissingTokenOwnerRecord(creator).into())},
                        ).unwrap();
                        owner_record.unwrap_or(realm.token_owner_record(&creator))
                    };

                    let proposal = if let Some(proposal_address) = pubkey_of(&arg_matches, "proposal") {
                        let proposal = governance.proposal(&proposal_address);
                        executor.check_and_create_object("Proposal", proposal.get_data().unwrap(),
                            |p| {
                                if p.governance != governance.governance_address ||
                                   p.governing_token_mint != realm.community_mint
                                {
                                    return Err(StateError::InvalidProposal.into());
                                }
                                Ok(None)
                            },
                            || {Err(StateError::MissingProposal(proposal_address).into())},
                        ).unwrap();
                        proposal
                    } else {
                        let proposal_index = governance.get_proposals_count();
                        let proposal = governance.proposal_by_index(proposal_index);
                        let name = arg_matches.value_of("name").unwrap();
                        let description = arg_matches.value_of("description").unwrap_or("");
                        executor.check_and_create_object("Proposal", proposal.get_data().unwrap(),
                            |p| {
                                if p.governance != governance.governance_address ||
                                   p.governing_token_mint != realm.community_mint
                                {
                                    return Err(StateError::InvalidProposal.into());
                                }
                                Ok(None)
                            },
                            || {
                                let transaction = client.create_transaction(
                                    &[
                                        proposal.create_proposal_instruction(
                                            &wallet.creator_keypair.pubkey(),
                                            &creator_owner_record,
                                            proposal_index, name, description,
                                        ),
                                    ],
                                    &[&wallet.creator_keypair],
                                )?;
                                Ok(Some(transaction))
                            },
                        ).unwrap();
                        proposal
                    };
                    println_bold!("Proposal: {}, Token owner: {}", proposal.proposal_address, creator_owner_record.token_owner_address);

                    let mut transaction_inserter = ProposalTransactionInserter {
                        proposal: &proposal,
                        creator_keypair: &wallet.creator_keypair,
                        creator_token_owner: &creator_owner_record,
                        hold_up_time: 0,     // TODO: Get hold_up_time from parameters
                        setup: send_trx,
                        verbose: verbose,
                        proposal_transaction_index: 0,
                    };
                    match cmd {
                        "create-tge" => setup_proposal_tge(&wallet, &client, &mut transaction_inserter, testing).unwrap(),
                        "create-empty" => {},
                        "create-upgrade-program" => {
                            let program: Pubkey = pubkey_of(&cmd_matches, "program").unwrap();
                            let buffer: Pubkey = pubkey_of(&cmd_matches, "buffer").unwrap();
                            setup_proposal_upgrade_program(&wallet, &client, &mut transaction_inserter, &program, &buffer).unwrap();
                        },
                        "create-vote-proposal" => {
                            let vote_proposal: Pubkey = pubkey_of(&cmd_matches, "vote-proposal").unwrap();
                            setup_proposal_vote_proposal(&wallet, &client, &mut transaction_inserter, &vote_proposal).unwrap();
                        },
                        _ => unreachable!(),
                    }
                },
                (cmd, _arg_matches) if ["sign-off", "approve", "finalize-vote", "execute"].contains(&cmd) => {
                    let proposal = governance.proposal(&pubkey_of(&arg_matches, "proposal").unwrap());

                    let creator_owner_record = realm.find_owner_or_delegate_record(&wallet.creator_keypair.pubkey()).unwrap().unwrap();
                    creator_owner_record.update_voter_weight_record_address().unwrap();
                    println!("Creator owner record: {}", creator_owner_record);

                    match cmd {
                        "sign-off" => sign_off_proposal(&wallet, &client, &proposal, verbose).unwrap(),
                        "approve"  => approve_proposal(&wallet, &client, &proposal, verbose).unwrap(),
                        "finalize-vote" => finalize_vote_proposal(&wallet, &client, &proposal, verbose).unwrap(),
                        "execute" => execute_proposal(&wallet, &client, &proposal, verbose).unwrap(),
                        _ => unreachable!(),
                    }
                },
                _ => unreachable!(),
            }
        },
        _ => unreachable!(),
    }
}
