//! Vote for proposal

use crate::prelude::*;

pub fn setup_proposal_vote_proposal(
    wallet: &Wallet,
    client: &Client,
    transaction_inserter: &mut ProposalTransactionInserter,
    proposal_address: &Pubkey,
) -> Result<(), ScriptError> {
    use borsh::BorshSerialize;
    use spl_governance::{
        instruction::cast_vote,
        state::{
            governance::GovernanceV2,
            proposal::ProposalV2,
            proposal_transaction::InstructionData,
            realm::RealmV2,
            vote_record::{get_vote_record_address, Vote, VoteChoice},
        },
    };

    let voter = transaction_inserter.proposal.governance.governance_address;
    let program_id = transaction_inserter.proposal.governance.realm.program_id;

    let proposal_data = client
        .get_account_data_borsh::<ProposalV2>(&program_id, proposal_address)?
        .ok_or(StateError::InvalidProposal)?;
    let governance_data = client
        .get_account_data_borsh::<GovernanceV2>(&program_id, &proposal_data.governance)?
        .ok_or(StateError::InvalidProposal)?;
    let realm_data = client
        .get_account_data_borsh::<RealmV2>(&program_id, &governance_data.realm)?
        .ok_or(StateError::InvalidProposal)?;

    let voted_realm = Realm::new(
        client,
        &program_id,
        &realm_data.name,
        &realm_data.community_mint,
    );
    voted_realm.update_max_voter_weight_record_address()?;
    let voted_governance = voted_realm.governance(&governance_data.governed_account);
    let voted_proposal = voted_governance.proposal(proposal_address);

    let voter_token_owner = voted_realm.token_owner_record(&voter);
    voter_token_owner.update_voter_weight_record_address()?;

    let vote_record_address = get_vote_record_address(
        &program_id,
        &voted_proposal.proposal_address,
        &voter_token_owner.token_owner_record_address,
    );

    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    executor.check_and_create_object(
        "VoteRecord for voter",
        client.get_account(&vote_record_address)?,
        |v| {
            println!("Vote record: {:?}", v);
            Ok(None)
        },
        || {
            let lamports =
                Rent::default().minimum_balance(4 + 32 + 32 + 1 + 8 + (4 + 4 + 1 + 1) + 8);
            println_bold!(
                "Charge {} account with {}.{:09} lamports",
                vote_record_address,
                lamports / 1_000_000_000,
                lamports % 1_000_000_000
            );
            let transaction =
                client.create_transaction_with_payer_only(&[system_instruction::transfer(
                    &wallet.payer_keypair.pubkey(),
                    &vote_record_address,
                    lamports,
                )])?;
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
        &voter_token_owner.token_owner_address, // as payer
        voter_token_owner.get_voter_weight_record_address(),
        voted_realm.settings().max_voter_weight_record_address,
        Vote::Approve(vec![VoteChoice {
            rank: 0,
            weight_percentage: 100,
        }]),
    )
    .into();

    println_bold!(
        "Add instruction to proposal: {}",
        base64::encode(instruction.try_to_vec()?)
    );

    transaction_inserter.insert_transaction_checked(
        &format!("Vote proposal {}", proposal_address),
        vec![instruction],
    )?;

    Ok(())
}
