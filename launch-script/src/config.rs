use std::str::FromStr;
use crate::prelude::*;

use solana_sdk::{
    hash::{ hash, Hash, },
};

pub struct Configuration<'a> {
    pub wallet: &'a Wallet,
    pub client: &'a Client<'a>,

    pub send_trx: bool,
    pub verbose: bool,
    pub testing: bool,
    pub start_time: NaiveDateTime,

    // pub community_mint: Pubkey,
    // pub maintenance_program_address: Pubkey,

    pub startup_realm_config: RealmConfig,
    pub working_realm_config: RealmConfig,
    pub community_governance_config: GovernanceConfig,
    pub emergency_governance_config: GovernanceConfig,
    pub maintenance_governance_config: GovernanceConfig,

    pub code_hashes: Vec<Hash>,
    pub delegates: Vec<Pubkey>,
    pub chain_id: u64,

    pub multi_sigs: Vec<MultiSig>,
    pub extra_token_accounts: Vec<ExtraTokenAccount>,
}

fn get_executable_hash(filepath: &str) -> Hash {
    std::fs::read(filepath)
        .map(|data| hash(&data) )
        .unwrap()
}

impl<'a> Configuration<'a> {
    pub fn create_from_config(
        wallet: &'a Wallet,
        client: &'a Client,
        send_trx: bool,
        verbose: bool,
        config: &ConfigFile,
    ) -> Self {
        Self::create(
            wallet,
            client,
            send_trx,
            verbose,
            config.delegates
                .iter()
                .map(|d| Pubkey::from_str(d.as_str()).unwrap() )
                .collect(),
            config.executables_paths
                .iter()
                .map(|fp| get_executable_hash(fp.as_str()) )
                .collect(),
            config.chain_id,
            config.testing,
            Some(config.start_time),
            // Pubkey::from_str(&config.community_mint).unwrap(),
            // Pubkey::from_str(&config.maintenance_program).unwrap(),
        )
    }

    pub fn create(
        wallet: &'a Wallet,
        client: &'a Client,
        send_trx: bool,
        verbose: bool,
        delegates: Vec<Pubkey>,
        code_hashes: Vec<Hash>,
        chain_id: u64,
        testing: bool,
        start_time: Option<NaiveDateTime>,
        // community_mint: Pubkey,
        // maintenance_pubkey: Pubkey,
    ) -> Self {
        let account = |seed, program| wallet.account_by_seed(seed, program);
        Self {
            wallet,
            client,
            send_trx,
            verbose,
            testing,
            start_time: start_time.unwrap_or_else(|| {
                if testing {
                    Utc::now().naive_utc()
                } else {
                    Utc::today().naive_utc().and_hms(0, 0, 0)
                }
            }),
            // community_mint,
            // maintenance_program_address: maintenance_pubkey,
            startup_realm_config: RealmConfig {
                council_token_mint: None,
                community_voter_weight_addin: Some(wallet.fixed_weight_addin_id),
                max_community_voter_weight_addin: Some(wallet.fixed_weight_addin_id),
                min_community_weight_to_create_governance: 1_000_000 * TOKEN_MULT,
                community_mint_max_vote_weight_source:
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
            },
            working_realm_config: RealmConfig {
                council_token_mint: None,
                community_voter_weight_addin: Some(wallet.vesting_addin_id),
                max_community_voter_weight_addin: None,
                min_community_weight_to_create_governance: 1_000_000 * TOKEN_MULT,
                community_mint_max_vote_weight_source:
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
            },
            community_governance_config: GovernanceConfig {
                vote_threshold_percentage: VoteThresholdPercentage::YesVote(1),
                min_community_weight_to_create_proposal: 3_000 * TOKEN_MULT,
                min_transaction_hold_up_time: (if testing {
                    Duration::minutes(1)
                } else {
                    Duration::days(2)
                })
                .num_seconds() as u32,
                max_voting_time: (if testing {
                    Duration::minutes(3)
                } else {
                    Duration::days(1)
                })
                .num_seconds() as u32,
                vote_tipping: VoteTipping::Disabled,
                proposal_cool_off_time: 0,
                min_council_weight_to_create_proposal: 0,
            },
            emergency_governance_config: GovernanceConfig {
                vote_threshold_percentage: VoteThresholdPercentage::YesVote(9),
                min_community_weight_to_create_proposal: 1_000_000 * TOKEN_MULT,
                min_transaction_hold_up_time: 0,
                max_voting_time: (if testing {
                    Duration::minutes(3)
                } else {
                    Duration::days(1)
                })
                .num_seconds() as u32,
                vote_tipping: VoteTipping::Disabled,
                proposal_cool_off_time: 0,
                min_council_weight_to_create_proposal: 0,
            },
            maintenance_governance_config: GovernanceConfig {
                vote_threshold_percentage: VoteThresholdPercentage::YesVote(1),
                min_community_weight_to_create_proposal: 200_000 * TOKEN_MULT,
                min_transaction_hold_up_time: 0,
                max_voting_time: (if testing {
                    Duration::minutes(3)
                } else {
                    Duration::days(1)
                })
                .num_seconds() as u32,
                vote_tipping: VoteTipping::Disabled,
                proposal_cool_off_time: 0,
                min_council_weight_to_create_proposal: 0,
            },
            delegates,
            code_hashes,
            chain_id,
            multi_sigs: vec![
                MultiSig {
                    name: "1".to_string(),
                    threshold: 2,
                    governed_accounts: vec![],
                    signers: vec![
                        pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
                        pubkey!("6tAoNNAB6sXMbt8phMjr46noQ5T18GnnkBftWcw1HfCW"),
                        pubkey!("EsyJ9wzg2VTCCfHmnyi7ePE9LU368iVCrEd4LZeDYMzJ"),
                    ],
                },
                MultiSig {
                    name: "2".to_string(),
                    threshold: 2,
                    governed_accounts: vec![],
                    signers: vec![
                        pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
                        pubkey!("H3cAYot4UJuY1jQhn8FtpeP4fHia3SXtvuKYaov7KMA9"),
                        pubkey!("8ZjncH1eKhJMmwqymWwPEAEaPjTSt91R1gMwx2bMyZqC"),
                    ],
                },
                MultiSig {
                    name: "4".to_string(),
                    threshold: 2,
                    governed_accounts: vec![],
                    signers: vec![
                        pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
                        pubkey!("2Smf7Kyskf3VXUKUB16GVgCizW4qDhvRREGCLcHt7bJV"),
                        pubkey!("EwNeN5ixjqNmBNGbVKDHd1iipStGhMC9u5yGsq7zsw6L"),
                    ],
                },
                MultiSig {
                    name: "5".to_string(),
                    threshold: 2,
                    governed_accounts: vec![account("MSIG_5.1", &spl_token::id())],
                    signers: if testing {
                        vec![
                            pubkey!("tstUPDM1tDgRgC8KALbXQ3hJeKQQTxDywyDVvxv51Lu"),
                            pubkey!("tstTLYLzy9Q5meFUmhhiXfnaGai96hc7Ludu3gQz8nh"),
                            wallet.payer_keypair.pubkey(),
                        ]
                    } else {
                        vec![
                            pubkey!("BU6N2Z68JPXLf247iYnHUTUv1B7p8AFWGTYkcjfeSwY8"),
                            pubkey!("C16ojhtyjzqenxHcg9hNjhAwZhdLJrCBKavfc4gqa1v3"),
                            pubkey!("4vdhzpPYPABJe9WvZA8pFzdbzYaHrj7yNwDQmjBCtts5"),
                        ]
                    },
                },
            ],
            extra_token_accounts: vec![
                ExtraTokenAccount::new(
                    210_000_000 * TOKEN_MULT,
                    Lockup::NoLockup,
                    AccountOwner::BothGovernance,
                ),
                ExtraTokenAccount::new(
                    80_000_000 * TOKEN_MULT,
                    Lockup::NoLockup,
                    AccountOwner::Key(pubkey!("tstzQJwDhrPNSmqtV5rmC26xbbeBf56xFz9wpyTV7tW")),
                ),
                ExtraTokenAccount::new(
                    188_762_400 * TOKEN_MULT,
                    Lockup::NoLockup,
                    AccountOwner::MultiSig("1", None),
                ),
                ExtraTokenAccount::new(
                    60_000_000 * TOKEN_MULT,
                    Lockup::For4Years,
                    AccountOwner::MultiSig("2", None),
                ),
                ExtraTokenAccount::new(
                    7_500_000 * TOKEN_MULT,
                    Lockup::For1year1yearLinear,
                    AccountOwner::MultiSig("4", None),
                ),
                ExtraTokenAccount::new(
                    3_750_000 * TOKEN_MULT,
                    Lockup::For1year1yearLinear,
                    AccountOwner::MultiSig("4", None),
                ),
                ExtraTokenAccount::new(
                    142_700_000 * TOKEN_MULT,
                    Lockup::For1year1yearLinear,
                    AccountOwner::MultiSig("5", None),
                ),
                ExtraTokenAccount::new(
                    1_000_000 * TOKEN_MULT,
                    Lockup::For1year1yearLinear,
                    AccountOwner::MultiSig("5", Some(account("MSIG_5.1", &spl_token::id()))),
                ),
            ],
        }
    }

    pub fn account_by_seed(&self, seed: &str, program: &Pubkey) -> Pubkey {
        self.wallet.account_by_seed(seed, program)
    }

    pub fn neon_multisig_address(&self) -> Pubkey {
        self.account_by_seed(&format!("{}_multisig", REALM_NAME), &spl_token::id())
    }

    pub fn get_schedule_size(&self, lockup: &Lockup) -> u32 {
        lockup.get_schedule_size()
    }

    pub fn get_schedule(&self, lockup: &Lockup, amount: u64) -> Vec<VestingSchedule> {
        if self.testing {
            lockup.get_testing_schedule(self.start_time, amount)
        } else {
            lockup.get_mainnet_schedule(self.start_time, amount)
        }
    }

    pub fn get_owner_address(&self, account_owner: &AccountOwner) -> Result<Pubkey, ScriptError> {
        match account_owner {
            AccountOwner::MainGovernance => {
                let realm = Realm::new(
                    self.client,
                    &self.wallet.governance_program_id,
                    REALM_NAME,
                    &self.wallet.community_pubkey,
                );
                let governance = realm.governance(&self.wallet.community_pubkey);
                Ok(governance.governance_address)
            }
            AccountOwner::EmergencyGovernance => {
                let realm = Realm::new(
                    self.client,
                    &self.wallet.governance_program_id,
                    REALM_NAME,
                    &self.wallet.governance_program_id,
                );
                let governance = realm.governance(&self.wallet.community_pubkey);
                Ok(governance.governance_address)
            }
            AccountOwner::BothGovernance => Ok(self.neon_multisig_address()),
            AccountOwner::MultiSig(msig_name, governed_address) => {
                let msig = self
                    .multi_sigs
                    .iter()
                    .find(|v| v.name == *msig_name)
                    .ok_or_else(|| StateError::UnknownMultiSig(msig_name.to_string()))?;
                if let Some(governed) = governed_address {
                    if !msig.governed_accounts.iter().any(|v| v == governed) {
                        return Err(StateError::UnknownMultiSigGoverned(
                            msig_name.to_string(),
                            *governed,
                        )
                        .into());
                    }
                }
                let seed: String = format!("MSIG_{}", msig_name);
                let msig_mint = self.account_by_seed(&seed, &spl_token::id());
                let msig_realm = Realm::new(
                    self.client,
                    &self.wallet.governance_program_id,
                    &seed,
                    &msig_mint,
                );
                let msig_governance = msig_realm.governance(&governed_address.unwrap_or(msig_mint));
                Ok(msig_governance.governance_address)
            }
            AccountOwner::Key(pubkey) => Ok(*pubkey),
        }
    }

    pub fn get_token_distribution(&self) -> Result<TokenDistribution, ScriptError> {
        let fixed_weight_addin =
            AddinFixedWeights::new(self.client, self.wallet.fixed_weight_addin_id);
        let token_distribution = TokenDistribution::new(self, &fixed_weight_addin)?;
        token_distribution.validate()?;
        Ok(token_distribution)
    }
}
