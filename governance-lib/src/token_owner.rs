use {
    crate::{
        client::Client,
        realm::Realm,
    },
    solana_sdk::{
        instruction::Instruction,
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
        pubkey::Pubkey,
        signature::Signature,
    },
    spl_governance::{
        state::token_owner_record::TokenOwnerRecordV2,
        instruction::{
            create_token_owner_record,
            set_governance_delegate,
        },
    },
    solana_client::{
        client_error::{ClientError, Result as ClientResult},
    },
    std::cell::RefCell,
};

#[derive(Debug)]
pub struct TokenOwner<'a> {
    pub realm: &'a Realm<'a>,
    pub token_owner: Pubkey,
    pub token_owner_record_address: Pubkey,
    pub voter_weight_record_address: Option<Pubkey>,
}

impl<'a> TokenOwner<'a> {
    fn get_client(&self) -> &Client<'a> {self.realm.client}

    pub fn set_voter_weight_record_address(&mut self, voter_weight_record_address: Option<Pubkey>) {
        self.voter_weight_record_address = voter_weight_record_address;
    }

    pub fn get_data(&self) -> ClientResult<Option<TokenOwnerRecordV2>> {
        self.realm.client.get_account_data::<TokenOwnerRecordV2>(
                &self.realm.program_id,
                &self.token_owner_record_address
            )
    }

    pub fn create_token_owner_record(&self) -> ClientResult<Signature> {
        self.realm.client.send_and_confirm_transaction_with_payer_only(
                &[
                    create_token_owner_record(
                        &self.realm.program_id,
                        &self.realm.address,
                        &self.token_owner,
                        &self.realm.community_mint,
                        &self.realm.client.payer.pubkey(),
                    ),
                ],
            )
    }

    pub fn set_delegate(&self, authority: &Keypair, new_delegate: &Option<Pubkey>) -> ClientResult<Signature> {
        self.realm.client.send_and_confirm_transaction(
                &[
                    set_governance_delegate(
                        &self.realm.program_id,
                        &authority.pubkey(),
                        &self.realm.address,
                        &self.realm.community_mint,
                        &self.token_owner,
                        new_delegate,
                    ),
                ],
                &[authority],
            )
    }
}
