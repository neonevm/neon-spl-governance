use crate::prelude::*;
use {
    serde::Deserialize,
};

#[derive(Debug,Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigFile {
    #[serde(default)]
    pub payer: String,

    pub creator: String,
    pub community_mint: String,

    pub governance_program: String,
    pub fixed_weight_addin: String,
    pub vesting_addin: String,
    pub neon_evm_program: String,
    pub maintenance_program: String,

    pub testing: bool,

    #[serde(with = "serde_datetime")]
    pub start_time: NaiveDateTime,
}

//mod serde_pubkey {
//    use serde::Deserialize;
//    use std::str::FromStr;
//    use solana_sdk::pubkey::Pubkey;
//
//    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
//    where D: serde::Deserializer<'de>
//    {
//        let s = String::deserialize(deserializer)?;
//        Pubkey::from_str(&s).map_err(serde::de::Error::custom)
//    }
//}

mod serde_datetime {
    use serde::Deserialize;
    use chrono::NaiveDateTime;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, "%FT%T").map_err(serde::de::Error::custom)
    }
}

//mod serde_keypair {
//    use serde::Deserialize;
//    use solana_sdk::signer::keypair::{Keypair, read_keypair_file};
//
//    pub fn deserialize<'de, D>(deserializer: D) -> Result<Keypair, D::Error>
//    where D: serde::Deserializer<'de>
//    {
//        let s = String::deserialize(deserializer)?;
//        read_keypair_file(s).map_err(serde::de::Error::custom)
//    }
//}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_deserialize() {
        let data = r#"{
            "payer": "../artifacts/payer.keypair",
            "creator": "5mAkTXJMyFxvEeUNpfiwAqUfD3qRSRmdh6j6YgXDwqrm",
            "community-mint": "../artifacts/community-mint.keypair",
            "governance-program": "../artifacts/spl-governance.keypair",
            "fixed-weight-addin": "../artifacts/addin-fixed-wights.keypair",
            "vesting-addin": "../artifacts/addin-vesting.keypair",
            "neon-evm-program": "../artifacts/neon-evm.keypair",
            "maintenance-program": "../artifacts/maintenance.keypair",
            "start-date": "2022-06-21T00:00:00",
            "testing": true
        }"#;

        let config_file: ConfigFile = serde_json::from_str(data).unwrap();
        println!("ConfigFile: {:#?}", config_file);

        let data = r#"{
            "creator":             "5mAkTXJMyFxvEeUNpfiwAqUfD3qRSRmdh6j6YgXDwqrm",
            "community-mint":      "EjLGfD8mpxKLwGDi8AiTisAbGtWWM2L3htkJ6MpvS8Hk",
            "governance-program":  "82pQHEmBbW6CQS8GzLP3WE2pCgMUPSW2XzpuSih3aFDk",
            "fixed-weight-addin":  "56cFVhzLFKuvRXQW68ACLpcbJonZeUBNDdLdZoo5fGnB",
            "vesting-addin":       "5tgpCGfXYaZhKWJsrNR4zyp4o4n3wSQ81i5MzPqKEeAK",
            "neon-evm-program":    "DCPSnJHB38e7vNK6o3AVLswJGRaP87iiNx2zvvapiKBz",
            "maintenance-program": "7aPH9mBAvUtJDGV2L1KyvpR5nKF7man5DZzBPaxmisg5",
            "start-date":          "2022-06-21T00:00:00",
            "testing":             true
        }"#;

        let config_file: ConfigFile = serde_json::from_str(data).unwrap();
        println!("ConfigFile2: {:#?}", config_file);
    }
}

