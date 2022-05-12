use {
    crate::{
        errors::GovernanceLibError,
        client::{Client, ClientResult},
        token_owner::TokenOwner,
        governance::Governance,
    },
    borsh::{BorshSchema,BorshSerialize},
    solana_sdk::{
        pubkey::Pubkey,
        instruction::Instruction,
        signer::{Signer, keypair::Keypair},
        signature::Signature,
    },
    spl_governance::{
        state::{
            enums::MintMaxVoteWeightSource,
            realm::{RealmV2, SetRealmAuthorityAction, get_realm_address},
            realm_config::{RealmConfigAccount, get_realm_config_address},
            governance::get_governance_address,
        },
        instruction::{
            set_realm_config,
            set_realm_authority,
            create_realm,
        },
    },
    spl_governance_addin_api::max_voter_weight::MaxVoterWeightRecord,
    std::cell::{RefCell, Ref, RefMut},
};

use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{MemcmpEncodedBytes, RpcFilterType, Memcmp},
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
};

const MIN_COMMUNITY_WEIGHT_TO_CREATE_GOVERNANCE: u64 = 1;

pub struct RealmConfig {
    pub council_token_mint: Option<Pubkey>,
    pub community_voter_weight_addin: Option<Pubkey>,
    pub max_community_voter_weight_addin: Option<Pubkey>,
    pub min_community_weight_to_create_governance: u64,
    pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,
}

#[derive(Debug)]
pub struct RealmSettings {
    pub max_voter_weight_record_address: Option<Pubkey>,
}

impl RealmSettings {
    pub fn default() -> Self {
        Self {
            max_voter_weight_record_address: None,
        }
    }
}

#[derive(Debug)]
pub struct Realm<'a> {
    pub client: &'a Client<'a>,
    pub program_id: Pubkey,
    pub realm_name: String,
    pub realm_address: Pubkey,
    pub community_mint: Pubkey,
    pub _settings: RefCell<RealmSettings>,
}

impl<'a> Realm<'a> {

    pub fn new(client: &'a Client<'a>, program_id: &Pubkey, realm_name: &str, community_mint: &Pubkey) -> Self {
        Self {
            client,
            program_id: *program_id,
            realm_name: realm_name.to_string(),
            realm_address: get_realm_address(program_id, realm_name),
            community_mint: *community_mint,
            _settings: RefCell::new(RealmSettings::default()),
        }
    }

    pub fn settings(&self) -> Ref<RealmSettings> {self._settings.borrow()}
    pub fn settings_mut(&self) -> RefMut<RealmSettings> {self._settings.borrow_mut()}

    pub fn update_max_voter_weight_record_address(&self) -> Result<Option<Pubkey>,GovernanceLibError> {
        #[derive(BorshSchema,BorshSerialize)]
        struct MaxVoterWeightFilterData {
            pub account_discriminator: [u8; 8],
            pub realm: Pubkey,
            pub governing_token_mint: Pubkey,
        }

        let max_voter_weight_record_address = match self.get_realm_config()? {
            Some(RealmConfigAccount {max_community_voter_weight_addin: Some(max_voter_weight_addin),..}) => {
                let filter = MaxVoterWeightFilterData {
                    account_discriminator: MaxVoterWeightRecord::ACCOUNT_DISCRIMINATOR,
                    realm: self.realm_address,
                    governing_token_mint: self.community_mint,
                };

                let config = RpcProgramAccountsConfig {
                    filters: Some(vec![
                        RpcFilterType::Memcmp(Memcmp {
                            offset: 0,
                            bytes: MemcmpEncodedBytes::Base58(bs58::encode(filter.try_to_vec()?).into_string()),
                            encoding: None,
                        }),
                    ]),
                    account_config: RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64),
                        data_slice: None,
                        commitment: Some(CommitmentConfig::confirmed()),
                    },
                    with_context: Some(false),
                };
                let accounts = self.client.solana_client.get_program_accounts_with_config(
                    &max_voter_weight_addin,
                    config,
                )?;
                if accounts.is_empty() {
                    return Err(GovernanceLibError::StateError(max_voter_weight_addin, "Missed max_voter_weight_record".to_string()));
                }
                Some(accounts[0].0)
            },
            _ => {None},
        };
        self.settings_mut().max_voter_weight_record_address = max_voter_weight_record_address;
        return Ok(max_voter_weight_record_address)
    }

    pub fn get_data(&self) -> ClientResult<Option<RealmV2>> {
        self.client.get_account_data_borsh::<RealmV2>(&self.program_id, &self.realm_address)
    }

    pub fn create_realm_instruction(&self, realm_authority: &Pubkey, realm_config: &RealmConfig) -> Instruction {
        create_realm(
            &self.program_id,
            &realm_authority,
            &self.community_mint,
            &self.client.payer.pubkey(),
            realm_config.council_token_mint,
            realm_config.community_voter_weight_addin,
            realm_config.max_community_voter_weight_addin,
            self.realm_name.clone(),
            realm_config.min_community_weight_to_create_governance,
            realm_config.community_mint_max_vote_weight_source.clone(),
        )
    }

    pub fn create_realm(&self, realm_authority: &'a Keypair, realm_config: &RealmConfig) -> ClientResult<Signature> {
        self.client.send_and_confirm_transaction_with_payer_only(
            &[
                self.create_realm_instruction(
                    &realm_authority.pubkey(),
                    realm_config,
                ),
            ],
        )       
    }

    pub fn token_owner_record<'b:'a>(&'b self, token_owner: &Pubkey) -> TokenOwner<'a> {
        TokenOwner::new(self, token_owner)
    }

    pub fn governance<'b:'a>(&'b self, governed_account: &Pubkey) -> Governance<'a> {
        let governance_address: Pubkey = get_governance_address(
                &self.program_id,
                &self.realm_address,
                governed_account,
            );
        Governance {
            realm: self,
            governance_address,
            governed_account: *governed_account,
        }
    }

    pub fn get_realm_config(&self) -> ClientResult<Option<RealmConfigAccount>> {
        let realm_config_address = get_realm_config_address(&self.program_id, &self.realm_address);
        self.client.get_account_data_borsh::<RealmConfigAccount>(&self.program_id, &realm_config_address)
    }

    pub fn set_realm_config_instruction(&self, realm_authority: &Pubkey, realm_config: &RealmConfig, payer: Option<Pubkey>) -> Instruction {
        set_realm_config(
            &self.program_id,
            &self.realm_address,
            realm_authority,
            realm_config.council_token_mint,
            &payer.unwrap_or(self.client.payer.pubkey()),
            realm_config.community_voter_weight_addin,
            realm_config.max_community_voter_weight_addin,
            realm_config.min_community_weight_to_create_governance,
            realm_config.community_mint_max_vote_weight_source.clone(),
        )
    }

    pub fn set_realm_config(&self, realm_authority: &Keypair, realm_config: &RealmConfig) -> ClientResult<Signature> {
        self.client.send_and_confirm_transaction(
                &[
                    self.set_realm_config_instruction(
                            &realm_authority.pubkey(),
                            realm_config,
                            None,   // default payer
                        ),
                ],
                &[realm_authority],
            )
    }

    pub fn set_realm_authority_instruction(&self, realm_authority: &Pubkey, new_realm_authority: Option<&Pubkey>, action: SetRealmAuthorityAction) -> Instruction {
        set_realm_authority(
            &self.program_id,
            &self.realm_address,
            &realm_authority,
            new_realm_authority,
            action
        )
    }
    pub fn set_realm_authority(&self, realm_authority: &Keypair, new_realm_authority: Option<&Pubkey>, action: SetRealmAuthorityAction) -> ClientResult<Signature> {
        self.client.send_and_confirm_transaction(
                &[
                    self.set_realm_authority_instruction(
                            &realm_authority.pubkey(),
                            new_realm_authority,
                            action,
                        ),
                ],
                &[realm_authority],
            )
    }
}
