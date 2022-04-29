mod errors;
mod tokens;
mod wallet;
mod helpers;
mod msig;
mod token_distribution;

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
    governance::Governance,
    proposal::Proposal,
    addin_fixed_weights::{VoterWeight, AddinFixedWeights},
    addin_vesting::AddinVesting,
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

#[derive(Debug,PartialEq)]
pub enum Lockup {
    NoLockup,
    For4Years,
    For1Year_1YearLinear,
}

impl Lockup {
    pub fn is_locked(&self) -> bool {*self != Lockup::NoLockup}
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

        executor.check_and_create_object(&seed, token_owner_record.get_data()?,
            |_| {
                // TODO check that all accounts needed to this owner created correctly
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
                            Rent::default().minimum_balance(vesting_addin.get_vesting_account_size(1, true)),
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
                            Rent::default().minimum_balance(vesting_addin.get_vesting_account_size(1, true)),
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
// Create IDO proposal
// =========================================================================
fn setup_proposal_ido(wallet: &Wallet, client: &Client, proposal_index: Option<u32>, setup: bool, verbose: bool) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {client, setup, verbose};

    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    realm.update_max_voter_weight_record_address()?;

    let fixed_weight_addin = AddinFixedWeights::new(&client, wallet.fixed_weight_addin_id);
    let governance = realm.governance(&wallet.community_pubkey);

    let creator_token_owner = realm.token_owner_record(&wallet.creator_token_owner_keypair.pubkey());
    creator_token_owner.update_voter_weight_record_address()?;

    let governance_proposal_count = governance.get_proposals_count();
    let proposal_number = proposal_index.unwrap_or(governance_proposal_count);
    if proposal_number > governance_proposal_count {return Err(StateError::InvalidProposalIndex.into());}
    println!("Use {} for proposal_index", proposal_number);

    let proposal: Proposal = governance.proposal(proposal_number);
    
    executor.check_and_create_object("Proposal IDO", proposal.get_data()?,
        |_| {Ok(None)},
        || {
            let transaction = client.create_transaction(
                &[
                    proposal.create_proposal_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &creator_token_owner,
                        &format!("{} {}", "IDO proposal", proposal_number),
                        "Proposal for IDO (delegate right to start IDO)",
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
    let governance = realm.governance(&wallet.community_pubkey);
    let emergency_governance = realm.governance(&wallet.governance_program_id);

    let account_owner_resolver = AccountOwnerResolver::new(wallet, client, MULTI_SIGS);
    let token_distribution = TokenDistribution::new(&fixed_weight_addin, &account_owner_resolver, EXTRA_TOKEN_ACCOUNTS)?;
    token_distribution.validate()?;

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
            // TODO Calculate schedule
            let schedule = vec!(VestingSchedule { release_time: 0, amount: token_account.amount });

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

fn finalize_vote_proposal(wallet: &Wallet, client: &Client, proposal_index: Option<u32>, verbose: bool) -> Result<(), ScriptError> {
    let realm = Realm::new(&client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    realm.update_max_voter_weight_record_address()?;
    let governance = realm.governance(&wallet.community_pubkey);

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
    let governance = realm.governance(&wallet.community_pubkey);

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

    let governance = realm.governance(&wallet.community_pubkey);
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
    let governance = realm.governance(&wallet.community_pubkey);

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
            .subcommand(SubCommand::with_name("create-ido")
                .about("Create Proposal for IDO")
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

    //let client = Client::new("http://localhost:8899", &wallet.payer_keypair);
    let client = Client::new("https://api.devnet.solana.com", &wallet.payer_keypair);

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
                ("create-ido", Some(arg_matches)) => {
                    setup_proposal_ido(&wallet, &client, proposal_index, send_trx, verbose).unwrap()
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
