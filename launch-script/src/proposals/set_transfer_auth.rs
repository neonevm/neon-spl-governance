//! Proposal for set transfer authority

use crate::prelude::*;

pub fn setup_set_transfer_auth(
    wallet: &Wallet,
    client: &Client,
    transaction_inserter: &mut ProposalTransactionInserter,
    cfg: &Configuration,
    account: &Pubkey,
    new_auth: &Pubkey,
) -> Result<(), ScriptError> {
    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    let neon_multisig = cfg.neon_multisig_address();

    executor.check_and_create_object(
        "Token account",
        get_account_data(client, account)?,
        |d| {
            assert_is_valid_account_data(d, account, &wallet.community_pubkey, &neon_multisig)?;
            Ok(None)
        },
        || Err(StateError::MissingSplTokenAccount(*account).into()),
    )?;

    transaction_inserter.insert_transaction_checked(
        &format!("Set transfer auth for {} to {}", account, new_auth),
        vec![spl_token::instruction::set_authority(
            &spl_token::id(),
            account,
            Some(new_auth),
            spl_token::instruction::AuthorityType::AccountOwner,
            &neon_multisig,
            &[&transaction_inserter.proposal.governance.governance_address],
        )?
        .into()],
    )?;

    Ok(())
}
