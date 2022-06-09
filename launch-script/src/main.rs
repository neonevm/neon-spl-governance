mod errors;
mod tokens;
mod wallet;
#[macro_use]
mod helpers;
mod msig;
mod token_distribution;
mod schedule_creator;
mod clap_utils;

use std::path::Path;
use crate::{
    tokens::{
        get_mint_data,
        get_account_data,
        get_multisig_data,
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
    clap_utils::is_valid_pubkey_or_none,
};
use solana_sdk::{
    pubkey::Pubkey,
    signer::{
        Signer,
    },
    system_instruction,
    rent::Rent,
};
use solana_clap_utils::{
    input_parsers::{pubkey_of, value_of},
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
    ArgMatches,
};

use governance_lib::{
    client::Client,
    realm::{RealmConfig, Realm},
    governance::Governance,
    proposal::Proposal,
    token_owner::TokenOwner,
    addin_fixed_weights::{AddinFixedWeights},
    addin_vesting::AddinVesting,
};
use solana_sdk::pubkey;

const REALM_NAME: &str = "NEON";
const NEON_SUPPLY_FRACTION: MintMaxVoteWeightSource = MintMaxVoteWeightSource::SupplyFraction(
        MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE/10);

enum ProposalInfo {
    Create(String,String),     // name, description
    Exists(Pubkey),            // proposal address
    Last,                      // last proposal in governance
}

#[derive(Debug,PartialEq,Copy,Clone)]
pub enum AccountOwner {
    MainGovernance,
    EmergencyGovernance,
    BothGovernance,
    MultiSig(&'static str),
    Key(Pubkey),
}

#[derive(Debug,PartialEq,Copy,Clone)]
pub enum Lockup {
    NoLockup,
    For4Years,
    For1year1yearLinear,
}

impl Lockup {
    pub fn default() -> Self {Lockup::For1year1yearLinear}

    pub fn is_locked(&self) -> bool {*self != Lockup::NoLockup}

    pub fn get_schedule_size(&self) -> u32 {
        match *self {
            Lockup::NoLockup => 1,
            Lockup::For4Years => 1,
            Lockup::For1year1yearLinear => 12,
        }
    }
}

pub struct ExtraTokenAccount {
    pub owner: AccountOwner,
    pub amount: u64,
    pub name: &'static str,
    pub lockup: Lockup,
}

//const CREATOR_TOKENS_SEED: str = format!("{}_account_0", REALM_NAME);
const TOKEN_MULT:u64 = u64::pow(10, 9);
const EXTRA_TOKEN_ACCOUNTS: &[ExtraTokenAccount] = &[
    ExtraTokenAccount {amount:   1_000_000 * TOKEN_MULT, name: "",         lockup: Lockup::For1year1yearLinear,  owner: AccountOwner::MultiSig("5.1")},
    ExtraTokenAccount {amount: 142_700_000 * TOKEN_MULT, name: "",         lockup: Lockup::For1year1yearLinear,  owner: AccountOwner::MultiSig("5")},
    ExtraTokenAccount {amount:   7_500_000 * TOKEN_MULT, name: "",         lockup: Lockup::For1year1yearLinear,  owner: AccountOwner::MultiSig("4")},
    ExtraTokenAccount {amount:   3_750_000 * TOKEN_MULT, name: "",         lockup: Lockup::For1year1yearLinear,  owner: AccountOwner::MultiSig("4")},
    ExtraTokenAccount {amount:  60_000_000 * TOKEN_MULT, name: "",         lockup: Lockup::For4Years,            owner: AccountOwner::MultiSig("2")},
    ExtraTokenAccount {amount: 188_762_400 * TOKEN_MULT, name: "",         lockup: Lockup::NoLockup,             owner: AccountOwner::MultiSig("1")},
    ExtraTokenAccount {amount: 210_000_000 * TOKEN_MULT, name: "Treasury", lockup: Lockup::NoLockup,             owner: AccountOwner::BothGovernance},
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
            pubkey!("9G9A27t7FEEP5L53svAPweJyZCDFESsGd5tvnQEZFJUY"),    // payer
        ],
    },
    MultiSig {name: "5.1", threshold: 2,
        signers: &[    // signers same as for MultiSig("5")
            pubkey!("tstUPDM1tDgRgC8KALbXQ3hJeKQQTxDywyDVvxv51Lu"),
            pubkey!("tstTLYLzy9Q5meFUmhhiXfnaGai96hc7Ludu3gQz8nh"),
            pubkey!("9G9A27t7FEEP5L53svAPweJyZCDFESsGd5tvnQEZFJUY"),    // payer
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
            AccountOwner::BothGovernance => {
                let multisig = Pubkey::create_with_seed(&self.wallet.creator_pubkey, &format!("{}_multisig", REALM_NAME), &spl_token::id())?;
                Ok(multisig)
            },
            AccountOwner::MultiSig(msig_name) => {
                if !self.multi_sigs.iter().any(|v| v.name == *msig_name) {
                    return Err(StateError::UnknownMultiSig(msig_name.to_string()).into());
                };
//                let multi_sig = self.multi_sigs.iter().find(|v| v.name == *msig_name)
//                        .ok_or(Err(StateError::UnknownMultiSig(msig_name.to_string())))?;
//                let governed_account: Pubkey = match account_seed {
//                    Some(account_seed) => Pubkey::create_with_seed(&self.wallet.creator_pubkey, &account_seed, &spl_token_id())?;
//                    None => msig_mint,
//                };
                let seed: String = format!("MSIG_{}", msig_name);
                let msig_mint = Pubkey::create_with_seed(&self.wallet.creator_pubkey, &seed, &spl_token::id())?;
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

    let realm = Realm::new(client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    let fixed_weight_addin = AddinFixedWeights::new(client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(client, wallet.vesting_addin_id);
    let main_governance = realm.governance(&wallet.community_pubkey);
    let emergency_governance = realm.governance(&wallet.governance_program_id);
    let neon_multisig =  Pubkey::create_with_seed(&wallet.creator_pubkey, &format!("{}_multisig", REALM_NAME), &spl_token::id())?;

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
            if !d.mint_authority.contains(&wallet.creator_pubkey) &&
                    !d.mint_authority.contains(&main_governance.governance_address) &&
                    !d.mint_authority.contains(&neon_multisig) {
                return Err(StateError::InvalidMintAuthority(wallet.community_pubkey, d.mint_authority).into());
            }
            if d.decimals != 9 {
                return Err(StateError::InvalidMintPrecision(wallet.community_pubkey).into());
            }
            Ok(None)
        },
        || {Err(StateError::MissingMint(wallet.community_pubkey).into())}
    )?;

    // -------------- Check or create Realm ---------------------------
    executor.check_and_create_object("Realm", realm.get_data()?,
        |d| {
            if d.community_mint != realm.community_mint {
                return Err(StateError::InvalidRealmCommunityMint(realm.realm_address, d.community_mint).into());
            }
            if d.authority != Some(wallet.creator_pubkey) &&
                    d.authority != Some(main_governance.governance_address) {
                return Err(StateError::InvalidRealmAuthority(realm.realm_address, d.authority).into());
            }
            Ok(None)
        },
        || {
            let transaction = client.create_transaction_with_payer_only(
                &[
                    realm.create_realm_instruction(
                        &wallet.creator_pubkey,
                        &RealmConfig {
                            council_token_mint: None,
                            community_voter_weight_addin: Some(wallet.fixed_weight_addin_id),
                            max_community_voter_weight_addin: Some(wallet.fixed_weight_addin_id),
                            min_community_weight_to_create_governance: 1_000_000 * 1_000_000_000,
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

    // -------------------- Setup multisig record --------- --------------------
    executor.check_and_create_object(&format!("Governance multisig for spl_token {}", neon_multisig),
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
                        &[&main_governance.governance_address, &emergency_governance.governance_address],
                        1,
                    ).unwrap(),
                ],
                &[wallet.get_creator_keypair()?]
            )?;
            Ok(Some(transaction))
        }
    )?;

    // -------------------- Create accounts for token_owner --------------------
    let special_accounts = token_distribution.get_special_accounts();
    for (i, voter_weight) in token_distribution.voter_list.iter().enumerate() {

        let token_owner_record = realm.token_owner_record(&voter_weight.voter);
        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_pubkey, &seed, &spl_token::id())?;

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
                        ).unwrap(),
                        system_instruction::transfer(         // Charge VestingRecord
                            &wallet.payer_keypair.pubkey(),
                            &vesting_addin.find_vesting_account(&vesting_token_account),
                            Rent::default().minimum_balance(vesting_addin.get_vesting_account_size(Lockup::default().get_schedule_size(), true)),
                        ),
                    ]);
                    let transaction = client.create_transaction(
                        &instructions,
                        &[wallet.get_creator_keypair()?]
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
        let token_account_address = Pubkey::create_with_seed(&wallet.creator_pubkey, &seed, &spl_token::id())?;
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
                    &[wallet.get_creator_keypair()?]
                )?;
                Ok(Some(transaction))
            }
        )?;
    }

    // ----------- Fill creator_token_owner record ---------------
    let creator_token_owner = wallet.payer_keypair.pubkey();
    let creator_token_owner_record = realm.token_owner_record(&creator_token_owner);

    // ------------- Setup main governance ------------------------
    executor.check_and_create_object("Main governance", main_governance.get_data()?,
        |_| {Ok(None)},
        || {
            let transaction = client.create_transaction(
                &[
                    main_governance.create_governance_instruction(
                        &wallet.creator_pubkey,
                        &creator_token_owner_record,
                        GovernanceConfig {
                            vote_threshold_percentage: VoteThresholdPercentage::YesVote(16),
                            min_community_weight_to_create_proposal: 3_000 * 1_000_000_000,
                            min_transaction_hold_up_time: 60,
                            max_voting_time: 3*60, //78200,
                            vote_tipping: VoteTipping::Disabled,
                            proposal_cool_off_time: 0,
                            min_council_weight_to_create_proposal: 0,
                        },
                    ),
                ],
                &[wallet.get_creator_keypair()?]
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
                        &wallet.creator_pubkey,
                        &creator_token_owner_record,
                        GovernanceConfig {
                            vote_threshold_percentage: VoteThresholdPercentage::YesVote(90),
                            min_community_weight_to_create_proposal: 1_000_000 * 1_000_000_000,
                            min_transaction_hold_up_time: 0,
                            max_voting_time: 3*60, //78200,
                            vote_tipping: VoteTipping::Disabled,
                            proposal_cool_off_time: 0,
                            min_council_weight_to_create_proposal: 0,
                        },
                    ),
                ],
                &[wallet.get_creator_keypair()?]
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
                    spl_associated_token_account::instruction::create_associated_token_account(
                        &wallet.payer_keypair.pubkey(),
                        &main_governance.governance_address,
                        &wallet.community_pubkey,
                        &spl_token::id(),
                    ),
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
            if d.mint_authority.contains(&wallet.creator_pubkey) {
                let instructions = [
                        spl_token::instruction::set_authority(
                            &spl_token::id(),
                            &wallet.community_pubkey,
                            Some(&neon_multisig),
                            spl_token::instruction::AuthorityType::MintTokens,
                            &wallet.creator_pubkey,
                            &[],
                        ).unwrap()
                    ].to_vec();
                let signers = [wallet.get_creator_keypair()?].to_vec();
                Ok(Some((instructions, signers,)))
            } else if d.mint_authority.contains(&neon_multisig) {
                Ok(None)
            } else {
                Err(StateError::InvalidMintAuthority(wallet.community_pubkey, d.mint_authority).into())
            }
        },
        || {if setup {Err(StateError::MissingMint(wallet.community_pubkey).into())} else {Ok(None)}},
    )?;

    // 2. Realm
    collector.check_and_create_object("Realm authority", realm.get_data()?,
        |d| {
            if d.authority == Some(wallet.creator_pubkey) {
                let instructions = [
                        realm.set_realm_authority_instruction(
                            &wallet.creator_pubkey,
                            Some(&main_governance.governance_address),
                            SetRealmAuthorityAction::SetChecked,
                        )
                    ].to_vec();
                let signers = [wallet.get_creator_keypair()?].to_vec();
                Ok(Some((instructions, signers,)))
            } else if d.authority == Some(main_governance.governance_address) ||
                      d.authority == Some(emergency_governance.governance_address) {
                Ok(None)
            } else {
                Err(StateError::InvalidRealmAuthority(realm.realm_address, d.authority).into())
            }
        },
        || {if setup {Err(StateError::MissingRealm(realm.realm_address).into())} else {Ok(None)}}
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
                if upgrade_authority == Some(wallet.creator_pubkey) {
                    let instructions = [
                            client.set_program_upgrade_authority_instruction(
                                program,
                                &wallet.creator_pubkey,
                                Some(&emergency_governance.governance_address),
                            )?
                        ].to_vec();
                    let signers = [wallet.get_creator_keypair()?].to_vec();
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
fn setup_proposal_upgrade_program(_wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        program: &Pubkey, buffer: &Pubkey
) -> Result<(), ScriptError> {
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
// Delegate vote
// =========================================================================
fn setup_proposal_delegate_vote(_wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        realm_address: &Pubkey, delegate: &Option<Pubkey>,
) -> Result<(), ScriptError> {
    use spl_governance::{
        instruction::set_governance_delegate,
        state::realm::RealmV2,
    };

    let program_id = transaction_inserter.proposal.governance.realm.program_id;
    let owner = transaction_inserter.proposal.governance.governance_address;

    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    let realm_data = client.get_account_data_borsh::<RealmV2>(&program_id, realm_address)?
            .ok_or(StateError::InvalidRealm(*realm_address))?;
    let realm = Realm::new(client, &program_id, &realm_data.name, &realm_data.community_mint);
    let token_owner_record = realm.token_owner_record(&owner);

    executor.check_and_create_object(&format!("Delegated token owner record for governance {}", owner),
        token_owner_record.get_data()?,
        |v| {
            println!("Token owner record: {:?}", v);
            Ok(None)
        },
        || {Err(StateError::MissingTokenOwnerRecord(owner).into())},
    )?;

    transaction_inserter.insert_transaction_checked(
            &format!("Delegate vote to {:?}", delegate),
            vec![
                set_governance_delegate(
                    &program_id,
                    &owner,
                    &realm.realm_address,
                    &realm.community_mint,
                    &owner,
                    delegate,
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

    let proposal_data = client.get_account_data_borsh::<ProposalV2>(&program_id, proposal_address)?
            .ok_or(StateError::InvalidProposal)?;
    let governance_data = client.get_account_data_borsh::<GovernanceV2>(&program_id, &proposal_data.governance)?
            .ok_or(StateError::InvalidProposal)?;
    let realm_data = client.get_account_data_borsh::<RealmV2>(&program_id, &governance_data.realm)?
            .ok_or(StateError::InvalidProposal)?;

    let voted_realm = Realm::new(client, &program_id, &realm_data.name, &realm_data.community_mint);
    voted_realm.update_max_voter_weight_record_address()?;
    let voted_governance = voted_realm.governance(&governance_data.governed_account);
    let voted_proposal = voted_governance.proposal(proposal_address);

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
// Proposal for set transfer authority
// =========================================================================
fn setup_set_transfer_auth(wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        account: &Pubkey, new_auth: &Pubkey,
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    let account_owner_resolver = AccountOwnerResolver::new(wallet, client, MULTI_SIGS);
    let neon_multisig = account_owner_resolver.get_owner_pubkey(&AccountOwner::BothGovernance)?;

    executor.check_and_create_object("Token account", get_account_data(client, account)?,
        |d| {
            assert_is_valid_account_data(d, account, &wallet.community_pubkey, &neon_multisig)?;
            Ok(None)
        },
        || {Err(StateError::MissingSplTokenAccount(*account).into())},
    )?;

    transaction_inserter.insert_transaction_checked(
        &format!("Set transfer auth for {} to {}", account, new_auth),
        vec![
            spl_token::instruction::set_authority(
                &spl_token::id(),
                account,
                Some(new_auth),
                spl_token::instruction::AuthorityType::AccountOwner,
                &neon_multisig, &[&transaction_inserter.proposal.governance.governance_address],
            )?.into(),
        ],
    )?;

    Ok(())
}


// =========================================================================
// Proposal for set mint authority
// =========================================================================
fn setup_set_mint_auth(wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        mint: &Pubkey, new_auth: &Pubkey,
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    let account_owner_resolver = AccountOwnerResolver::new(wallet, client, MULTI_SIGS);
    let neon_multisig = account_owner_resolver.get_owner_pubkey(&AccountOwner::BothGovernance)?;

    executor.check_and_create_object("Token account", get_mint_data(client, mint)?,
        |d| {
            if !d.mint_authority.contains(&neon_multisig) {
                return Err(StateError::InvalidMintAuthority(*mint, d.mint_authority).into());
            }
            Ok(None)
        },
        || {Err(StateError::MissingMint(*mint).into())},
    )?;

    transaction_inserter.insert_transaction_checked(
        &format!("Set mint auth for {} to {}", mint, new_auth),
        vec![
            spl_token::instruction::set_authority(
                &spl_token::id(),
                mint,
                Some(new_auth),
                spl_token::instruction::AuthorityType::MintTokens,
                &neon_multisig, &[&transaction_inserter.proposal.governance.governance_address],
            )?.into(),
        ],
    )?;

    Ok(())
}


// =========================================================================
// Proposal for transfer tokens
// =========================================================================
fn setup_proposal_transfer(wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        from: &Pubkey, to: &Pubkey, amount: u64,
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    let account_owner_resolver = AccountOwnerResolver::new(wallet, client, MULTI_SIGS);
    let neon_multisig = account_owner_resolver.get_owner_pubkey(&AccountOwner::BothGovernance)?;

    executor.check_and_create_object("Token account", get_account_data(client, from)?,
        |d| {
            assert_is_valid_account_data(d, from, &wallet.community_pubkey, &neon_multisig)?;
            Ok(None)
        },
        || {Err(StateError::MissingSplTokenAccount(*from).into())},
    )?;

    transaction_inserter.insert_transaction_checked(
        &format!("Transfer {} to {}", from, to),
        vec![
            spl_token::instruction::transfer(
                &spl_token::id(),
                from, to,
                &neon_multisig, &[&transaction_inserter.proposal.governance.governance_address],
                amount,
            )?.into(),
        ],
    )?;

    Ok(())
}


// =========================================================================
// Create TGE proposal (Token Genesis Event)
// =========================================================================
fn setup_proposal_tge(wallet: &Wallet, client: &Client, transaction_inserter: &mut ProposalTransactionInserter, testing: bool) -> Result<(), ScriptError> {
    let schedule_creator = ScheduleCreator::new(testing);

    let realm = Realm::new(client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    realm.update_max_voter_weight_record_address()?;

    let fixed_weight_addin = AddinFixedWeights::new(client, wallet.fixed_weight_addin_id);
    let vesting_addin = AddinVesting::new(client, wallet.vesting_addin_id);
    let governance = realm.governance(&wallet.community_pubkey);
    let emergency_governance = realm.governance(&wallet.governance_program_id);

    let account_owner_resolver = AccountOwnerResolver::new(wallet, client, MULTI_SIGS);
    let token_distribution = TokenDistribution::new(&fixed_weight_addin, &account_owner_resolver, EXTRA_TOKEN_ACCOUNTS)?;
    token_distribution.validate()?;

    let neon_multisig = account_owner_resolver.get_owner_pubkey(&AccountOwner::BothGovernance)?;
    
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
                    &neon_multisig, &[&governance.governance_address],
                    token_distribution.info.total_amount,
                )?.into(),
            ],
        )?;

    let special_accounts = token_distribution.get_special_accounts();
    println!("Special accounts: {:?}", special_accounts);
    for (i, voter) in token_distribution.voter_list.iter().enumerate() {
        if special_accounts.contains(&voter.voter) {continue;}

        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_pubkey, &seed, &spl_token::id()).unwrap();
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
        let token_account_address = Pubkey::create_with_seed(&wallet.creator_pubkey, &seed, &spl_token::id())?;

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

fn finalize_vote_proposal(_wallet: &Wallet, _client: &Client, proposal: &Proposal, _verbose: bool) -> Result<(), ScriptError> {
    let proposal_data = proposal.get_data()?.ok_or(StateError::InvalidProposalIndex)?;
    proposal.finalize_vote(&proposal_data.token_owner_record)?;

    Ok(())
}

fn sign_off_proposal(wallet: &Wallet, _client: &Client, proposal_owner: &TokenOwner, proposal: &Proposal, _verbose: bool) -> Result<(), ScriptError> {
    let proposal_data = proposal.get_data()?.ok_or(StateError::InvalidProposalIndex)?;
    if proposal_data.state == ProposalState::Draft {
        proposal.sign_off_proposal(&wallet.payer_keypair, proposal_owner)?;
    }

    Ok(())
}

fn approve_proposal(wallet: &Wallet, client: &Client, proposal: &Proposal, _verbose: bool) -> Result<(), ScriptError> {
    use spl_governance::state::vote_record::get_vote_record_address;
    let proposal_data = proposal.get_data()?.ok_or(StateError::InvalidProposalIndex)?;

    for voter in wallet.voter_keypairs.iter() {
        let token_owner = proposal.governance.realm.token_owner_record(&voter.pubkey());
        if (token_owner.get_data()?).is_some() {
            token_owner.update_voter_weight_record_address()?;

            let vote_record_address = get_vote_record_address(
                    &proposal.governance.realm.program_id,
                    &proposal.proposal_address,
                    &token_owner.token_owner_record_address);
            if !client.account_exists(&vote_record_address) {
                let signature = proposal.cast_vote(&proposal_data.token_owner_record, voter, &token_owner, true)?;
                println!("CastVote {} {:?}", voter.pubkey(), signature);
            }
        }
    }

    Ok(())
}

fn execute_proposal(_wallet: &Wallet, _client: &Client, proposal: &Proposal, _verbose: bool) -> Result<(), ScriptError> {
    let result = proposal.execute_transactions(0)?;
    println!("Execute transactions from proposal option 0: {:?}", result);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_proposal_create(wallet: &Wallet, client: &Client, governance: &Governance,
        proposal_info: &ProposalInfo, cmd: &str, cmd_matches: &ArgMatches<'_>, setup: bool, verbose: bool
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {client, setup, verbose};
    let realm = governance.realm;
    let owner_record = {
        let creator = wallet.payer_keypair.pubkey();
        let owner_record = realm.find_owner_or_delegate_record(&creator)?;
        executor.check_and_create_object("Creator owner record", owner_record.as_ref(),
            |record| {
                record.update_voter_weight_record_address()?;
                Ok(None)
            },
            || {
                println_bold!("Can't create proposal because missing token owner record for {}", creator);
                if !setup {println_bold!("But you can create it manually using generated instructions (rerun script with -v)")};
                Err(StateError::MissingTokenOwnerRecord(creator).into())
            },
        )?;
        // If missed correct token owner record for payer, we can setup some
        // record to make other checks (in check mode only!)
        owner_record.unwrap_or_else(|| realm.token_owner_record(&creator))
    };

    let check_proposal = |proposal: &Proposal| {
        executor.check_and_create_object("Proposal", proposal.get_data()?,
            |p| {
                if p.governance != governance.governance_address ||
                   p.governing_token_mint != realm.community_mint
                {
                    return Err(StateError::InvalidProposal.into());
                }
                Ok(None)
            },
            || {Err(StateError::MissingProposal(proposal.proposal_address).into())},
        )
    };
    let proposal = match proposal_info {
        ProposalInfo::Last => {
            let proposal_index = governance.get_proposals_count()-1;
            let proposal = governance.proposal_by_index(proposal_index);
            check_proposal(&proposal)?;
            proposal
        },
        ProposalInfo::Exists(proposal_address) => {
            let proposal = governance.proposal(proposal_address);
            check_proposal(&proposal)?;
            proposal
        },
        ProposalInfo::Create(name, description) => {
            let proposal_index = governance.get_proposals_count();
            let proposal = governance.proposal_by_index(proposal_index);
            executor.check_and_create_object("Proposal", proposal.get_data()?,
                |p| {
                    if p.governance != governance.governance_address ||
                       p.governing_token_mint != realm.community_mint
                    {
                        return Err(StateError::InvalidProposal.into());
                    }
                    Ok(None)
                },
                || {
                    let transaction = client.create_transaction_with_payer_only(
                        &[
                            proposal.create_proposal_instruction(
                                &wallet.payer_keypair.pubkey(),
                                &owner_record,
                                proposal_index, name, description,
                            ),
                        ],
                    )?;
                    Ok(Some(transaction))
                },
            )?;
            proposal
        }
    };
    println_bold!("Proposal: {}, Token owner: {}", proposal.proposal_address, owner_record.token_owner_address);

    let mut transaction_inserter = ProposalTransactionInserter {
        proposal: &proposal,
        creator_keypair: &wallet.payer_keypair,
        creator_token_owner: &owner_record,
        hold_up_time: governance.get_data()?.map(|d| d.config.min_transaction_hold_up_time).unwrap_or(0),
        setup,
        verbose,
        proposal_transaction_index: 0,
    };
    match cmd {
        "create-tge" => setup_proposal_tge(wallet, client, &mut transaction_inserter, true)?,
        "create-empty" => {},
        "create-upgrade-program" => {
            let program: Pubkey = pubkey_of(cmd_matches, "program").unwrap();
            let buffer: Pubkey = pubkey_of(cmd_matches, "buffer").unwrap();
            setup_proposal_upgrade_program(wallet, client, &mut transaction_inserter, &program, &buffer)?
        },
        "create-delegate-vote" => {
            let realm = pubkey_of(cmd_matches, "realm").unwrap();
            let delegate: Option<Pubkey> = pubkey_of(cmd_matches, "delegate");
            setup_proposal_delegate_vote(wallet, client, &mut transaction_inserter, &realm, &delegate)?
        },
        "create-vote-proposal" => {
            let vote_proposal: Pubkey = pubkey_of(cmd_matches, "vote-proposal").unwrap();
            setup_proposal_vote_proposal(wallet, client, &mut transaction_inserter, &vote_proposal)?
        },
        "create-transfer" => {
            let from: Pubkey = pubkey_of(cmd_matches, "from").unwrap();
            let to: Pubkey = pubkey_of(cmd_matches, "to").unwrap();
            let amount = cmd_matches.value_of("amount").map(|v| v.parse::<u64>().unwrap()).unwrap();
            setup_proposal_transfer(wallet, client, &mut transaction_inserter, &from, &to, amount)?
        },
        "create-set-transfer-auth" => {
            let account: Pubkey = pubkey_of(cmd_matches, "account").unwrap();
            let new_auth: Pubkey = pubkey_of(cmd_matches, "new-auth").unwrap();
            setup_set_transfer_auth(wallet, client, &mut transaction_inserter, &account, &new_auth)?
        },
        "create-set-mint-auth" => {
            let mint: Pubkey = pubkey_of(cmd_matches, "mint").unwrap();
            let new_auth: Pubkey = pubkey_of(cmd_matches, "new-auth").unwrap();
            setup_set_mint_auth(wallet, client, &mut transaction_inserter, &mint, &new_auth)?
        },
        _ => unreachable!(),
    }

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
        .arg(
            Arg::with_name("artifacts")
                .long("artifacts")
                .default_value("artifacts")
                .takes_value(true)
                .help("Directory with keypair- or pubkey-files")
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
                    .conflicts_with("proposal")
                    .required(true)
                    .takes_value(true)
                    .value_name("PROPOSAL_NAME")
                    .help("Proposal name")
            )
            .arg(
                Arg::with_name("description")
                    .long("description")
                    .short("d")
                    .conflicts_with("proposal")
                    .takes_value(true)
                    .value_name("PROPOSAL_DESCRIPTION")
                    .help("Proposal description")
            )
            .arg(
                Arg::with_name("proposal")
                    .long("proposal")
                    .short("p")
                    .conflicts_with("name")
                    .required(true)
                    .takes_value(true)
                    .value_name("PROPOSAL_ADDRESS")
                    .help("Proposal address")
            )
            .arg(
                Arg::with_name("governance")
                    .long("governance")
                    .short("g")
                    .default_value("COMMUNITY")
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
                        .required(true)
                        .takes_value(true)
                        .value_name("PROGRAM")
                        .help("Program address")
                )
                .arg(
                    Arg::with_name("buffer")
                        .long("buffer")
                        .short("b")
                        .required(true)
                        .takes_value(true)
                        .value_name("BUFFER")
                        .help("Buffer with new program")
                )
            )
            .subcommand(SubCommand::with_name("create-set-transfer-auth")
                .about("Create proposal for set transfer token authority")
                .arg(
                    Arg::with_name("account")
                        .long("account")
                        .required(true)
                        .takes_value(true)
                        .value_name("ACCOUNT")
                        .help("Token account")
                )
                .arg(
                    Arg::with_name("new-auth")
                        .long("new-auth")
                        .required(true)
                        .takes_value(true)
                        .value_name("NEW_AUTH")
                        .help("New transfer authority")
                )
            )
            .subcommand(SubCommand::with_name("create-set-mint-auth")
                .about("Create proposal for set mint token authority")
                .arg(
                    Arg::with_name("mint")
                        .long("mint")
                        .required(true)
                        .takes_value(true)
                        .value_name("MINT")
                        .help("Token mint")
                )
                .arg(
                    Arg::with_name("new-auth")
                        .long("new-auth")
                        .required(true)
                        .takes_value(true)
                        .value_name("NEW_AUTH")
                        .help("New transfer authority")
                )
            )
            .subcommand(SubCommand::with_name("create-transfer")
                .about("Create proposal for transfer tokens")
                .arg(
                    Arg::with_name("from")
                        .long("from")
                        .required(true)
                        .takes_value(true)
                        .value_name("FROM")
                        .help("From token account")
                )
                .arg(
                    Arg::with_name("to")
                        .long("to")
                        .required(true)
                        .takes_value(true)
                        .value_name("TO")
                        .help("To token account")
                )
                .arg(
                    Arg::with_name("amount")
                        .long("amount")
                        .required(true)
                        .takes_value(true)
                        .value_name("AMOUNT")
                        .help("Transfer token amount")
                )
            )
            .subcommand(SubCommand::with_name("create-delegate-vote")
                .about("Create proposal for delegate vote (token owner record)")
                .arg(
                    Arg::with_name("realm")
                        .long("realm")
                        .required(true)
                        .takes_value(true)
                        .value_name("REALM")
                        .help("Realm in which located token_owner_record (owned by used governance)")
                )
                .arg(
                    Arg::with_name("delegate")
                        .long("delegate")
                        .required(true)
                        .takes_value(true)
                        .validator(is_valid_pubkey_or_none)
                        .value_name("DELEGATE")
                        .help("Delegate account")
                )
            )
            .subcommand(SubCommand::with_name("create-vote-proposal")
                .about("Create proposal for CastVote")
                .arg(
                    Arg::with_name("vote-proposal")
                        .long("vote-proposal")
                        .required(true)
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

    let wallet = Wallet::new(Path::new(matches.value_of("artifacts").unwrap())).unwrap();
    wallet.display();

    let client = Client::new("http://localhost:8899", &wallet.payer_keypair);

    let send_trx: bool = matches.is_present("send_trx");
    let verbose: bool = matches.is_present("verbose");
    let _testing: bool = matches.is_present("testing");
    match matches.subcommand() {
        ("environment", Some(_)) => {
            process_environment(&wallet, &client, send_trx, verbose).unwrap()
        },
        ("proposal", Some(arg_matches)) => {
            let governance_name = arg_matches.value_of("governance").unwrap_or("COMMUNITY");
            let (realm_name, realm_mint, governed_address) = match governance_name {
                "COMMUNITY" => (REALM_NAME, wallet.community_pubkey, wallet.community_pubkey),
                "EMERGENCY" => (REALM_NAME, wallet.community_pubkey, wallet.governance_program_id),
                name if name.starts_with("MSIG_") => {
                    let msig_mint = Pubkey::create_with_seed(&wallet.creator_pubkey, name, &spl_token::id()).unwrap();
                    (name, msig_mint, msig_mint)
                },
                _ => unreachable!(),
            };
            let realm = Realm::new(&client, &wallet.governance_program_id, realm_name, &realm_mint);
            realm.update_max_voter_weight_record_address().unwrap();
            let governance = realm.governance(&governed_address);

            let proposal_info = if let Some("LAST") = arg_matches.value_of("proposal") {
                ProposalInfo::Last
            } else if let Some(proposal) = pubkey_of(arg_matches, "proposal") {
                ProposalInfo::Exists(proposal)
            } else if let Some(name) = value_of(arg_matches, "name") {
                let description = value_of(arg_matches, "description").unwrap_or_else(|| "".to_string());
                ProposalInfo::Create(name, description)
            } else {
                unreachable!()
            };

            match arg_matches.subcommand() {
                (cmd, Some(cmd_matches)) if cmd.starts_with("create-") => {
                    process_proposal_create(&wallet, &client, &governance, &proposal_info, cmd, cmd_matches, send_trx, verbose).unwrap()
                },
                (cmd, _) if ["sign-off", "approve", "finalize-vote", "execute"].contains(&cmd) => {
                    let proposal = match proposal_info {
                        ProposalInfo::Last => {
                            let proposal_index = governance.get_proposals_count() - 1;
                            governance.proposal_by_index(proposal_index)
                        },
                        ProposalInfo::Exists(proposal_address) => {
                            governance.proposal(&proposal_address)
                        },
                        ProposalInfo::Create(_, _) => {
                            unreachable!()
                        },
                    };

                    let owner_record = realm.find_owner_or_delegate_record(&wallet.payer_keypair.pubkey()).unwrap().unwrap();
                    owner_record.update_voter_weight_record_address().unwrap();
                    println!("Owner record: {}", owner_record);

                    match cmd {
                        "sign-off" => sign_off_proposal(&wallet, &client, &owner_record, &proposal, verbose).unwrap(),
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
