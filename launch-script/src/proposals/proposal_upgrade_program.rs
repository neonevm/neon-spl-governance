//! Upgrade program

use crate::{
    errors::{ScriptError, StateError},
    helpers::{ProposalTransactionInserter, TransactionExecutor},
    wallet::Wallet,
};
use governance_lib::client::Client;
use solana_sdk::{pubkey::Pubkey, signer::Signer};

pub fn setup_proposal_upgrade_program(
    _wallet: &Wallet,
    client: &Client,
    transaction_inserter: &mut ProposalTransactionInserter,
    program: &Pubkey,
    buffer: &Pubkey,
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    executor.check_and_create_object(
        "Program",
        client.get_program_upgrade_authority(program)?,
        |authority| {
            if *authority != transaction_inserter.proposal.governance.governance_address {
                Err(StateError::InvalidProgramUpgradeAuthority(*program, Some(*authority)).into())
            } else {
                Ok(None)
            }
        },
        || Err(StateError::InvalidProgramUpgradeAuthority(*program, None).into()),
    )?;

    transaction_inserter.insert_transaction_checked(
        &format!("Upgrade program {}", program),
        vec![solana_sdk::bpf_loader_upgradeable::upgrade(
            program,
            buffer,
            &transaction_inserter.proposal.governance.governance_address,
            &client.payer.pubkey(),
        )
        .into()],
    )?;

    Ok(())
}
