use {
    crate::{
        client::Client,
        token_owner::TokenOwner,
        governance::Governance,
    },
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
            token_owner_record::get_token_owner_record_address,
            governance::get_governance_address,
        },
        instruction::{
            set_realm_config,
            set_realm_authority,
            create_realm,
        },
    },
    solana_client::{
        client_error::ClientError,
    },
    std::cell::{RefCell, Ref, RefMut},
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

    pub fn get_data(&self) -> Result<Option<RealmV2>, ClientError> {
        self.client.get_account_data::<RealmV2>(&self.program_id, &self.realm_address)
    }

    pub fn create_realm(&self, realm_authority: &'a Keypair, voter_weight_addin: Option<Pubkey>, max_voter_weight_addin: Option<Pubkey>) -> Result<Signature, ClientError> {
        self.client.send_and_confirm_transaction_with_payer_only(
            &[
                create_realm(
                    &self.program_id,
                    &realm_authority.pubkey(),
                    &self.community_mint,
                    &self.client.payer.pubkey(),
                    None,
                    voter_weight_addin,
                    max_voter_weight_addin,
                    self.realm_name.clone(),
                    MIN_COMMUNITY_WEIGHT_TO_CREATE_GOVERNANCE,
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                )
            ],
        )       
    }

    pub fn token_owner_record<'b:'a>(&'b self, token_owner: &Pubkey) -> TokenOwner<'a> {
        let token_owner_record_address: Pubkey = get_token_owner_record_address(
                &self.program_id,
                &self.realm_address,
                &self.community_mint, token_owner
            );
        TokenOwner {
            realm: self,
            token_owner_address: *token_owner,
            token_owner_record_address,
            voter_weight_record_address: None,
        }
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

    pub fn get_realm_config(&self) -> Result<Option<RealmConfigAccount>,ClientError> {
        let realm_config_address = get_realm_config_address(&self.program_id, &self.realm_address);
        self.client.get_account_data::<RealmConfigAccount>(&self.program_id, &realm_config_address)
    }

    pub fn set_realm_config_instruction(&self, realm_authority: &Pubkey, realm_config: &RealmConfig) -> Instruction {
        set_realm_config(
            &self.program_id,
            &self.realm_address,
            realm_authority,
            realm_config.council_token_mint,
            &self.client.payer.pubkey(),
            realm_config.community_voter_weight_addin,
            realm_config.max_community_voter_weight_addin,
            realm_config.min_community_weight_to_create_governance,
            realm_config.community_mint_max_vote_weight_source.clone(),
        )
    }

    pub fn set_realm_config(&self, realm_authority: &Keypair, realm_config: &RealmConfig) -> Result<Signature,ClientError> {
        self.client.send_and_confirm_transaction(
                &[
                    self.set_realm_config_instruction(
                            &realm_authority.pubkey(),
                            realm_config,
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
    pub fn set_realm_authority(&self, realm_authority: &Keypair, new_realm_authority: Option<&Pubkey>, action: SetRealmAuthorityAction) -> Result<Signature,ClientError> {
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
