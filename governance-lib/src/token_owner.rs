use {
    crate::{
        realm::Realm,
    },
    solana_sdk::{
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
        client_error::Result as ClientResult,
    },
    std::fmt,
};

#[derive(Debug)]
pub struct TokenOwner<'a> {
    pub realm: &'a Realm<'a>,
    pub token_owner_address: Pubkey,
    pub token_owner_record_address: Pubkey,
    pub voter_weight_record_address: Option<Pubkey>,
}

impl<'a> fmt::Display for TokenOwner<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TokenOwner")
            .field("client", self.realm.client)
            .field("realm", &self.realm.realm_address)
            .field("token_owner", &self.token_owner_address)
            .field("token_owner_record", &self.token_owner_record_address)
            .field("voter_weight_record", &self.voter_weight_record_address)
            .finish()
    }
}

impl<'a> TokenOwner<'a> {

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
                        &self.realm.realm_address,
                        &self.token_owner_address,
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
                        &self.realm.realm_address,
                        &self.realm.community_mint,
                        &self.token_owner_address,
                        new_delegate,
                    ),
                ],
                &[authority],
            )
    }
}
