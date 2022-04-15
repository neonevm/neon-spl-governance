use {
    crate::{
        client::Client,
        token_owner::TokenOwner,
        governance::Governance,
    },
    borsh::BorshDeserialize,
    solana_sdk::{
        borsh::try_from_slice_unchecked,
        pubkey::Pubkey,
        instruction::Instruction,
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
        signature::Signature,
    },
    spl_governance::{
        state::{
            enums::MintMaxVoteWeightSource,
            realm::{RealmV2, SetRealmAuthorityAction},
            realm_config::{RealmConfigAccount, get_realm_config_address},
            token_owner_record::{TokenOwnerRecordV2, get_token_owner_record_address},
            governance::{GovernanceConfig, GovernanceV2, get_governance_address},
        },
        instruction::{
            create_token_owner_record,
            create_governance,
            create_mint_governance,
            set_realm_config,
            set_realm_authority,
        },
    },
    solana_client::{
        client_error::ClientError,
    },
    std::cell::{RefCell, Ref, RefMut},
};

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
//    authority: &'a Keypair,
//    payer_authority: &'a Keypair,
    pub client: &'a Client<'a>,
    pub address: Pubkey,
    pub community_mint: Pubkey,
    pub data: RealmV2,
    //max_voter_weight_addin_address: Option<Pubkey>,
    // voter_weight_addin_address: Option<Pubkey>,
    pub _settings: RefCell<RealmSettings>,
}

impl<'a> Realm<'a> {

    pub fn settings(&self) -> Ref<RealmSettings> {self._settings.borrow()}
    pub fn settings_mut(&self) -> RefMut<RealmSettings> {self._settings.borrow_mut()}

    pub fn get_token_owner_record_address(&self, token_owner: &Pubkey) -> Pubkey {
        get_token_owner_record_address(&self.client.spl_governance_program_address, &self.address, &self.community_mint, token_owner)
    }

    pub fn get_token_owner_record_v2(&self, token_owner: &Pubkey) -> TokenOwnerRecordV2 {
        let record_address = self.get_token_owner_record_address(token_owner);
        let mut dt: &[u8] = &self.client.solana_client.get_account_data(&record_address).unwrap();
        TokenOwnerRecordV2::deserialize(&mut dt).unwrap()
    }

    pub fn create_token_owner_record<'b:'a>(&'b self, token_owner: &Pubkey) -> Result<TokenOwner<'a>,ClientError> {
        let token_owner_record_address: Pubkey = self.get_token_owner_record_address(token_owner);

        if !self.client.account_exists(&token_owner_record_address) {
            let create_token_owner_record_instruction: Instruction =
                create_token_owner_record(
                    &self.client.spl_governance_program_address,
                    &self.address,
                    token_owner,
                    &self.community_mint,
                    &self.client.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_token_owner_record_instruction,
                    ],
                    Some(&self.client.payer.pubkey()),
                    &[
                        self.client.payer,
                    ],
                    self.client.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.client.solana_client.send_and_confirm_transaction(&transaction)?;
        }
        Ok(
            TokenOwner {
                realm: self,
                //authority: token_owner_keypair,
                token_owner: *token_owner,
                token_owner_record_address,
                token_owner_record: self.get_token_owner_record_v2(token_owner),
                // voter_weight_record_authority: None,
                voter_weight_record_address: None,
                // voter_weight_record: None,
            }
        )
    }

    pub fn get_governance_address(&self, governed_account_pubkey: &Pubkey) -> Pubkey {
        get_governance_address(&self.client.spl_governance_program_address, &self.address, governed_account_pubkey)
    }

    pub fn get_governance_v2(&self, governed_account_pubkey: &Pubkey) -> GovernanceV2 {
        let governance_pubkey: Pubkey = self.get_governance_address(governed_account_pubkey);

        let mut dt: &[u8] = &self.client.solana_client.get_account_data(&governance_pubkey).unwrap();
        GovernanceV2::deserialize(&mut dt).unwrap()
    }

    pub fn create_governance<'b:'a>(&'b self, create_authority: &Keypair, token_owner: &TokenOwner,
            governed_account_pubkey: &Pubkey, gov_config: GovernanceConfig) -> Result<Governance<'a>,ClientError>
    {
        let governance_pubkey: Pubkey = self.get_governance_address(governed_account_pubkey);

        if !self.client.account_exists(&governance_pubkey) {
            let create_governance_instruction: Instruction =
                create_governance(
                    &self.client.spl_governance_program_address,
                    &self.address,
                    Some(governed_account_pubkey),
                    &token_owner.token_owner_record_address,
                    &self.client.payer.pubkey(),
                    &create_authority.pubkey(),       // realm_authority OR token_owner authority
                    token_owner.voter_weight_record_address,
                    gov_config,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_governance_instruction,
                    ],
                    Some(&self.client.payer.pubkey()),
                    &[
                        create_authority,
                        self.client.payer,
                    ],
                    self.client.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.client.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        }
        Ok(
            Governance {
                realm: self,
                address: governance_pubkey,
                data: self.get_governance_v2(governed_account_pubkey)
            }
        )
    }

    pub fn create_mint_governance<'b:'a>(&'b self, create_authority: &Keypair, token_owner: &TokenOwner,
            governed_mint: &Pubkey, governed_mint_authority: &Keypair, gov_config: GovernanceConfig,
            transfer_mint_authorities: bool) -> Result<Governance<'a>,ClientError>
    {
        let governance_pubkey: Pubkey = self.get_governance_address(governed_mint);

        if !self.client.account_exists(&governance_pubkey) {
            let create_mint_governance_instruction: Instruction =
                create_mint_governance(
                    &self.client.spl_governance_program_address,
                    &self.address,
                    &governed_mint,
                    &governed_mint_authority.pubkey(),
                    &token_owner.token_owner_record_address,
                    &self.client.payer.pubkey(),
                    &create_authority.pubkey(),       // realm_authority OR token_owner authority
                    token_owner.voter_weight_record_address,
                    gov_config,
                    transfer_mint_authorities,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_mint_governance_instruction,
                    ],
                    Some(&self.client.payer.pubkey()),
                    &[
                        create_authority,
                        self.client.payer,
                    ],
                    self.client.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.client.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        }
        Ok(
            Governance {
                realm: self,
                address: governance_pubkey,
                data: self.get_governance_v2(governed_mint)
            }
        )
    }

    pub fn get_realm_config(&self) -> Result<RealmConfigAccount,ClientError> {
        let realm_config_address = get_realm_config_address(&self.client.spl_governance_program_address, &self.address);
        let realm_config_data = self.client.solana_client.get_account_data(&realm_config_address)?;
        let realm_config: RealmConfigAccount = try_from_slice_unchecked(&realm_config_data).unwrap();
        Ok(realm_config)
    }

    pub fn set_realm_config_instruction(&self, realm_authority: &Pubkey, realm_config: &RealmConfig) -> Instruction {
        set_realm_config(
            &self.client.spl_governance_program_address,
            &self.address,
            realm_authority,
            realm_config.council_token_mint,
            &self.client.payer.pubkey(),
            realm_config.community_voter_weight_addin,
            realm_config.max_community_voter_weight_addin,
            realm_config.min_community_weight_to_create_governance,
            realm_config.community_mint_max_vote_weight_source.clone(),
        )
    }

    pub fn set_realm_config(&self, realm_authority: &Keypair, realm_config: &RealmConfig) -> Result<(),ClientError> {
        let transaction: Transaction =
            Transaction::new_signed_with_payer(
                &[
                    set_realm_config(
                        &self.client.spl_governance_program_address,
                        &self.address,
                        &realm_authority.pubkey(),
                        realm_config.council_token_mint,
                        &self.client.payer.pubkey(),
                        realm_config.community_voter_weight_addin,
                        realm_config.max_community_voter_weight_addin,
                        realm_config.min_community_weight_to_create_governance,
                        realm_config.community_mint_max_vote_weight_source.clone(),
                    ),
                ],
                Some(&self.client.payer.pubkey()),
                &[
                    realm_authority,
                    self.client.payer,
                ],
                self.client.solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.client.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        Ok(())
    }

    pub fn set_realm_authority_instruction(&self, realm_authority: &Pubkey, new_realm_authority: Option<&Pubkey>, action: SetRealmAuthorityAction) -> Instruction {
        set_realm_authority(
            &self.client.spl_governance_program_address,
            &self.address,
            &realm_authority,
            new_realm_authority,
            action
        )
    }
    pub fn set_realm_authority(&self, realm_authority: &Keypair, new_realm_authority: Option<&Pubkey>, action: SetRealmAuthorityAction) -> Result<Signature,ClientError> {
        let transaction: Transaction = 
            Transaction::new_signed_with_payer(
                &[
                    set_realm_authority(
                        &self.client.spl_governance_program_address,
                        &self.address,
                        &realm_authority.pubkey(),
                        new_realm_authority,
                        action
                    ),
                ],
                Some(&self.client.payer.pubkey()),
                &[
                    realm_authority,
                    self.client.payer,
                ],
                self.client.solana_client.get_latest_blockhash()?,
            );

        self.client.solana_client.send_and_confirm_transaction(&transaction)
    }
}
