use {
    crate::{
        errors::GovernanceLibError,
        realm::Realm,
        client::ClientResult,
    },
    borsh::{BorshSchema,BorshSerialize},
    solana_sdk::{
        signer::{Signer, keypair::Keypair},
        pubkey::Pubkey,
        instruction::Instruction,
        signature::Signature,
    },
    spl_governance::{
        state::{
            realm_config::RealmConfigAccount,
            token_owner_record::{
                TokenOwnerRecordV2,
                get_token_owner_record_address,
            },
        },
        instruction::{
            create_token_owner_record,
            set_governance_delegate,
        },
    },
    spl_governance_addin_api::voter_weight::VoterWeightRecord,
    std::fmt,
    std::cell::{RefCell, Ref, RefMut},
};
use solana_account_decoder::{UiDataSliceConfig, UiAccountEncoding};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{MemcmpEncodedBytes, RpcFilterType, Memcmp},
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
};

#[derive(Debug)]
pub struct TokenOwnerSettings {
    pub voter_weight_record_address: Option<Pubkey>,
}

impl TokenOwnerSettings {
    pub fn default() -> Self {
        Self {
            voter_weight_record_address: None,
        }
    }
}

#[derive(Debug)]
pub struct TokenOwner<'a> {
    pub realm: &'a Realm<'a>,
    pub token_owner_address: Pubkey,
    pub token_owner_record_address: Pubkey,
    _settings: RefCell<TokenOwnerSettings>,
    //pub voter_weight_record_address: Option<Pubkey>,
}

impl<'a> fmt::Display for TokenOwner<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TokenOwner")
            .field("client", self.realm.client)
            .field("realm", &self.realm.realm_address)
            .field("token_owner", &self.token_owner_address)
            .field("token_owner_record", &self.token_owner_record_address)
            .field("settings", &self._settings.borrow())
            .finish()
    }
}

impl<'a> TokenOwner<'a> {

    pub fn new(realm: &'a Realm, token_owner: &Pubkey) -> Self {
        let token_owner_record_address: Pubkey = get_token_owner_record_address(
                &realm.program_id,
                &realm.realm_address,
                &realm.community_mint,
                token_owner,
            );
        Self {
            realm,
            token_owner_address: *token_owner,
            token_owner_record_address,
            _settings: RefCell::new(TokenOwnerSettings::default()),
        }
    }

    pub fn settings(&self) -> Ref<TokenOwnerSettings> {self._settings.borrow()}
    pub fn settings_mut(&self) -> RefMut<TokenOwnerSettings> {self._settings.borrow_mut()}

    pub fn get_data(&self) -> ClientResult<Option<TokenOwnerRecordV2>> {
        self.realm.client.get_account_data_borsh::<TokenOwnerRecordV2>(
                &self.realm.program_id,
                &self.token_owner_record_address
            )
    }

    pub fn set_voter_weight_record_address(&mut self, voter_weight_record_address: Option<Pubkey>) {
        self.settings_mut().voter_weight_record_address = voter_weight_record_address;
    }

    pub fn get_voter_weight_record_address(&self) -> Option<Pubkey> {
        self.settings().voter_weight_record_address
    }

    pub fn update_voter_weight_record_address(&self) -> Result<Option<Pubkey>,GovernanceLibError> {
        #[derive(BorshSchema,BorshSerialize)]
        struct VoterWeightFilterData {
            pub account_discriminator: [u8; 8],
            pub realm: Pubkey,
            pub governing_token_mint: Pubkey,
            pub governing_token_owner: Pubkey,
        }

        let voter_weight_record_address = match self.realm.get_realm_config()? {
            Some(RealmConfigAccount {community_voter_weight_addin: Some(voter_weight_addin),..}) => {
                let filter = VoterWeightFilterData {
                    account_discriminator: VoterWeightRecord::ACCOUNT_DISCRIMINATOR,
                    realm: self.realm.realm_address,
                    governing_token_mint: self.realm.community_mint,
                    governing_token_owner: self.token_owner_address,
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
                let accounts = self.realm.client.solana_client.get_program_accounts_with_config(
                    &voter_weight_addin,
                    config,
                )?;
                if accounts.is_empty() {
                    return Err(GovernanceLibError::StateError(voter_weight_addin, 
                            format!("Missed voter_weight_record for {}", self.token_owner_address)));
                }
                Some(accounts[0].0)
            },
            _ => {None},
        };
        self.settings_mut().voter_weight_record_address = voter_weight_record_address;
        return Ok(voter_weight_record_address)
    }

    pub fn create_token_owner_record_instruction(&self) -> Instruction {
        create_token_owner_record(
            &self.realm.program_id,
            &self.realm.realm_address,
            &self.token_owner_address,
            &self.realm.community_mint,
            &self.realm.client.payer.pubkey(),
        )
    }

    pub fn create_token_owner_record(&self) -> ClientResult<Signature> {
        self.realm.client.send_and_confirm_transaction_with_payer_only(
                &[
                    self.create_token_owner_record_instruction(),
                ],
            )
    }

    pub fn set_delegate_instruction(&self, authority: &Pubkey, new_delegate: &Option<Pubkey>) -> Instruction {
        set_governance_delegate(
            &self.realm.program_id,
            &authority,
            &self.realm.realm_address,
            &self.realm.community_mint,
            &self.token_owner_address,
            new_delegate,
        )
    }

    pub fn set_delegate(&self, authority: &Keypair, new_delegate: &Option<Pubkey>) -> ClientResult<Signature> {
        self.realm.client.send_and_confirm_transaction(
                &[
                    self.set_delegate_instruction(&authority.pubkey(), new_delegate),
                ],
                &[authority],
            )
    }
}
