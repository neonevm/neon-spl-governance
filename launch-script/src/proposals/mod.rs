mod proposal_delegate_vote;
mod proposal_tge;
mod proposal_transfer;
mod proposal_upgrade_program;
mod proposal_vote_proposal;
mod set_mint_auth;
mod set_transfer_auth;

use proposal_delegate_vote::setup_proposal_delegate_vote;
use proposal_tge::setup_proposal_tge;
use proposal_transfer::setup_proposal_transfer;
use proposal_upgrade_program::setup_proposal_upgrade_program;
use proposal_vote_proposal::setup_proposal_vote_proposal;
use set_mint_auth::setup_set_mint_auth;
use set_transfer_auth::setup_set_transfer_auth;

use crate::{
    errors::{ScriptError, StateError},
    helpers::{ProposalTransactionInserter, TransactionExecutor},
    wallet::Wallet,
    Configuration,
};
use clap::ArgMatches;
use governance_lib::{client::Client, governance::Governance, proposal::Proposal};
use solana_clap_utils::input_parsers::pubkey_of;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

pub enum ProposalInfo {
    Create(String, String), // name, description
    Exists(Pubkey),         // proposal address
    Last,                   // last proposal in governance
}

#[allow(clippy::too_many_arguments)]
pub fn process_proposal_create(
    wallet: &Wallet,
    client: &Client,
    governance: &Governance,
    proposal_info: &ProposalInfo,
    cmd: &str,
    cmd_matches: &ArgMatches<'_>,
    cfg: &Configuration,
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {
        client,
        setup: cfg.send_trx,
        verbose: cfg.verbose,
    };
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
                                             if !cfg.send_trx {println_bold!("But you can create it manually using generated instructions (rerun script with -v)")};
                                             Err(StateError::MissingTokenOwnerRecord(creator).into())
                                         },
        )?;
        // If missed correct token owner record for payer, we can setup some
        // record to make other checks (in check mode only!)
        owner_record.unwrap_or_else(|| realm.token_owner_record(&creator))
    };

    let check_proposal = |proposal: &Proposal| {
        executor.check_and_create_object(
            "Proposal",
            proposal.get_data()?,
            |p| {
                if p.governance != governance.governance_address
                    || p.governing_token_mint != realm.community_mint
                {
                    return Err(StateError::InvalidProposal.into());
                }
                Ok(None)
            },
            || Err(StateError::MissingProposal(proposal.proposal_address).into()),
        )
    };
    let proposal = match proposal_info {
        ProposalInfo::Last => {
            let proposal_index = governance.get_proposals_count()? - 1;
            let proposal = governance.proposal_by_index(proposal_index);
            check_proposal(&proposal)?;
            proposal
        }
        ProposalInfo::Exists(proposal_address) => {
            let proposal = governance.proposal(proposal_address);
            check_proposal(&proposal)?;
            proposal
        }
        ProposalInfo::Create(name, description) => {
            let proposal_index = governance.get_proposals_count()?;
            let proposal = governance.proposal_by_index(proposal_index);
            executor.check_and_create_object(
                "Proposal",
                proposal.get_data()?,
                |p| {
                    if p.governance != governance.governance_address
                        || p.governing_token_mint != realm.community_mint
                    {
                        return Err(StateError::InvalidProposal.into());
                    }
                    Ok(None)
                },
                || {
                    let transaction = client.create_transaction_with_payer_only(&[proposal
                        .create_proposal_instruction(
                            &wallet.payer_keypair.pubkey(),
                            &owner_record,
                            proposal_index,
                            name,
                            description,
                        )])?;
                    Ok(Some(transaction))
                },
            )?;
            proposal
        }
    };
    println_bold!(
        "Proposal: {}, Token owner: {}",
        proposal.proposal_address,
        owner_record.token_owner_address
    );

    let mut transaction_inserter = ProposalTransactionInserter {
        proposal: &proposal,
        creator_keypair: &wallet.payer_keypair,
        creator_token_owner: &owner_record,
        hold_up_time: governance
            .get_data()?
            .map(|d| d.config.min_transaction_hold_up_time)
            .unwrap_or(0),
        setup: cfg.send_trx,
        verbose: cfg.verbose,
        proposal_transaction_index: 0,
    };
    match cmd {
        "create-tge" => setup_proposal_tge(wallet, client, &mut transaction_inserter, cfg)?,
        "create-empty" => {}
        "create-upgrade-program" => {
            let program: Pubkey = pubkey_of(cmd_matches, "program").unwrap();
            let buffer: Pubkey = pubkey_of(cmd_matches, "buffer").unwrap();
            setup_proposal_upgrade_program(
                wallet,
                client,
                &mut transaction_inserter,
                &program,
                &buffer,
            )?
        }
        "create-delegate-vote" => {
            let realm = pubkey_of(cmd_matches, "realm").unwrap();
            let delegate: Option<Pubkey> = pubkey_of(cmd_matches, "delegate");
            setup_proposal_delegate_vote(
                wallet,
                client,
                &mut transaction_inserter,
                &realm,
                &delegate,
            )?
        }
        "create-vote-proposal" => {
            let vote_proposal: Pubkey = pubkey_of(cmd_matches, "vote-proposal").unwrap();
            setup_proposal_vote_proposal(wallet, client, &mut transaction_inserter, &vote_proposal)?
        }
        "create-transfer" => {
            let from: Pubkey = pubkey_of(cmd_matches, "from").unwrap();
            let to: Pubkey = pubkey_of(cmd_matches, "to").unwrap();
            let amount = cmd_matches
                .value_of("amount")
                .map(|v| v.parse::<u64>().unwrap())
                .unwrap();
            setup_proposal_transfer(
                wallet,
                client,
                &mut transaction_inserter,
                cfg,
                &from,
                &to,
                amount,
            )?
        }
        "create-set-transfer-auth" => {
            let account: Pubkey = pubkey_of(cmd_matches, "account").unwrap();
            let new_auth: Pubkey = pubkey_of(cmd_matches, "new-auth").unwrap();
            setup_set_transfer_auth(
                wallet,
                client,
                &mut transaction_inserter,
                cfg,
                &account,
                &new_auth,
            )?
        }
        "create-set-mint-auth" => {
            let mint: Pubkey = pubkey_of(cmd_matches, "mint").unwrap();
            let new_auth: Pubkey = pubkey_of(cmd_matches, "new-auth").unwrap();
            setup_set_mint_auth(
                wallet,
                client,
                &mut transaction_inserter,
                cfg,
                &mint,
                &new_auth,
            )?
        }
        _ => unreachable!(),
    }

    Ok(())
}
