use {
    crate::{
        client::SplGovernanceInteractor,
        token_owner::TokenOwner,
        governance::Governance,
    },
    borsh::BorshDeserialize,
    solana_sdk::{
        pubkey::Pubkey,
        instruction::Instruction,
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
    },
    spl_governance::{
        state::{
            realm::RealmV2,
            token_owner_record::{TokenOwnerRecordV2, get_token_owner_record_address},
            governance::{GovernanceConfig, GovernanceV2, get_governance_address},
        },
        instruction::{
            create_token_owner_record,
            create_governance,
        },
    },
    solana_client::{
        client_error::ClientError,
    },
};

#[derive(Debug)]
pub struct Realm<'a> {
//    authority: &'a Keypair,
//    payer_authority: &'a Keypair,
    pub interactor: &'a SplGovernanceInteractor<'a>,
    pub address: Pubkey,
    pub community_mint: Pubkey,
    pub data: RealmV2,
    //max_voter_weight_addin_address: Option<Pubkey>,
    pub max_voter_weight_record_address: Option<Pubkey>,
    // voter_weight_addin_address: Option<Pubkey>,
}

impl<'a> Realm<'a> {

    pub fn set_max_voter_weight_record_address(&mut self, max_voter_weight_record_address: Option<Pubkey>) {
        self.max_voter_weight_record_address = max_voter_weight_record_address;
    }

    pub fn get_token_owner_record_address(&self, token_owner: &Pubkey) -> Pubkey {
        get_token_owner_record_address(&self.interactor.spl_governance_program_address, &self.address, &self.community_mint, token_owner)
    }

    pub fn get_token_owner_record_v2(&self, token_owner: &Pubkey) -> TokenOwnerRecordV2 {
        let record_address = self.get_token_owner_record_address(token_owner);
        let mut dt: &[u8] = &self.interactor.solana_client.get_account_data(&record_address).unwrap();
        TokenOwnerRecordV2::deserialize(&mut dt).unwrap()
    }

    pub fn create_token_owner_record<'b:'a>(&'b self, token_owner: &Pubkey) -> Result<TokenOwner<'a>,ClientError> {
        let token_owner_record_address: Pubkey = self.get_token_owner_record_address(token_owner);

        if !self.interactor.account_exists(&token_owner_record_address) {
            let create_token_owner_record_instruction: Instruction =
                create_token_owner_record(
                    &self.interactor.spl_governance_program_address,
                    &self.address,
                    token_owner,
                    &self.community_mint,
                    &self.interactor.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_token_owner_record_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        self.interactor.payer,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction)?;
        }
        Ok(
            TokenOwner {
                realm: self,
                //authority: token_owner_keypair,
                token_owner_record_address,
                token_owner_record: self.get_token_owner_record_v2(token_owner),
                // voter_weight_record_authority: None,
                voter_weight_record_address: None,
                // voter_weight_record: None,
            }
        )
    }

    pub fn get_governance_address(&self, governed_account_pubkey: &Pubkey) -> Pubkey {
        get_governance_address(&self.interactor.spl_governance_program_address, &self.address, governed_account_pubkey)
    }

    pub fn get_governance_v2(&self, governed_account_pubkey: &Pubkey) -> GovernanceV2 {
        let governance_pubkey: Pubkey = self.get_governance_address(governed_account_pubkey);

        let mut dt: &[u8] = &self.interactor.solana_client.get_account_data(&governance_pubkey).unwrap();
        GovernanceV2::deserialize(&mut dt).unwrap()
    }

    pub fn create_governance<'b:'a>(&'b self, create_authority: &Keypair, token_owner: &TokenOwner, governed_account_pubkey: &Pubkey, gov_config: GovernanceConfig) -> Result<Governance<'a>,ClientError> {
        let governance_pubkey: Pubkey = self.get_governance_address(governed_account_pubkey);

        if !self.interactor.account_exists(&governance_pubkey) {
            let create_governance_instruction: Instruction =
                create_governance(
                    &self.interactor.spl_governance_program_address,
                    &self.address,
                    Some(governed_account_pubkey),
                    &token_owner.token_owner_record_address,
                    &self.interactor.payer.pubkey(),
                    &create_authority.pubkey(),       // realm_authority OR token_owner authority
                    token_owner.voter_weight_record_address,
                    gov_config,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        create_governance_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        create_authority,
                        self.interactor.payer,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        }
        Ok(
            Governance {
                realm: self,
                address: governance_pubkey,
                data: self.get_governance_v2(governed_account_pubkey)
            }
        )
    }
}
