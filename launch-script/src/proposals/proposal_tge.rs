//! Create TGE proposal (Token Genesis Event)

use crate::{
    errors::ScriptError, helpers::ProposalTransactionInserter, lockup::Lockup, wallet::Wallet,
    Configuration, REALM_NAME,
};
use governance_lib::{addin_vesting::AddinVesting, client::Client, realm::Realm};
use spl_governance::state::realm::SetRealmAuthorityAction;

pub fn setup_proposal_tge(
    wallet: &Wallet,
    client: &Client,
    transaction_inserter: &mut ProposalTransactionInserter,
    cfg: &Configuration,
) -> Result<(), ScriptError> {
    let realm = Realm::new(
        client,
        &wallet.governance_program_id,
        REALM_NAME,
        &wallet.community_pubkey,
    );
    realm.update_max_voter_weight_record_address()?;

    let vesting_addin = AddinVesting::new(client, wallet.vesting_addin_id);
    let governance = realm.governance(&wallet.community_pubkey);
    let emergency_governance = realm.governance(&wallet.governance_program_id);
    let token_distribution = cfg.get_token_distribution()?;
    let neon_multisig = cfg.neon_multisig_address();

    let governance_token_account =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &governance.governance_address,
            &wallet.community_pubkey,
            &spl_token::id(),
        );
    println!("Governance address: {}", governance.governance_address);
    println!("Governance token account: {}", governance_token_account);

    transaction_inserter.insert_transaction_checked(
        "Mint tokens",
        vec![spl_token::instruction::mint_to(
            &spl_token::id(),
            &wallet.community_pubkey,
            &governance_token_account,
            &neon_multisig,
            &[&governance.governance_address],
            token_distribution.info.total_amount,
        )?
        .into()],
    )?;

    let special_accounts = token_distribution.get_special_accounts();
    println!("Special accounts: {:?}", special_accounts);
    for (i, voter) in token_distribution.voter_list.iter().enumerate() {
        if special_accounts.contains(&voter.voter) {
            continue;
        }

        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = cfg.account_by_seed(&seed, &spl_token::id());
        let lockup = Lockup::default();
        let schedule = cfg.get_schedule(&lockup, voter.weight);

        transaction_inserter.insert_transaction_checked(
            &format!(
                "Deposit {} to {} on token account {}",
                voter.weight, voter.voter, vesting_token_account
            ),
            vec![vesting_addin
                .deposit_with_realm_instruction(
                    &governance.governance_address,      // source_token_authority
                    &governance_token_account,           // source_token_account
                    &voter.voter,                        // vesting_owner
                    &vesting_token_account,              // vesting_token_account
                    schedule,                            // schedule
                    &realm,                              // realm
                    Some(governance.governance_address), // payer
                )?
                .into()],
        )?;
    }

    for (i, token_account) in token_distribution.extra_token_accounts().iter().enumerate() {
        let seed: String = format!("{}_account_{}", REALM_NAME, i);
        let token_account_address = cfg.account_by_seed(&seed, &spl_token::id());

        if token_account.lockup.is_locked() {
            let token_account_owner = cfg.get_owner_address(&token_account.owner)?;
            let schedule = cfg.get_schedule(&token_account.lockup, token_account.amount);

            transaction_inserter.insert_transaction_checked(
                &format!(
                    "Deposit {} to {} on token account {}",
                    token_account.amount, token_account_owner, token_account_address
                ),
                vec![vesting_addin
                    .deposit_with_realm_instruction(
                        &governance.governance_address,      // source_token_authority
                        &governance_token_account,           // source_token_account
                        &token_account_owner,                // vesting_owner
                        &token_account_address,              // vesting_token_account
                        schedule,                            // schedule
                        &realm,                              // realm
                        Some(governance.governance_address), // payer
                    )?
                    .into()],
            )?;
        } else {
            transaction_inserter.insert_transaction_checked(
                &format!(
                    "Transfer {} to {} ({})",
                    token_account.amount, token_account_address, seed
                ),
                vec![spl_token::instruction::transfer(
                    &spl_token::id(),
                    &governance_token_account,
                    &token_account_address,
                    &governance.governance_address,
                    &[],
                    token_account.amount,
                )?
                .into()],
            )?;
        }
    }

    transaction_inserter.insert_transaction_checked(
        "Change to Vesting addin",
        vec![realm
            .set_realm_config_instruction(
                &governance.governance_address, // we already passed realm under governance
                &cfg.working_realm_config,
                Some(governance.governance_address), // payer
            )
            .into()],
    )?;

    transaction_inserter.insert_transaction_checked(
        "Pass Realm under Emergency-governance",
        vec![realm
            .set_realm_authority_instruction(
                &governance.governance_address,
                Some(&emergency_governance.governance_address),
                SetRealmAuthorityAction::SetChecked,
            )
            .into()],
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
