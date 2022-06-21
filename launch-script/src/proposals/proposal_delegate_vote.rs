//! Delegate vote

use crate::prelude::*;

pub fn setup_proposal_delegate_vote(
    _wallet: &Wallet,
    client: &Client,
    transaction_inserter: &mut ProposalTransactionInserter,
    realm_address: &Pubkey,
    delegate: &Option<Pubkey>,
) -> Result<(), ScriptError> {
    use spl_governance::{instruction::set_governance_delegate, state::realm::RealmV2};

    let program_id = transaction_inserter.proposal.governance.realm.program_id;
    let owner = transaction_inserter.proposal.governance.governance_address;

    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    let realm_data = client
        .get_account_data_borsh::<RealmV2>(&program_id, realm_address)?
        .ok_or(StateError::InvalidRealm(*realm_address))?;
    let realm = Realm::new(
        client,
        &program_id,
        &realm_data.name,
        &realm_data.community_mint,
    );
    let token_owner_record = realm.token_owner_record(&owner);

    executor.check_and_create_object(
        &format!("Delegated token owner record for governance {}", owner),
        token_owner_record.get_data()?,
        |v| {
            println!("Token owner record: {:?}", v);
            Ok(None)
        },
        || Err(StateError::MissingTokenOwnerRecord(owner).into()),
    )?;

    transaction_inserter.insert_transaction_checked(
        &format!("Delegate vote to {:?}", delegate),
        vec![set_governance_delegate(
            &program_id,
            &owner,
            &realm.realm_address,
            &realm.community_mint,
            &owner,
            delegate,
        )
        .into()],
    )?;

    Ok(())
}
