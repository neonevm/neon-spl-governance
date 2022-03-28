
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

const VOTERS_KEY_FILE_PATH: [&str;5] = [
    "artifacts/voter1.keypair",
    "artifacts/voter2.keypair",
    "artifacts/voter3.keypair",
    "artifacts/voter4.keypair",
    "artifacts/voter5.keypair",
];

// const REALM_NAME: &'static str = "Test Realm";
const REALM_NAME: &'static str = "_Test_Realm_12";
// const REALM_NAME: &'static str = "Test Realm 6";
const PROPOSAL_NAME: &'static str = "Proposal To Vote";
const PROPOSAL_DESCRIPTION: &'static str = "proposal_description";

fn main() {

    let program_id = read_keypair_file(GOVERNANCE_KEY_FILE_PATH).unwrap().pubkey();
    println!("Governance Program Id: {}", program_id);

    let community_pubkey = read_keypair_file(COMMUTINY_MINT_KEY_FILE_PATH).unwrap().pubkey();
    println!("Community Token Mint Pubkey: {}", community_pubkey);

    let voter_weight_addin_pubkey = read_keypair_file(VOTER_WEIGHT_ADDIN_KEY_FILE_PATH).unwrap().pubkey();
    println!("Voter Weight Addin Pubkey: {}", voter_weight_addin_pubkey);

    let governed_account_pubkey = read_keypair_file(GOVERNED_MINT_KEY_FILE_PATH).unwrap().pubkey();
    println!("Governed Account (Mint) Pubkey: {}", governed_account_pubkey);

    let mut voter_keypairs = vec!();
    for (i, file) in VOTERS_KEY_FILE_PATH.iter().enumerate() {
        let keypair = read_keypair_file(file).unwrap();
        println!("Voter{} Pubkey: {}", i, keypair.pubkey());
        voter_keypairs.push(keypair);
    }

    let owner_keypair = &voter_keypairs[0];

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

    let mut token_owners = vec!();
    for (i, keypair) in voter_keypairs.iter().enumerate() {
        let token_owner: TokenOwner = interactor.create_token_owner_record(&realm, keypair).unwrap();
        let token_owner: TokenOwner = interactor.setup_voter_weight_record_fixed(&realm, token_owner).unwrap();
        println!("Token Owner {} \n{:?}", i, token_owner);
        token_owners.push(token_owner);
    }

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

    let governance: Governance = interactor.create_governance(&realm, &token_owners[0], &governed_account_pubkey, gov_config).unwrap();
    println!("{:?}", governance);

    let proposal_number: u32 = 
        if governance.get_proposal_count() > 0 {
            // governance.get_proposal_count()
            0
        } else {
            0
        };
    let proposal: Proposal = interactor.create_proposal(&realm, &token_owners[0], &governance, PROPOSAL_NAME, PROPOSAL_DESCRIPTION, proposal_number).unwrap();
    println!("{:?}", proposal);

    // let result = interactor.add_signatory(&realm, &governance, &proposal, &token_owner);
    // println!("Add signatory {:?}", result);

    let proposal: Proposal = 
        if proposal.data.state == ProposalState::Draft {
            interactor.sign_off_proposal(&realm, &governance, proposal, &token_owners[0]).unwrap()
        } else {
            proposal
        };
    println!("{:?}\n", proposal);

    // // let result = interactor.cast_vote(&realm, &governance, &proposal, &token_owner, Some(max_voter_weight_record_pubkey), true);
    for (i, owner) in token_owners.iter().enumerate() {
        let yes = i == 0 || i == 3 || i == 4;
        let result = interactor.cast_vote(&realm, &governance, &proposal, &owner, yes);
        println!("CastVote {} {:?}", i, result);
    }
}
