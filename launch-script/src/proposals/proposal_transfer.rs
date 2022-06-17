//! Proposal for transfer tokens

use crate::prelude::*;

pub fn setup_proposal_transfer(
    wallet: &Wallet,
    client: &Client,
    transaction_inserter: &mut ProposalTransactionInserter,
    cfg: &Configuration,
    from: &Pubkey,
    to: &Pubkey,
    amount: u64,
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    let neon_multisig = cfg.neon_multisig_address();

    executor.check_and_create_object(
        "Token account",
        get_account_data(client, from)?,
        |d| {
            assert_is_valid_account_data(d, from, &wallet.community_pubkey, &neon_multisig)?;
            Ok(None)
        },
        || Err(StateError::MissingSplTokenAccount(*from).into()),
    )?;

    transaction_inserter.insert_transaction_checked(
        &format!("Transfer {} to {}", from, to),
        vec![spl_token::instruction::transfer(
            &spl_token::id(),
            from,
            to,
            &neon_multisig,
            &[&transaction_inserter.proposal.governance.governance_address],
            amount,
        )?
        .into()],
    )?;

    Ok(())
}
