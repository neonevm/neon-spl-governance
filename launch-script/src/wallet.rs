use {
    crate::ScriptError,
    solana_sdk::{
        pubkey::Pubkey,
        signer::{
            Signer,
            keypair::{Keypair, read_keypair_file},
        },
    },
};

const GOVERNANCE_KEY_FILE_PATH:         &str = "solana-program-library/target/deploy/spl_governance-keypair.json";
const VOTER_WEIGHT_ADDIN_KEY_FILE_PATH: &str = "target/deploy/spl_governance_addin_fixed_weights-keypair.json";
const VESTING_ADDIN_KEY_FILE_PATH:      &str = "target/deploy/spl_governance_addin_vesting-keypair.json";
const COMMUTINY_MINT_KEY_FILE_PATH:     &str = "governance-test-scripts/community_mint.keypair";
const GOVERNED_MINT_KEY_FILE_PATH:      &str = "governance-test-scripts/governance.keypair";
const PAYER_KEY_FILE_PATH:   &str = "artifacts/payer.keypair";
const CREATOR_KEY_FILE_PATH: &str = "artifacts/creator.keypair";

const VOTERS_KEY_FILE_PATH: [&str;5] = [
    "artifacts/voter1.keypair",
    "artifacts/voter2.keypair",
    "artifacts/voter3.keypair",
    "artifacts/voter4.keypair",
    "artifacts/voter5.keypair",
];

pub struct Wallet {
    pub governance_program_id: Pubkey,
    pub fixed_weight_addin_id: Pubkey,
    pub vesting_addin_id: Pubkey,

    pub community_pubkey: Pubkey,
    pub community_keypair: Keypair,
    pub governed_account_pubkey: Pubkey,

    pub payer_keypair: Keypair,
    pub creator_keypair: Keypair,
    pub voter_keypairs: Vec<Keypair>,
}

impl Wallet {
    pub fn new() -> Result<Self,ScriptError> {
        let community_keypair = read_keypair_file(COMMUTINY_MINT_KEY_FILE_PATH)?;
        Ok(Self {
            governance_program_id: read_keypair_file(GOVERNANCE_KEY_FILE_PATH)?.pubkey(),
            fixed_weight_addin_id: read_keypair_file(VOTER_WEIGHT_ADDIN_KEY_FILE_PATH)?.pubkey(),
            vesting_addin_id: read_keypair_file(VESTING_ADDIN_KEY_FILE_PATH)?.pubkey(),

            community_pubkey: community_keypair.pubkey(),
            community_keypair: community_keypair,
            governed_account_pubkey: read_keypair_file(GOVERNED_MINT_KEY_FILE_PATH)?.pubkey(),

            payer_keypair: read_keypair_file(PAYER_KEY_FILE_PATH)?,
            creator_keypair: read_keypair_file(CREATOR_KEY_FILE_PATH)?,
            voter_keypairs: {
                let mut voter_keypairs = vec!();
                for file in VOTERS_KEY_FILE_PATH.iter() {
                    voter_keypairs.push(read_keypair_file(file)?);
                }
                voter_keypairs
            },
        })
    }

    pub fn display(&self) {
        println!("Governance Program Id:   {}", self.governance_program_id);
        println!("Fixed Weight Addin Id:   {}", self.fixed_weight_addin_id);
        println!("Vesting Addin Id:        {}", self.vesting_addin_id);

        println!("Community Token Mint:    {}", self.community_pubkey);
        println!("Governed Account (Mint): {}", self.governed_account_pubkey);

        println!("Payer Pubkey:            {}", self.payer_keypair.pubkey());
        println!("Creator Pubkey:          {}", self.creator_keypair.pubkey());
        println!("Voter pubkeys:");
        for (i, ref keypair) in self.voter_keypairs.iter().enumerate() {
            println!("\t{}: {}", i, keypair.pubkey());
        }
    }
}

