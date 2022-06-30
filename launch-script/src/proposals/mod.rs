mod proposal_delegate_vote;
mod proposal_tge;
mod proposal_transfer;
mod proposal_upgrade_program;
mod proposal_vote_proposal;
mod set_mint_auth;
mod set_transfer_auth;
mod create_treasury_pool;
mod create_upgrade_evm;

pub mod prelude {
    pub use super::approve_proposal;
    pub use super::execute_proposal;
    pub use super::finalize_vote_proposal;
    pub use super::process_proposal_create;
    pub use super::sign_off_proposal;
    pub use super::ProposalInfo;
}

use solana_sdk::signer::keypair::read_keypair_file;

use crate::prelude::*;
use proposal_delegate_vote::setup_proposal_delegate_vote;
use proposal_tge::setup_proposal_tge;
use proposal_transfer::setup_proposal_transfer;
use proposal_upgrade_program::setup_proposal_upgrade_program;
use proposal_vote_proposal::setup_proposal_vote_proposal;
use set_mint_auth::setup_set_mint_auth;
use set_transfer_auth::setup_set_transfer_auth;
use create_treasury_pool::create_collateral_pool_accounts;
use create_upgrade_evm::create_upgrade_evm;

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
        },
        "create-start-evm" => {
            let buffer_pubkey: Pubkey = pubkey_of(cmd_matches, "buffer").unwrap();
            create_collateral_pool_accounts(&mut transaction_inserter, cfg)?;
            create_upgrade_evm(client, &mut transaction_inserter, cfg, buffer_pubkey)?
        },
        _ => unreachable!(),
    }

    Ok(())
}

pub fn finalize_vote_proposal(
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

pub fn sign_off_proposal(
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

pub fn approve_proposal(
    _wallet: &Wallet,
    client: &Client,
    proposal: &Proposal,
    _verbose: bool,
    voters_dir: &str,
) -> Result<(), ScriptError> {
    use spl_governance::state::vote_record::get_vote_record_address;
    let proposal_data = proposal
        .get_data()?
        .ok_or(StateError::InvalidProposalIndex)?;

    let voter_keypairs = {
        let mut voter_keypairs = vec!();
        for file in Path::new(voters_dir).read_dir()
            .map_err(|err| StateError::ConfigError(format!("'{}' should be a directory: {}", voters_dir, err)))?
        {
            let path = file?.path();
            match read_keypair_file(path.clone()) {
                Ok(keypair) => voter_keypairs.push(keypair),
                Err(err) => println!("Skip '{}' due to {}", path.display(), err),
            };
        }
        voter_keypairs
    };

    for voter in voter_keypairs.iter() {
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

pub fn execute_proposal(
    _wallet: &Wallet,
    _client: &Client,
    proposal: &Proposal,
    _verbose: bool,
) -> Result<(), ScriptError> {
    let result = proposal.execute_transactions(0)?;
    println!("Execute transactions from proposal option 0: {:?}", result);

    Ok(())
}
