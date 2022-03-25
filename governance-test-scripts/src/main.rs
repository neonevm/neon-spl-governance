
use solana_sdk::{
    pubkey::{ Pubkey },
    signer::{
        Signer,
        keypair::{ Keypair, read_keypair_file },
    },
};

use spl_governance::{
    state::{
        enums::{
            VoteThresholdPercentage,
            VoteTipping,
            ProposalState,
        },
        governance::{
            GovernanceConfig,
        },
    },
};

use spl_governance_addin_fixed_weights::{
    instruction::{
        get_max_voter_weight_address,
        get_voter_weight_address,
    }
};

// mod tokens;
mod commands;

use commands::{ Realm, Governance, Proposal, TokenOwner };

const GOVERNANCE_KEY_FILE_PATH: &'static str = "solana-program-library/target/deploy/spl_governance-keypair.json";
const VOTER_WEIGHT_ADDIN_KEY_FILE_PATH: &'static str = "target/deploy/spl_governance_addin_fixed_weights-keypair.json";
const COMMUTINY_MINT_KEY_FILE_PATH: &'static str = "governance-test-scripts/community_mint.keypair";
const GOVERNED_MINT_KEY_FILE_PATH: &'static str = "governance-test-scripts/governance.keypair";

const VOTER1_KEY_FILE_PATH: &'static str = "artifacts/voter1.keypair";
const VOTER2_KEY_FILE_PATH: &'static str = "artifacts/voter2.keypair";
const VOTER3_KEY_FILE_PATH: &'static str = "artifacts/voter3.keypair";
const VOTER4_KEY_FILE_PATH: &'static str = "artifacts/voter4.keypair";
const VOTER5_KEY_FILE_PATH: &'static str = "artifacts/voter5.keypair";

// const REALM_NAME: &'static str = "Test Realm";
const REALM_NAME: &'static str = "_Test_Realm_5";
// const REALM_NAME: &'static str = "Test Realm 6";
const PROPOSAL_NAME: &'static str = "Proposal To Vote";
const PROPOSAL_DESCRIPTION: &'static str = "proposal_description";

fn main() {

    let owner_keypair: Keypair = read_keypair_file(VOTER1_KEY_FILE_PATH).unwrap();
    // let owner_pubkey: Pubkey = owner_keypair.pubkey();
    // println!("Owner Pubkey: {}", owner_pubkey);

    let program_keypair: Keypair = read_keypair_file(GOVERNANCE_KEY_FILE_PATH).unwrap();
    let program_id: Pubkey = program_keypair.pubkey();
    println!("Governance Program Id: {}", program_id);

    let community_keypair: Keypair = read_keypair_file(COMMUTINY_MINT_KEY_FILE_PATH).unwrap();
    let community_pubkey: Pubkey = community_keypair.pubkey();
    println!("Community Token Mint Pubkey: {}", community_pubkey);

    let voter_weight_addin_keypair: Keypair = read_keypair_file(VOTER_WEIGHT_ADDIN_KEY_FILE_PATH).unwrap();
    let voter_weight_addin_pubkey: Pubkey = voter_weight_addin_keypair.pubkey();
    println!("Voter Weight Addin Pubkey: {}", voter_weight_addin_pubkey);

    let governed_account_keypair: Keypair = read_keypair_file(GOVERNED_MINT_KEY_FILE_PATH).unwrap();
    let governed_account_pubkey: Pubkey = governed_account_keypair.pubkey();
    println!("Governed Account (Mint) Pubkey: {}", governed_account_pubkey);

    let voter1_keypair: Keypair = read_keypair_file(VOTER1_KEY_FILE_PATH).unwrap();
    let voter1_pubkey: Pubkey = voter1_keypair.pubkey();
    println!("Voter1 Pubkey: {}", voter1_pubkey);

    let voter2_keypair: Keypair = read_keypair_file(VOTER2_KEY_FILE_PATH).unwrap();
    let voter2_pubkey: Pubkey = voter2_keypair.pubkey();
    println!("Voter2 Pubkey: {}", voter2_pubkey);

    let voter3_keypair: Keypair = read_keypair_file(VOTER3_KEY_FILE_PATH).unwrap();
    let voter3_pubkey: Pubkey = voter3_keypair.pubkey();
    println!("Voter3 Pubkey: {}", voter3_pubkey);

    let voter4_keypair: Keypair = read_keypair_file(VOTER4_KEY_FILE_PATH).unwrap();
    let voter4_pubkey: Pubkey = voter4_keypair.pubkey();
    println!("Voter4 Pubkey: {}", voter4_pubkey);

    let voter5_keypair: Keypair = read_keypair_file(VOTER5_KEY_FILE_PATH).unwrap();
    let voter5_pubkey: Pubkey = voter5_keypair.pubkey();
    println!("Voter5 Pubkey: {}", voter5_pubkey);

    // let max_voter_weight_record_keypair: Keypair = read_keypair_file(MAX_VOTER_WEIGHT_RECORD_KEY_FILE_PATH).unwrap();
    // let max_voter_weight_record_pubkey: Pubkey = max_voter_weight_record_keypair.pubkey();
    // println!("Max Voter Weight Record Pubkey: {}", max_voter_weight_record_pubkey);

    // let voter_weight_record_keypair: Keypair = read_keypair_file(VOTER_WEIGHT_RECORD_KEY_FILE_PATH).unwrap();
    // let voter_weight_record_pubkey: Pubkey = voter_weight_record_keypair.pubkey();
    // println!("Voter Weight Record Pubkey: {}", voter_weight_record_pubkey);

    // let voter2_weight_record_keypair: Keypair = read_keypair_file(VOTER2_WEIGHT_RECORD_KEY_FILE_PATH).unwrap();
    // let voter2_weight_record_pubkey: Pubkey = voter2_weight_record_keypair.pubkey();
    // println!("Voter2 Weight Record Pubkey: {}", voter2_weight_record_pubkey);

    // let voter3_weight_record_keypair: Keypair = read_keypair_file(VOTER3_WEIGHT_RECORD_KEY_FILE_PATH).unwrap();
    // let voter3_weight_record_pubkey: Pubkey = voter3_weight_record_keypair.pubkey();
    // println!("Voter3 Weight Record Pubkey: {}", voter3_weight_record_pubkey);

    let interactor = commands::SplGovernanceInteractor::new("http://localhost:8899", program_id, voter_weight_addin_pubkey);
    // let interactor = commands::SplGovernanceInteractor::new("https://api.devnet.solana.com", program_id, voter_weight_addin_pubkey);

    let realm: Realm = interactor.create_realm(owner_keypair, &community_pubkey, Some(voter_weight_addin_pubkey), REALM_NAME).unwrap();
    println!("{:?}", realm);

    println!("Realm Pubkey: {}", interactor.get_realm_address(REALM_NAME));

    // let result = interactor.setup_max_voter_weight_record_mock(&realm, max_voter_weight_record_keypair, 10_000_000_000);
    let result = interactor.setup_max_voter_weight_record_fixed(&realm);
    println!("{:?}", result);

    let (max_voter_weight_record_address,_) = get_max_voter_weight_address(&voter_weight_addin_pubkey, &realm.address, &community_pubkey);
    println!("MaxVoterWeightRecord Pubkey {:?}", max_voter_weight_record_address);
    let max_voter_weight_record = interactor.get_max_voter_weight_record(&max_voter_weight_record_address);
    println!("{:?}", max_voter_weight_record);
    // return;

    let token_owner1: TokenOwner = interactor.create_token_owner_record(&realm, voter1_keypair).unwrap();
    // let token_owner: TokenOwner = interactor.setup_voter_weight_record_mock(&realm, token_owner, voter_weight_record_keypair, 10_000_000_000).unwrap();
    let token_owner1: TokenOwner = interactor.setup_voter_weight_record_fixed(&realm, token_owner1).unwrap();
    println!("Token Owner 1 \n{:?}", token_owner1);

    let (voter_weight_record_address,_) = get_voter_weight_address(&voter_weight_addin_pubkey, &realm.address, &community_pubkey, &token_owner1.authority.pubkey());
    let voter_weight_record = interactor.get_voter_weight_record(&voter_weight_record_address);
    println!("Token Owner 1 VoterWeightRecord \n{:?}", voter_weight_record);

    let token_owner2: TokenOwner = interactor.create_token_owner_record(&realm, voter2_keypair).unwrap();
    // let token_owner2: TokenOwner = interactor.setup_voter_weight_record_mock(&realm, token_owner2, voter2_weight_record_keypair, 2_000_000_000).unwrap();
    let token_owner2: TokenOwner = interactor.setup_voter_weight_record_fixed(&realm, token_owner2).unwrap();
    println!("Token Owner 2 \n{:?}", token_owner2);

    let (voter_weight_record_address,_) = get_voter_weight_address(&voter_weight_addin_pubkey, &realm.address, &community_pubkey, &token_owner2.authority.pubkey());
    let voter_weight_record = interactor.get_voter_weight_record(&voter_weight_record_address);
    println!("Token Owner 2 VoterWeightRecord \n{:?}", voter_weight_record);

    let token_owner3: TokenOwner = interactor.create_token_owner_record(&realm, voter3_keypair).unwrap();
    let token_owner3: TokenOwner = interactor.setup_voter_weight_record_fixed(&realm, token_owner3).unwrap();
    println!("Token Owner 3 \n{:?}", token_owner3);

    let (voter_weight_record_address,_) = get_voter_weight_address(&voter_weight_addin_pubkey, &realm.address, &community_pubkey, &token_owner3.authority.pubkey());
    let voter_weight_record = interactor.get_voter_weight_record(&voter_weight_record_address);
    println!("Token Owner 3 VoterWeightRecord \n{:?}", voter_weight_record);

    let token_owner4: TokenOwner = interactor.create_token_owner_record(&realm, voter4_keypair).unwrap();
    let token_owner4: TokenOwner = interactor.setup_voter_weight_record_fixed(&realm, token_owner4).unwrap();
    println!("Token Owner 4 \n{:?}", token_owner4);

    let (voter_weight_record_address,_) = get_voter_weight_address(&voter_weight_addin_pubkey, &realm.address, &community_pubkey, &token_owner4.authority.pubkey());
    let voter_weight_record = interactor.get_voter_weight_record(&voter_weight_record_address);
    println!("Token Owner 4 VoterWeightRecord \n{:?}", voter_weight_record);

    let token_owner5: TokenOwner = interactor.create_token_owner_record(&realm, voter5_keypair).unwrap();
    let token_owner5: TokenOwner = interactor.setup_voter_weight_record_fixed(&realm, token_owner5).unwrap();
    println!("Token Owner 5 \n{:?}", token_owner4);

    let (voter_weight_record_address,_) = get_voter_weight_address(&voter_weight_addin_pubkey, &realm.address, &community_pubkey, &token_owner5.authority.pubkey());
    let voter_weight_record = interactor.get_voter_weight_record(&voter_weight_record_address);
    println!("Token Owner 5 VoterWeightRecord \n{:?}", voter_weight_record);

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

    let governance: Governance = interactor.create_governance(&realm, &token_owner1, &governed_account_pubkey, gov_config).unwrap();
    println!("{:?}", governance);

    let proposal_number: u32 = 
        if governance.get_proposal_count() > 0 {
            // governance.get_proposal_count()
            0
        } else {
            0
        };
    let proposal: Proposal = interactor.create_proposal(&realm, &token_owner1, &governance, PROPOSAL_NAME, PROPOSAL_DESCRIPTION, proposal_number).unwrap();
    println!("{:?}", proposal);

    // let result = interactor.add_signatory(&realm, &governance, &proposal, &token_owner);
    // println!("Add signatory {:?}", result);

    let proposal: Proposal = 
        if proposal.data.state == ProposalState::Draft {
            interactor.sign_off_proposal(&realm, &governance, proposal, &token_owner1).unwrap()
        } else {
            proposal
        };
    println!("{:?}\n", proposal);

    // // let result = interactor.cast_vote(&realm, &governance, &proposal, &token_owner, Some(max_voter_weight_record_pubkey), true);
    let result = interactor.cast_vote(&realm, &governance, &proposal, &token_owner1, true);
    println!("{:?}", result);

    let result = interactor.cast_vote(&realm, &governance, &proposal, &token_owner2, false);
    println!("{:?}", result);

    // let result = interactor.cast_vote(&realm, &governance, &proposal, &token_owner3, false);
    // println!("{:?}", result);

    // let result = interactor.cast_vote(&realm, &governance, &proposal, &token_owner4, true);
    // println!("{:?}", result);

    // let result = interactor.cast_vote(&realm, &governance, &proposal, &token_owner5, true);
    // println!("{:?}", result);

}
