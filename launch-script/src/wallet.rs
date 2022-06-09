use {
    crate::ScriptError,
    solana_sdk::{
        pubkey::{Pubkey, read_pubkey_file},
        signer::{
            Signer,
            keypair::{Keypair, read_keypair_file},
        },
    },
    std::path::Path,
};

const PAYER_KEYPAIR_FILENAME: &str = "payer.keypair";

const GOVERNANCE_PROGRAM_KEY_FILENAME: &str = "spl-governance";
const FIXED_WEIGHT_ADDIN_KEY_FILENAME: &str = "addin-fixed-weights";
const VESTING_ADDIN_KEY_FILENAME: &str = "addin-vesting";
const COMMUNITY_MINT_KEY_FILENAME: &str = "community-mint";
const CREATOR_KEY_FILENAME: &str = "creator";
const VOTERS_FILE_DIR: &str = "voters";

pub struct Wallet {
    pub governance_program_id: Pubkey,
    pub fixed_weight_addin_id: Pubkey,
    pub vesting_addin_id: Pubkey,
    pub community_pubkey: Pubkey,

    pub payer_keypair: Keypair,
    pub creator_pubkey: Pubkey,
    pub creator_keypair: Option<Keypair>,
    pub voter_keypairs: Vec<Keypair>,
}

impl Wallet {
    pub fn new(artifacts: &Path) -> Result<Self,ScriptError> {
        let (creator_pubkey, creator_keypair) = Self::read_keypair_or_pubkey(artifacts, CREATOR_KEY_FILENAME)?;
        Ok(Self {
            governance_program_id: Self::read_keypair_or_pubkey(artifacts, GOVERNANCE_PROGRAM_KEY_FILENAME)?.0,
            fixed_weight_addin_id: Self::read_keypair_or_pubkey(artifacts, FIXED_WEIGHT_ADDIN_KEY_FILENAME)?.0,
            vesting_addin_id: Self::read_keypair_or_pubkey(artifacts, VESTING_ADDIN_KEY_FILENAME)?.0,

            community_pubkey: Self::read_keypair_or_pubkey(artifacts, COMMUNITY_MINT_KEY_FILENAME)?.0,

            payer_keypair: read_keypair_file(artifacts.join(PAYER_KEYPAIR_FILENAME))?,
            creator_pubkey,
            creator_keypair,
            voter_keypairs: {
                let mut voter_keypairs = vec!();
                for file in artifacts.join(VOTERS_FILE_DIR).as_path().read_dir()? {
                    voter_keypairs.push(read_keypair_file(file?.path())?);
                }
                voter_keypairs
            },
        })
    }

    fn read_keypair_or_pubkey(artifacts: &Path, filename: &str) -> Result<(Pubkey,Option<Keypair>), ScriptError> {
        let mut filepath = artifacts.join(filename);
        filepath.set_extension("keypair");
        read_keypair_file(filepath.as_path())
            .map(|keypair| (keypair.pubkey(),Some(keypair),))
            .or_else(|_| {
                filepath.set_extension("pubkey");
                read_pubkey_file(filepath.to_str().unwrap()).map(|pubkey| (pubkey,None,))
            })
            .map_err(|err| err.into())
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

        println!("Voter pubkeys:");
        for (i, keypair) in self.voter_keypairs.iter().enumerate() {
            println!("\t{} {}", i, keypair.pubkey());
        }
    }
}

