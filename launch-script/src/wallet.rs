use {
    crate::ScriptError,
    solana_sdk::{
        pubkey::Pubkey,
        signer::{
            Signer,
            keypair::{Keypair, read_keypair_file},
        },
    },
    std::{
        fs,
    },
};

const GOVERNANCE_KEY_FILE_PATH:             &str = "artifacts/spl-governance.keypair";
const VOTER_WEIGHT_ADDIN_KEY_FILE_PATH:     &str = "artifacts/addin-fixed-weights.keypair";
const VESTING_ADDIN_KEY_FILE_PATH:          &str = "artifacts/addin-vesting.keypair";
const COMMUTINY_MINT_KEY_FILE_PATH:         &str = "launch-script/community_mint.keypair";
const PAYER_KEY_FILE_PATH:                  &str = "artifacts/payer.keypair";
const CREATOR_KEY_FILE_PATH:                &str = "artifacts/creator.keypair";
const CREATOR_TOKEN_OWNER_KEY_FILE_PATH:    &str = "artifacts/creator_token_owner.keypair";
const VOTERS_KEY_FILE_DIR:                  &str = "artifacts/voters/";

pub struct Wallet {
    pub governance_program_id: Pubkey,
    pub fixed_weight_addin_id: Pubkey,
    pub vesting_addin_id: Pubkey,

    pub community_pubkey: Pubkey,
    pub community_keypair: Keypair,

    pub payer_keypair: Keypair,
    pub creator_pubkey: Pubkey,
    pub creator_keypair: Option<Keypair>,
    pub creator_token_owner_keypair: Keypair,
    pub voter_keypairs: Vec<Keypair>,
}

impl Wallet {
    pub fn new() -> Result<Self,ScriptError> {
        let community_keypair = read_keypair_file(COMMUTINY_MINT_KEY_FILE_PATH)?;
        let creator_keypair = read_keypair_file(CREATOR_KEY_FILE_PATH)?;
        Ok(Self {
            governance_program_id: read_keypair_file(GOVERNANCE_KEY_FILE_PATH)?.pubkey(),
            fixed_weight_addin_id: read_keypair_file(VOTER_WEIGHT_ADDIN_KEY_FILE_PATH)?.pubkey(),
            vesting_addin_id: read_keypair_file(VESTING_ADDIN_KEY_FILE_PATH)?.pubkey(),

            community_pubkey: community_keypair.pubkey(),
            community_keypair,

            payer_keypair: read_keypair_file(PAYER_KEY_FILE_PATH)?,
            creator_pubkey: creator_keypair.pubkey(),
            creator_keypair: Some(creator_keypair),
            creator_token_owner_keypair: read_keypair_file(CREATOR_TOKEN_OWNER_KEY_FILE_PATH)?,
            voter_keypairs: {
                let mut voter_keypairs = vec!();
                for file in fs::read_dir(VOTERS_KEY_FILE_DIR)? {
                    voter_keypairs.push(read_keypair_file(file?.path())?);
                }
                voter_keypairs
            },
        })
    }

    pub fn get_creator_keypair(&self) -> Result<&Keypair, ScriptError> {
        match &self.creator_keypair {
            Some(keypair) => Ok(keypair),
            None => Err(ScriptError::MissingSignerKeypair),
        }
    }

    pub fn display(&self) {
        println!("Governance Program Id:   {}", self.governance_program_id);
        println!("Fixed Weight Addin Id:   {}", self.fixed_weight_addin_id);
        println!("Vesting Addin Id:        {}", self.vesting_addin_id);

        println!("Community Token Mint:    {}", self.community_pubkey);

        println!("Payer Pubkey:            {}", self.payer_keypair.pubkey());
        println!("Creator Pubkey:          {}   private key {}", self.creator_pubkey,
                if self.creator_keypair.is_some() {"PRESENT"} else {"MISSING"});

        println!("Creator token owner:     {}", self.creator_token_owner_keypair.pubkey());
        println!("Voter pubkeys:");
        for (i, keypair) in self.voter_keypairs.iter().enumerate() {
            println!("\t{} {}", i, keypair.pubkey());
        }
    }
}

