use {
    crate::ScriptError,
    crate::errors::StateError,
    crate::config_file::ConfigFile,
    solana_sdk::{
        pubkey::{read_pubkey_file, Pubkey},
        signer::{
            keypair::{read_keypair_file, Keypair},
            Signer,
        },
    },
    std::path::Path,
    std::str::FromStr,
};

const PAYER_KEYPAIR_FILENAME: &str = "payer.keypair";

const GOVERNANCE_PROGRAM_KEY_FILENAME: &str = "spl-governance";
const FIXED_WEIGHT_ADDIN_KEY_FILENAME: &str = "addin-fixed-weights";
const VESTING_ADDIN_KEY_FILENAME: &str = "addin-vesting";
const COMMUNITY_MINT_KEY_FILENAME: &str = "community-mint";
const NEON_EVM_PROGRAM_KEY_FILENAME: &str = "neon-evm";
const MAINTENANCE_PROGRAM_KEY_FILENAME: &str = "maintenance";
const CREATOR_KEY_FILENAME: &str = "creator";

pub struct Wallet {
    pub governance_program_id: Pubkey,
    pub fixed_weight_addin_id: Pubkey,
    pub vesting_addin_id: Pubkey,
    pub community_pubkey: Pubkey,
    pub neon_evm_program_id: Pubkey,
    pub maintenance_program_id: Pubkey,

    pub payer_keypair: Keypair,
    pub creator_pubkey: Pubkey,
    pub creator_keypair: Option<Keypair>,
}

impl Wallet {
    pub fn new(artifacts: &Path) -> Result<Self, ScriptError> {
        let (creator_pubkey, creator_keypair) =
            Self::read_keypair_or_pubkey(artifacts, CREATOR_KEY_FILENAME)?;
        Ok(Self {
            governance_program_id: Self::read_keypair_or_pubkey(
                artifacts,
                GOVERNANCE_PROGRAM_KEY_FILENAME,
            )?
            .0,
            fixed_weight_addin_id: Self::read_keypair_or_pubkey(
                artifacts,
                FIXED_WEIGHT_ADDIN_KEY_FILENAME,
            )?
            .0,
            vesting_addin_id: Self::read_keypair_or_pubkey(artifacts, VESTING_ADDIN_KEY_FILENAME)?
                .0,

            community_pubkey: Self::read_keypair_or_pubkey(artifacts, COMMUNITY_MINT_KEY_FILENAME)?
                .0,
            neon_evm_program_id: Self::read_keypair_or_pubkey(
                artifacts,
                NEON_EVM_PROGRAM_KEY_FILENAME,
            )?
            .0,
            maintenance_program_id: Self::read_keypair_or_pubkey(
                artifacts,
                MAINTENANCE_PROGRAM_KEY_FILENAME,
            )?
            .0,

            payer_keypair: read_keypair_file(artifacts.join(PAYER_KEYPAIR_FILENAME))?,
            creator_pubkey,
            creator_keypair,
        })
    }

    pub fn new_from_config(config: &ConfigFile) -> Result<Self,ScriptError> {
        let (creator_pubkey, creator_keypair) = Self::parse_pubkey_or_read_keypair(&config.creator)?;
        Ok(Self {
            governance_program_id: Self::parse_pubkey_or_read_keypair(&config.governance_program)?.0,
            fixed_weight_addin_id: Self::parse_pubkey_or_read_keypair(&config.fixed_weight_addin)?.0,
            vesting_addin_id: Self::parse_pubkey_or_read_keypair(&config.vesting_addin)?.0,

            community_pubkey: Self::parse_pubkey_or_read_keypair(&config.community_mint)?.0,
            neon_evm_program_id: Self::parse_pubkey_or_read_keypair(&config.neon_evm_program)?.0,

            payer_keypair: read_keypair_file(&config.payer)?,
            creator_pubkey,
            creator_keypair,
        })
    }

    fn parse_pubkey_or_read_keypair(value: &str) -> Result<(Pubkey, Option<Keypair>), ScriptError> {
        Pubkey::from_str(value).map(|v| (v, None))
        .or_else(|_|
            read_keypair_file(value).map(|keypair| (keypair.pubkey(), Some(keypair)))
        )
        .map_err(|err| StateError::ConfigError(format!("'{}' should be pubkey or keypair file: {}", value, err)).into())
    }

    fn read_keypair_or_pubkey(artifacts: &Path, filename: &str) -> Result<(Pubkey,Option<Keypair>), ScriptError> {
        let mut filepath = artifacts.join(filename);
        filepath.set_extension("keypair");
        read_keypair_file(filepath.as_path())
            .map(|keypair| (keypair.pubkey(), Some(keypair)))
            .or_else(|_| {
                filepath.set_extension("pubkey");
                read_pubkey_file(filepath.to_str().unwrap()).map(|pubkey| (pubkey, None))
            })
            .map_err(|err| err.into())
    }

    pub fn get_creator_keypair(&self) -> Result<&Keypair, ScriptError> {
        match &self.creator_keypair {
            Some(keypair) => Ok(keypair),
            None => Err(ScriptError::MissingSignerKeypair),
        }
    }

    pub fn account_by_seed(&self, seed: &str, program: &Pubkey) -> Pubkey {
        Pubkey::create_with_seed(&self.creator_pubkey, seed, program).unwrap()
    }

    pub fn display(&self) {
        println!("Governance Program Id:   {}", self.governance_program_id);
        println!("Fixed Weight Addin Id:   {}", self.fixed_weight_addin_id);
        println!("Vesting Addin Id:        {}", self.vesting_addin_id);

        println!("Community Token Mint:    {}", self.community_pubkey);
        println!("Neon EVM Program Id:     {}", self.neon_evm_program_id);

        println!("Payer Pubkey:            {}", self.payer_keypair.pubkey());
        println!("Creator Pubkey:          {}   private key {}", self.creator_pubkey,
                if self.creator_keypair.is_some() {"PRESENT"} else {"MISSING"});
    }
}
