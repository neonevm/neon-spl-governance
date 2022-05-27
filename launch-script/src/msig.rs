use crate::{
    tokens::{get_mint_data, get_account_data, create_mint_instructions},
    errors::{StateError, ScriptError},
    wallet::Wallet,
    helpers::{
        TransactionExecutor,
        TransactionCollector,
        ProposalTransactionInserter,
    },
};
use solana_sdk::{
    pubkey::Pubkey,
    signer::{
        Signer,
        keypair::Keypair,
    },
    system_instruction,
    rent::Rent,
};

use spl_governance::{
    state::{
        enums::{
            MintMaxVoteWeightSource,
            VoteThresholdPercentage,
            VoteTipping,
            ProposalState,
        },
        governance::GovernanceConfig,
        realm::SetRealmAuthorityAction,
    },
};

use governance_lib::{
    client::Client,
    realm::{RealmConfig, Realm},
    governance::Governance,
    proposal::Proposal,
    addin_fixed_weights::AddinFixedWeights,
    addin_vesting::AddinVesting,
};

use spl_governance_addin_vesting::state::VestingSchedule;


#[derive(Debug)]
pub struct MultiSig {
    pub name: &'static str,
    pub threshold: u16,
    pub signers: &'static [Pubkey],
}


pub fn setup_msig(wallet: &Wallet, client: &Client, executor: &TransactionExecutor, msig: &MultiSig) -> Result<Pubkey, ScriptError>
{
    let seed: String = format!("MSIG_{}", msig.name);
    let msig_mint = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;
    let msig_realm = Realm::new(&client, &wallet.governance_program_id, &seed, &msig_mint);
    let msig_governance = msig_realm.governance(&msig_mint);

    let vesting_addin = AddinVesting::new(client, wallet.vesting_addin_id);

    // Workaround to create governance from realm creator
    let mut creator_token_owner = msig_realm.token_owner_record(&wallet.creator_keypair.pubkey());
    creator_token_owner.set_voter_weight_record_address(Some(Pubkey::default()));

    println!("- '{}', threshold {}/{}, mint {}, governance {}",
            msig.name, msig.threshold, msig.signers.len(),
            msig_mint, msig_governance.governance_address);
    println!("\tsigners: {:?}", msig.signers);

    // ----------- Check or create multi_sig mint ----------------------
    let creator_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
            &wallet.creator_keypair.pubkey(), &msig_mint, &spl_token::id());
    executor.check_and_create_object(&format!("{} Mint '{}'", msig.name, msig_mint),
        get_mint_data(client, &msig_mint)?,
        |d| {
            if !d.mint_authority.contains(&wallet.creator_keypair.pubkey()) &&
                    !d.mint_authority.contains(&msig_governance.governance_address) {
                return Err(StateError::InvalidMintAuthority(msig_mint, d.mint_authority).into());
            }
            Ok(None)
        },
        || {
            let transaction = client.create_transaction(
                &[
                    system_instruction::create_account_with_seed(
                        &wallet.payer_keypair.pubkey(),              // from
                        &msig_mint,                                  // to
                        &wallet.creator_keypair.pubkey(),            // base
                        &seed,                                       // seed
                        Rent::default().minimum_balance(82),         // lamports
                        82,                                          // space
                        &spl_token::id(),                            // owner
                    ),
                    spl_token::instruction::initialize_mint(
                        &spl_token::id(),
                        &msig_mint,
                        &wallet.creator_keypair.pubkey(),
                        None,
                        0,
                    )?,
                    spl_associated_token_account::create_associated_token_account(
                        &wallet.payer_keypair.pubkey(),
                        &wallet.creator_keypair.pubkey(),
                        &msig_mint,
                    ).into(),
                    spl_token::instruction::mint_to(
                        &spl_token::id(),
                        &msig_mint,
                        &creator_token_account,
                        &wallet.creator_keypair.pubkey(), &[],
                        msig.signers.len() as u64,
                    )?.into(),
                ],
                &[&wallet.creator_keypair],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // -------------- Check or create Realm ---------------------------
    executor.check_and_create_object(&format!("{} Realm '{}'", msig.name, msig_realm.realm_address),
        msig_realm.get_data()?,
        |d| {
            if d.community_mint != msig_realm.community_mint {
                return Err(StateError::InvalidRealmCommunityMint(msig_realm.realm_address, d.community_mint).into());
            }
            if d.authority != Some(wallet.creator_keypair.pubkey()) &&
                    d.authority != Some(msig_governance.governance_address) {
                return Err(StateError::InvalidRealmAuthority(msig_realm.realm_address, d.authority).into());
            }
            Ok(None)
        },
        || {
            let transaction = client.create_transaction_with_payer_only(
                &[
                    msig_realm.create_realm_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &RealmConfig {
                            council_token_mint: None,
                            community_voter_weight_addin: Some(wallet.vesting_addin_id),
                            max_community_voter_weight_addin: None,
                            min_community_weight_to_create_governance: 1,            // TODO Verify parameters!
                            community_mint_max_vote_weight_source: MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                        },
                    ),
                ],
            )?;
            Ok(Some(transaction))
        },
    )?;

    // -------------------- Create accounts for token_owner --------------------
    for (i, owner) in msig.signers.iter().enumerate() {
        let token_owner_record = msig_realm.token_owner_record(&owner);
        let seed: String = format!("{}_msig_{}", msig.name, i);
        let vesting_token_account = Pubkey::create_with_seed(&wallet.creator_keypair.pubkey(), &seed, &spl_token::id())?;
        let schedule = vec!(VestingSchedule { release_time: 0, amount: 1 });

        executor.check_and_create_object(&format!("{} owner {}", msig.name, owner),
            token_owner_record.get_data()?,
            |_| {
                // TODO check that all accounts needed to this owner created correctly
                Ok(None)
            },
            || {
                let transaction = client.create_transaction(
                    &[
                        token_owner_record.create_token_owner_record_instruction(),
                        system_instruction::create_account_with_seed(
                            &wallet.payer_keypair.pubkey(),       // from
                            &vesting_token_account,               // to
                            &wallet.creator_keypair.pubkey(),     // base
                            &seed,                                // seed
                            Rent::default().minimum_balance(165), // lamports
                            165,                                  // space
                            &spl_token::id(),                     // owner
                        ),
                        spl_token::instruction::initialize_account(
                            &spl_token::id(),
                            &vesting_token_account,
                            &msig_mint,
                            &vesting_addin.find_vesting_account(&vesting_token_account),
                        ).unwrap(),
                        vesting_addin.deposit_with_realm_instruction(
                            &wallet.creator_keypair.pubkey(),  // source_token_authority
                            &creator_token_account,            // source_token_account
                            &owner,                            // vesting_owner
                            &vesting_token_account,            // vesting_token_account
                            schedule,                          // schedule
                            &msig_realm,                       // realm
                            None,                              // default payer
                        )?.into(),
                    ],
                    &[&wallet.creator_keypair],
                )?;
                Ok(Some(transaction))
            }
        )?;
    }

    // ------------- Setup main governance ------------------------
    let threshold = 100u16 * msig.threshold / msig.signers.len() as u16;
    let gov_config: GovernanceConfig =
        GovernanceConfig {
            vote_threshold_percentage: VoteThresholdPercentage::YesVote(threshold.try_into().unwrap()),
            min_community_weight_to_create_proposal: 1,
            min_transaction_hold_up_time: 0,
            max_voting_time: 78200,
            vote_tipping: VoteTipping::Early,
            proposal_cool_off_time: 0,
            min_council_weight_to_create_proposal: 0,
        };

    executor.check_and_create_object(&format!("{} Governance", msig.name), msig_governance.get_data()?,
        |_| {Ok(None)},
        || {
            let transaction = client.create_transaction(
                &[
                    msig_governance.create_governance_instruction(
                        &wallet.creator_keypair.pubkey(),
                        &creator_token_owner,
                        gov_config
                    ),
                ],
                &[&wallet.creator_keypair]
            )?;
            Ok(Some(transaction))
        }
    )?;

    // --------------- Pass token and programs to governance ------
    let mut collector = TransactionCollector::new(client, executor.setup, executor.verbose,
            &format!("{} pass under governance", msig.name));
    // 1. Mint
    collector.check_and_create_object(&format!("{} token mint-authority", msig.name),
        get_mint_data(client, &msig_mint)?,
        |d| {
            if d.mint_authority.contains(&wallet.creator_keypair.pubkey()) {
                let instructions = [
                        spl_token::instruction::set_authority(
                            &spl_token::id(),
                            &msig_mint,
                            Some(&msig_governance.governance_address),
                            spl_token::instruction::AuthorityType::MintTokens,
                            &wallet.creator_keypair.pubkey(),
                            &[],
                        ).unwrap()
                    ].to_vec();
                let signers = [&wallet.creator_keypair].to_vec();
                Ok(Some((instructions, signers,)))
            } else if d.mint_authority.contains(&msig_governance.governance_address) {
                Ok(None)
            } else {
                Err(StateError::InvalidMintAuthority(msig_mint, d.mint_authority).into())
            }
        },
        || {if executor.setup {Err(StateError::MissingMint(msig_mint).into())} else {Ok(None)}},
    )?;

    // 2. Realm
    collector.check_and_create_object(&format!("{} realm authority", msig.name),
        msig_realm.get_data()?,
        |d| {
            if d.authority == Some(wallet.creator_keypair.pubkey()) {
                let instructions = [
                        msig_realm.set_realm_authority_instruction(
                            &wallet.creator_keypair.pubkey(),
                            Some(&msig_governance.governance_address),
                            SetRealmAuthorityAction::SetChecked,
                        )
                    ].to_vec();
                let signers = [&wallet.creator_keypair].to_vec();
                Ok(Some((instructions, signers,)))
            } else if d.authority == Some(msig_governance.governance_address) {
                Ok(None)
            } else {
                Err(StateError::InvalidRealmAuthority(msig_realm.realm_address, d.authority).into())
            }
        },
        || {if executor.setup {Err(StateError::MissingRealm(msig_realm.realm_address).into())} else {Ok(None)}}
    )?;
    collector.execute_transaction()?;

    Ok(msig_governance.governance_address)
}


