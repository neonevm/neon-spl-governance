
use solana_sdk::{
    signer::{
        Signer,
        keypair::read_keypair_file,
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

//use spl_governance_addin_fixed_weights::{
//    instruction::{
//        get_max_voter_weight_address,
//        get_voter_weight_address,
//    }
//};

// mod tokens;

use governance_lib::{
    client::SplGovernanceInteractor,
    realm::Realm,
    governance::Governance,
    proposal::Proposal,
    token_owner::TokenOwner,
    addin_fixed_weights::AddinFixedWeights,
};

const GOVERNANCE_KEY_FILE_PATH: &str = "../artifacts/spl_governance-keypair.json";
const VOTER_WEIGHT_ADDIN_KEY_FILE_PATH: &str = "../artifacts/addin-fixed-weights.keypair";
const COMMUTINY_MINT_KEY_FILE_PATH: &str = "../governance-test-scripts/community_mint.keypair";
const GOVERNED_MINT_KEY_FILE_PATH: &str = "../governance-test-scripts/governance.keypair";
const PAYER_KEY_FILE_PATH: &str = "../artifacts/payer.keypair";

const VOTERS_KEY_FILE_PATH: [&str;5] = [
    "../artifacts/voter1.keypair",
    "../artifacts/voter2.keypair",
    "../artifacts/voter3.keypair",
    "../artifacts/voter4.keypair",
    "../artifacts/voter5.keypair",
];

// const REALM_NAME: &str = "Test Realm";
const REALM_NAME: &str = "Test_Realm_3";
// const REALM_NAME: &str = "Test Realm 6";
const PROPOSAL_NAME: &str = "Proposal To Vote";
const PROPOSAL_DESCRIPTION: &str = "proposal_description";

fn main() {

    let program_id = read_keypair_file(GOVERNANCE_KEY_FILE_PATH).unwrap().pubkey();
    println!("Governance Program Id: {}", program_id);

    let payer_keypair = read_keypair_file(PAYER_KEY_FILE_PATH).unwrap();
    println!("Payer Pubkey: {}", payer_keypair.pubkey());

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

    let interactor = SplGovernanceInteractor::new(
            "http://localhost:8899",
            program_id,
            voter_weight_addin_pubkey,
            &payer_keypair);
    // let interactor = SplGovernanceInteractor::new("https://api.devnet.solana.com", program_id, voter_weight_addin_pubkey);

    let mut realm: Realm = interactor.create_realm(
        owner_keypair,
        &community_pubkey,
        Some(voter_weight_addin_pubkey),
        REALM_NAME).unwrap();
    println!("{:?}", realm);
    println!("Realm Pubkey: {}", interactor.get_realm_address(REALM_NAME));

    let fixed_weight_addin = AddinFixedWeights::new(&interactor, voter_weight_addin_pubkey);
    let result = fixed_weight_addin.setup_max_voter_weight_record(&realm);
    println!("VoterWeightAddin.setup_max_voter_weight_record = {:?}", result);
    realm.set_max_voter_weight_record_address(Some(result.unwrap()));

    let mut token_owners = vec!();
    for (i, keypair) in voter_keypairs.iter().enumerate() {
        let mut token_owner: TokenOwner = realm.create_token_owner_record(&keypair.pubkey()).unwrap();
        let voter_weight_record = fixed_weight_addin.setup_voter_weight_record(&realm, &keypair.pubkey()).unwrap();
        token_owner.set_voter_weight_record_address(Some(voter_weight_record));
        println!("Token Owner {} \n{:?}, voter_weight_record: {}", i, token_owner, voter_weight_record);
        token_owners.push(token_owner);
        if i == 2 {
            let result = fixed_weight_addin.set_vote_percentage_fixed(&realm, &keypair, 5000);
            println!("{:?}", result);
        }
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

    let governance: Governance = realm.create_governance(
            &voter_keypairs[0],
            &token_owners[0],
            &governed_account_pubkey,
            gov_config).unwrap();
    println!("{:?}", governance);

    let proposal_number: u32 = 0;
//        if governance.get_proposal_count() > 0 {
//            // governance.get_proposal_count()
//            0
//        } else {
//            0
//        };
    let proposal: Proposal = governance.create_proposal(
            &voter_keypairs[0],
            &token_owners[0],
            PROPOSAL_NAME,
            PROPOSAL_DESCRIPTION,
            proposal_number).unwrap();
    println!("{:?}", proposal);

    // let result = interactor.add_signatory(&realm, &governance, &proposal, &token_owner);
    // println!("Add signatory {:?}", result);

    if proposal.get_state().unwrap() == ProposalState::Draft {
        proposal.sign_off_proposal(
                &voter_keypairs[0],
                &token_owners[0]).unwrap();
    }

    for (i, owner) in token_owners.iter().enumerate() {
        let yes = i == 0 || i == 3 || i == 4;
        let result = proposal.cast_vote(
                &voter_keypairs[i],
                owner, yes);
        println!("CastVote {} {:?}", i, result);
    }
}
