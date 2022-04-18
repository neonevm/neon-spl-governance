mod tokens;
mod token_distribution;

use crate::{
    tokens::create_mint,
};
use solana_sdk::{
    pubkey::Pubkey,
    signer::{
        Signer,
        keypair::read_keypair_file,
    },
    transaction::Transaction,
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
        governance::{
            GovernanceConfig,
            GovernanceV2,
        },
        realm::SetRealmAuthorityAction,
    },
};

//use spl_governance_addin_fixed_weights::{
//    instruction::{
//        get_max_voter_weight_address,
//        get_voter_weight_address,
//    }
//};

use spl_governance_addin_vesting::state::VestingSchedule;

// mod tokens;

use governance_lib::{
    client::Client,
    realm::{RealmConfig, Realm},
    governance::Governance,
    proposal::Proposal,
    token_owner::TokenOwner,
    addin_fixed_weights::AddinFixedWeights,
    addin_vesting::AddinVesting,
};

use std::cell::RefCell;

use crate::token_distribution::DISTRIBUTION_LIST;
const ADDITIONAL_SUPPLY: u64 = 10_000_000;

const GOVERNANCE_KEY_FILE_PATH: &str = "solana-program-library/target/deploy/spl_governance-keypair.json";
const VOTER_WEIGHT_ADDIN_KEY_FILE_PATH: &str = "target/deploy/spl_governance_addin_fixed_weights-keypair.json";
const VESTING_ADDIN_KEY_FILE_PATH: &str = "target/deploy/spl_governance_addin_vesting-keypair.json";
const COMMUTINY_MINT_KEY_FILE_PATH: &str = "governance-test-scripts/community_mint.keypair";
const GOVERNED_MINT_KEY_FILE_PATH: &str = "governance-test-scripts/governance.keypair";
const PAYER_KEY_FILE_PATH: &str = "artifacts/payer.keypair";
const CREATOR_KEY_FILE_PATH: &str = "artifacts/creator.keypair";

const VOTERS_KEY_FILE_PATH: [&str;5] = [
    "artifacts/voter1.keypair",
    "artifacts/voter2.keypair",
    "artifacts/voter3.keypair",
    "artifacts/voter4.keypair",
    "artifacts/voter5.keypair",
];

// const REALM_NAME: &str = "Test Realm";
const REALM_NAME: &str = "Test_Realm_9";
// const REALM_NAME: &str = "Test Realm 6";
//const PROPOSAL_NAME: &str = "Token Genesis Event";
//const PROPOSAL_DESCRIPTION: &str = "proposal_description";

fn main() {

    let program_id = read_keypair_file(GOVERNANCE_KEY_FILE_PATH).unwrap().pubkey();
    println!("Governance Program Id: {}", program_id);

    let payer_keypair = read_keypair_file(PAYER_KEY_FILE_PATH).unwrap();
    println!("Payer Pubkey: {}", payer_keypair.pubkey());

    let creator_keypair = read_keypair_file(CREATOR_KEY_FILE_PATH).unwrap();
    println!("Creator Pubkey: {}", creator_keypair.pubkey());

    let community_keypair = read_keypair_file(COMMUTINY_MINT_KEY_FILE_PATH).unwrap();
    let community_pubkey = community_keypair.pubkey();
    println!("Community Token Mint Pubkey: {}", community_pubkey);

    let voter_weight_addin_pubkey = read_keypair_file(VOTER_WEIGHT_ADDIN_KEY_FILE_PATH).unwrap().pubkey();
    println!("Voter Weight Addin Pubkey: {}", voter_weight_addin_pubkey);

    let vesting_addin_pubkey = read_keypair_file(VESTING_ADDIN_KEY_FILE_PATH).unwrap().pubkey();
    println!("Vesting Addin Pubkey: {}", vesting_addin_pubkey);

    let governed_account_pubkey = read_keypair_file(GOVERNED_MINT_KEY_FILE_PATH).unwrap().pubkey();
    println!("Governed Account (Mint) Pubkey: {}", governed_account_pubkey);

    let mut voter_keypairs = vec!();
    for (i, file) in VOTERS_KEY_FILE_PATH.iter().enumerate() {
        let keypair = read_keypair_file(file).unwrap();
        println!("Voter{} Pubkey: {}", i, keypair.pubkey());
        voter_keypairs.push(keypair);
    }

    let client = Client::new("http://localhost:8899", &payer_keypair);
    // let client = Client::new("https://api.devnet.solana.com", program_id, voter_weight_addin_pubkey);

    let mint = client.get_account_data_pack::<spl_token::state::Mint>(&spl_token::id(), &community_pubkey).unwrap();
    if let Some(_mint) = mint {
//        if !mint.mint_authority.contains(&payer_keypair.pubkey()) {
//            panic!("Invalid mint authority: actual {:?}, expected {}", mint.mint_authority, creator_keypair.pubkey());
//        }
    } else {
        let result = create_mint(
                &client.solana_client,
                &payer_keypair,
                &community_keypair,
                &creator_keypair.pubkey(),
                None,
                6).unwrap();
        println!("Created community mint: {}", result);
    }

    let realm = Realm::new(&client, &program_id, REALM_NAME, &community_pubkey);
    if let Some(realm_data) = realm.get_data().unwrap() {
        if realm_data.community_mint != realm.community_mint {
            panic!("Invalid Realm community mint: expected {}, actual {}",
                    realm.community_mint, realm_data.community_mint);
        }
    } else {
        realm.create_realm(
                &creator_keypair,
                Some(voter_weight_addin_pubkey),
                Some(voter_weight_addin_pubkey),
            ).unwrap();
    }
    println!("{:?}", realm);
    println!("Realm Pubkey: {}", realm.realm_address); //client.get_realm_address(REALM_NAME));

    let fixed_weight_addin = AddinFixedWeights::new(&client, voter_weight_addin_pubkey);
    let result = fixed_weight_addin.setup_max_voter_weight_record(&realm);
    println!("VoterWeightAddin.setup_max_voter_weight_record = {:?}", result);
    realm.settings_mut().max_voter_weight_record_address = Some(result.unwrap());

    let vesting_addin = AddinVesting::new(&client, vesting_addin_pubkey);

    //let mut creator_token_owner: TokenOwner = realm.create_token_owner_record(&creator_keypair.pubkey()).unwrap();
    //let creator_voter_weight = fixed_weight_addin.setup_voter_weight_record(&realm, &creator_keypair.pubkey()).unwrap();
    //creator_token_owner.set_voter_weight_record_address(Some(creator_voter_weight));
    //creator_token_owner.set_voter_weight_record_address(Some(creator_keypair.pubkey()));

    let mut token_owners = vec!();
    for (i, keypair) in voter_keypairs.iter().enumerate() {
        let mut token_owner: TokenOwner = realm.token_owner_record(&keypair.pubkey());
        if let None = token_owner.get_data().unwrap() {
            token_owner.create_token_owner_record().unwrap();
        }
        let voter_weight_record = fixed_weight_addin.setup_voter_weight_record(&realm, &keypair.pubkey()).unwrap();
        token_owner.set_voter_weight_record_address(Some(voter_weight_record));
        println!("Token Owner {} \n{:?}, voter_weight_record: {}", i, token_owner, voter_weight_record);
        token_owners.push(token_owner);
    }

    let result = token_owners[0].set_delegate(&voter_keypairs[0], &Some(creator_keypair.pubkey())).unwrap();
    println!("Set delegate for voter[0]: {:?}", result);

    let gov_config: GovernanceConfig =
        GovernanceConfig {
            vote_threshold_percentage: VoteThresholdPercentage::YesVote(60),
            min_community_weight_to_create_proposal: 10,
            min_transaction_hold_up_time: 0,
            max_voting_time: 78200,
            vote_tipping: VoteTipping::Strict,
            proposal_cool_off_time: 0,
            min_council_weight_to_create_proposal: 0,
        };

    let governance = realm.governance(&governed_account_pubkey);
    if let None = governance.get_data().unwrap() {
        governance.create_governance(
                &creator_keypair,
                &token_owners[0],
                gov_config,
            ).unwrap();
    }
    println!("{}", governance);
    println!("{:?}", governance);

    // STEP 2: Pass Token and Realm under governance
    // transaction if already correct authority)
    let mut instructions = vec!();
    let mint = client.get_account_data_pack::<spl_token::state::Mint>(&spl_token::id(), &community_pubkey).unwrap().unwrap();
    if mint.mint_authority.contains(&creator_keypair.pubkey()) {
        instructions.push(
                spl_token::instruction::set_authority(
                    &spl_token::id(),
                    &community_pubkey,
                    Some(&governance.governance_address),
                    spl_token::instruction::AuthorityType::MintTokens,
                    &creator_keypair.pubkey(),
                    &[],
                ).unwrap()
            );
    }
    let realm_data = client.get_account_data::<spl_governance::state::realm::RealmV2>(&program_id, &realm.realm_address).unwrap().unwrap();
    if realm_data.authority == Some(creator_keypair.pubkey()) {
        instructions.push(
                realm.set_realm_authority_instruction(
                    &creator_keypair.pubkey(),
                    Some(&governance.governance_address),
                    SetRealmAuthorityAction::SetChecked,
                )
            );
    }
    if !instructions.is_empty() {
        let transaction: Transaction = 
            Transaction::new_signed_with_payer(
                &instructions,
                Some(&payer_keypair.pubkey()),
                &[
                    &creator_keypair,
                    &payer_keypair,
                ],
                client.solana_client.get_latest_blockhash().unwrap(),
            );
        client.solana_client.send_and_confirm_transaction(&transaction).unwrap();
    }

    // =========================================================================
    // Create TGE proposal (Token Genesis Event)
    // =========================================================================

    let proposal_number = governance.get_proposals_count();
    let proposal: Proposal = governance.proposal(proposal_number);
    proposal.create_proposal(
            &creator_keypair,
            &token_owners[0],
            &format!("{} {}", "Token Genesis Event", proposal_number),
            "Proposal for Token Genesis Event (mint tokens and distribute it)",
        ).unwrap();
    println!("{:?}", proposal);

    // let result = client.add_signatory(&realm, &governance, &proposal, &token_owner);
    // println!("Add signatory {:?}", result);
    
    let governance_token_account = spl_associated_token_account::get_associated_token_address_with_program_id(
                        &governance.governance_address,
                        &community_pubkey,
                        &spl_token::id(),
                    );
    println!("Governance address: {}", governance.governance_address);
    println!("Governance token account: {}", governance_token_account);
    if !client.account_exists(&governance_token_account) {
        let transaction: Transaction = 
            Transaction::new_signed_with_payer(
                &[
                    spl_associated_token_account::create_associated_token_account(
                        &payer_keypair.pubkey(),
                        &governance.governance_address,
                        &community_pubkey,
                    ),
                ],
                Some(&payer_keypair.pubkey()),
                &[
                    &payer_keypair,
                ],
                client.solana_client.get_latest_blockhash().unwrap(),
            );
        let signature = client.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        println!("Create associated token account {}", signature);
    }

    let total_amount = DISTRIBUTION_LIST.iter().map(|(_, amount)| amount).sum::<u64>() + ADDITIONAL_SUPPLY;
    proposal.insert_transaction(
            &creator_keypair,
            &token_owners[0],
            0, 0, 0,
            vec![
                spl_token::instruction::mint_to(
                    &spl_token::id(),
                    &community_pubkey,
                    &governance_token_account,
                    &governance.governance_address, &[],
                    total_amount,
                ).unwrap().into(),
            ],
        ).unwrap();

    for (i, (owner, amount)) in DISTRIBUTION_LIST.iter().enumerate() {
        let seed: String = format!("{}_vesting_{}", REALM_NAME, i);
        let vesting_token_account = Pubkey::create_with_seed(&creator_keypair.pubkey(), &seed, &spl_token::id()).unwrap();
        // TODO Calculate schedule
        let schedule = vec!(VestingSchedule { release_time: 0, amount: *amount });
        println!("{}, Voter {}, amount {}, token_account {}", i, owner, amount, vesting_token_account);

        let mut instructions = vec!();
        if !client.account_exists(&vesting_token_account) {
            instructions.extend([
                system_instruction::create_account_with_seed(
                    &payer_keypair.pubkey(),              // from
                    &vesting_token_account,               // to
                    &creator_keypair.pubkey(),            // base
                    &seed,                                // seed
                    Rent::default().minimum_balance(165), // lamports
                    165,                                  // space
                    &spl_token::id(),                     // owner
                ),
                spl_token::instruction::initialize_account(
                    &spl_token::id(),
                    &vesting_token_account,
                    &community_pubkey,
                    &vesting_addin.find_vesting_account(&vesting_token_account),
                ).unwrap(),
            ]);
        }
        instructions.push(
                proposal.insert_transaction_instruction(
                    &creator_keypair.pubkey(),
                    &token_owners[0],
                    0, (i+1).try_into().unwrap(), 0,
                    vec![
                        vesting_addin.deposit_with_realm_instruction(
                            &governance.governance_address,          // source_token_authority
                            &governance_token_account,    // source_token_account
                            &owner,                       // vesting_owner
                            &vesting_token_account,       // vesting_token_account
                            schedule,                     // schedule
                            &realm,                       // realm
                        ).unwrap().into(),
                    ],
                ),
            );
            
        let result = client.send_and_confirm_transaction(&instructions, &[&creator_keypair]).unwrap();
        println!("   created: {}", result);
    }

    // Change to other VoterWeight addin
    proposal.insert_transaction(
        &creator_keypair,
        &token_owners[0],
        0, (DISTRIBUTION_LIST.len()+1).try_into().unwrap(), 0,
        vec![
            realm.set_realm_config_instruction(
                &governance.governance_address,       // we already passed realm under governance
                &RealmConfig {
                    council_token_mint: None,
                    community_voter_weight_addin: Some(vesting_addin_pubkey),
                    max_community_voter_weight_addin: None,
                    min_community_weight_to_create_governance: 1,
                    community_mint_max_vote_weight_source: MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                }
            ).into(),
        ],
    ).unwrap();

    // Change Governance config
    proposal.insert_transaction(
        &creator_keypair,
        &token_owners[0],
        0, (DISTRIBUTION_LIST.len()+2).try_into().unwrap(), 0,
        vec![
            governance.set_governance_config_instruction(
                GovernanceConfig {
                    vote_threshold_percentage: VoteThresholdPercentage::YesVote(2),
                    min_community_weight_to_create_proposal: 3*1000_000,
                    min_transaction_hold_up_time: 0,
                    max_voting_time: 1*60, // 3*24*3600,
                    vote_tipping: VoteTipping::Disabled,
                    proposal_cool_off_time: 0,                 // not implemented in the current version
                    min_council_weight_to_create_proposal: 0,  // council token does not used
                },
            ).into(),
        ],
    ).unwrap();

    if proposal.get_state().unwrap() == ProposalState::Draft {
        proposal.sign_off_proposal(&creator_keypair, &token_owners[0]).unwrap();
    }

    for (i, owner) in token_owners.iter().enumerate() {
        let yes_no = i == 0 || i == 3 || i == 4;
        let result = proposal.cast_vote(&token_owners[0], &voter_keypairs[i], owner, yes_no);
        println!("CastVote {} {:?}", i, result);
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    let result = proposal.execute_transactions(0).unwrap();
    println!("Execute transactions from proposal option 0: {:?}", result);


    // ===================================================================================
    // Check correctly operation after switching to vesting-addin
    // ===================================================================================
    realm.settings_mut().max_voter_weight_record_address = None;
    for (ref mut token_owner) in token_owners.iter_mut() {
        let token_owner_pubkey = token_owner.token_owner_address;
        let voter_weight_record = vesting_addin.get_voter_weight_record_address(&token_owner_pubkey, &realm);
        token_owner.set_voter_weight_record_address(Some(voter_weight_record));
    }

    // ===================================================================================
    // Create proposal
    // ===================================================================================
    let proposal: Proposal = governance.proposal(governance.get_proposals_count());
    proposal.create_proposal(
            &creator_keypair,
            &token_owners[0],
            "Deploy EVM",
            "Deploy EVM and configure governance to control it",
        ).unwrap();
    println!("{:?}", proposal);

    if proposal.get_state().unwrap() == ProposalState::Draft {
        proposal.sign_off_proposal(&creator_keypair, &token_owners[0]).unwrap();
    }

    for (i, owner) in token_owners.iter().enumerate() {
        let yes_no = i == 0 || i == 3 || i == 4;
        let result = proposal.cast_vote(&token_owners[0], &voter_keypairs[i], owner, yes_no);
        println!("CastVote {} {:?}", i, result);
    }
}
