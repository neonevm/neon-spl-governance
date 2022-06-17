//! Proposal for set mint authority

use crate::{
    errors::{ScriptError, StateError},
    helpers::{ProposalTransactionInserter, TransactionExecutor},
    tokens::get_mint_data,
    wallet::Wallet,
    Configuration,
};
use governance_lib::client::Client;
use solana_sdk::pubkey::Pubkey;

pub fn setup_set_mint_auth(
    _wallet: &Wallet,
    client: &Client,
    transaction_inserter: &mut ProposalTransactionInserter,
    cfg: &Configuration,
    mint: &Pubkey,
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
        get_mint_data(client, mint)?,
        |d| {
            if !d.mint_authority.contains(&neon_multisig) {
                return Err(StateError::InvalidMintAuthority(*mint, d.mint_authority).into());
            }
            Ok(None)
        },
        || Err(StateError::MissingMint(*mint).into()),
    )?;

    transaction_inserter.insert_transaction_checked(
        &format!("Set mint auth for {} to {}", mint, new_auth),
        vec![spl_token::instruction::set_authority(
            &spl_token::id(),
            mint,
            Some(new_auth),
            spl_token::instruction::AuthorityType::MintTokens,
            &neon_multisig,
            &[&transaction_inserter.proposal.governance.governance_address],
        )?
        .into()],
    )?;

    Ok(())
}
