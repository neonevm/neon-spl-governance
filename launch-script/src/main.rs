mod errors;
mod tokens;
mod wallet;
#[macro_use]
mod helpers;
mod clap_utils;
mod config;
mod lockup;
mod msig;
mod process;
mod proposals;
mod token_distribution;

use crate::{
    clap_utils::is_valid_pubkey_or_none,
    errors::{ScriptError, StateError},
    lockup::Lockup,
    wallet::Wallet,
};
use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg, SubCommand};
pub use config::Configuration;
use governance_lib::{client::Client, proposal::Proposal, realm::Realm, token_owner::TokenOwner};
use process::process_environment_dao;
use proposals::{process_proposal_create, ProposalInfo};
use solana_clap_utils::input_parsers::{pubkey_of, value_of};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use spl_governance::state::enums::ProposalState;
use std::path::Path;

pub const TOKEN_MULT: u64 = u64::pow(10, 9);
pub const REALM_NAME: &str = "NEON";

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum AccountOwner {
    MainGovernance,
    EmergencyGovernance,
    BothGovernance,
    MultiSig(&'static str, Option<Pubkey>),
    Key(Pubkey),
}

pub struct ExtraTokenAccount {
    pub owner: AccountOwner,
    pub amount: u64,
    pub lockup: Lockup,
}

impl ExtraTokenAccount {
    const fn new(amount: u64, lockup: Lockup, owner: AccountOwner) -> Self {
        Self {
            amount,
            lockup,
            owner,
        }
    }
}

fn finalize_vote_proposal(
    _wallet: &Wallet,
    _client: &Client,
    proposal: &Proposal,
    _verbose: bool,
) -> Result<(), ScriptError> {
    let proposal_data = proposal
        .get_data()?
        .ok_or(StateError::InvalidProposalIndex)?;
    proposal.finalize_vote(&proposal_data.token_owner_record)?;

    Ok(())
}

fn sign_off_proposal(
    wallet: &Wallet,
    _client: &Client,
    proposal_owner: &TokenOwner,
    proposal: &Proposal,
    _verbose: bool,
) -> Result<(), ScriptError> {
    let proposal_data = proposal
        .get_data()?
        .ok_or(StateError::InvalidProposalIndex)?;
    if proposal_data.state == ProposalState::Draft {
        proposal.sign_off_proposal(&wallet.payer_keypair, proposal_owner)?;
    }

    Ok(())
}

fn approve_proposal(
    wallet: &Wallet,
    client: &Client,
    proposal: &Proposal,
    _verbose: bool,
) -> Result<(), ScriptError> {
    use spl_governance::state::vote_record::get_vote_record_address;
    let proposal_data = proposal
        .get_data()?
        .ok_or(StateError::InvalidProposalIndex)?;

    for voter in wallet.voter_keypairs.iter() {
        let token_owner = proposal
            .governance
            .realm
            .token_owner_record(&voter.pubkey());
        if (token_owner.get_data()?).is_some() {
            token_owner.update_voter_weight_record_address()?;

            let vote_record_address = get_vote_record_address(
                &proposal.governance.realm.program_id,
                &proposal.proposal_address,
                &token_owner.token_owner_record_address,
            );
            if !client.account_exists(&vote_record_address) {
                let signature = proposal.cast_vote(
                    &proposal_data.token_owner_record,
                    voter,
                    &token_owner,
                    true,
                )?;
                println!("CastVote {} {:?}", voter.pubkey(), signature);
            }
        }
    }

    Ok(())
}

fn execute_proposal(
    _wallet: &Wallet,
    _client: &Client,
    proposal: &Proposal,
    _verbose: bool,
) -> Result<(), ScriptError> {
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
        .arg(
            Arg::with_name("url")
                .long("url")
                .short("u")
                .takes_value(true)
                .global(true)
                .default_value("http://localhost:8899")
                .help("Url to solana cluster")
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
            .subcommand(SubCommand::with_name("dao")
                .about("Prepare environment for DAO")
            )
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

    let url = matches.value_of("url").unwrap();
    let client = Client::new(url, &wallet.payer_keypair);

    let send_trx: bool = matches.is_present("send_trx");
    let verbose: bool = matches.is_present("verbose");
    let testing: bool = matches.is_present("testing");
    // TODO: parse `start_time`
    let cfg = Configuration::create(&wallet, &client, send_trx, verbose, testing, None);

    match matches.subcommand() {
        ("environment", Some(arg_matches)) => {
            let (cmd, _) = arg_matches.subcommand();
            match cmd {
                "dao" => process_environment_dao(&wallet, &client, &cfg).unwrap(),
                _ => unreachable!(),
            }
        }
        ("proposal", Some(arg_matches)) => {
            let governance_name = arg_matches.value_of("governance").unwrap_or("COMMUNITY");
            let (realm_name, realm_mint, governed_address) = match governance_name {
                "COMMUNITY" => (REALM_NAME, wallet.community_pubkey, wallet.community_pubkey),
                "EMERGENCY" => (
                    REALM_NAME,
                    wallet.community_pubkey,
                    wallet.governance_program_id,
                ),
                name if name.starts_with("MSIG_") => {
                    if name.contains('.') {
                        let (msig_name, governed) = name.split_once('.').unwrap();
                        let msig_mint = cfg.account_by_seed(msig_name, &spl_token::id());
                        (msig_name, msig_mint, Pubkey::try_from(governed).unwrap())
                    } else {
                        let msig_mint = cfg.account_by_seed(name, &spl_token::id());
                        (name, msig_mint, msig_mint)
                    }
                }
                _ => unreachable!(),
            };
            let realm = Realm::new(
                &client,
                &wallet.governance_program_id,
                realm_name,
                &realm_mint,
            );
            realm.update_max_voter_weight_record_address().unwrap();
            let governance = realm.governance(&governed_address);

            let proposal_info = if let Some("LAST") = arg_matches.value_of("proposal") {
                ProposalInfo::Last
            } else if let Some(proposal) = pubkey_of(arg_matches, "proposal") {
                ProposalInfo::Exists(proposal)
            } else if let Some(name) = value_of(arg_matches, "name") {
                let description =
                    value_of(arg_matches, "description").unwrap_or_else(|| "".to_string());
                ProposalInfo::Create(name, description)
            } else {
                unreachable!()
            };

            match arg_matches.subcommand() {
                (cmd, Some(cmd_matches)) if cmd.starts_with("create-") => process_proposal_create(
                    &wallet,
                    &client,
                    &governance,
                    &proposal_info,
                    cmd,
                    cmd_matches,
                    &cfg,
                )
                .unwrap(),
                (cmd, _) if ["sign-off", "approve", "finalize-vote", "execute"].contains(&cmd) => {
                    let proposal = match proposal_info {
                        ProposalInfo::Last => {
                            let proposal_index = governance.get_proposals_count().unwrap() - 1;
                            governance.proposal_by_index(proposal_index)
                        }
                        ProposalInfo::Exists(proposal_address) => {
                            governance.proposal(&proposal_address)
                        }
                        ProposalInfo::Create(_, _) => {
                            unreachable!()
                        }
                    };

                    let owner_record = realm
                        .find_owner_or_delegate_record(&wallet.payer_keypair.pubkey())
                        .unwrap()
                        .unwrap();
                    owner_record.update_voter_weight_record_address().unwrap();
                    println!("Owner record: {}", owner_record);

                    match cmd {
                        "sign-off" => {
                            sign_off_proposal(&wallet, &client, &owner_record, &proposal, verbose)
                                .unwrap()
                        }
                        "approve" => {
                            approve_proposal(&wallet, &client, &proposal, verbose).unwrap()
                        }
                        "finalize-vote" => {
                            finalize_vote_proposal(&wallet, &client, &proposal, verbose).unwrap()
                        }
                        "execute" => {
                            execute_proposal(&wallet, &client, &proposal, verbose).unwrap()
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}
