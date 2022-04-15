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
        instruction::set_governance_delegate,
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
    pub token_owner_record: TokenOwnerRecordV2,
    pub voter_weight_record_address: Option<Pubkey>,
}

impl<'a> TokenOwner<'a> {
    fn get_client(&self) -> &Client<'a> {self.realm.client}

    pub fn set_voter_weight_record_address(&mut self, voter_weight_record_address: Option<Pubkey>) {
        self.voter_weight_record_address = voter_weight_record_address;
    }

    pub fn set_delegate(&self, authority: &Keypair, new_delegate: &Option<Pubkey>) -> ClientResult<Signature> {
        let payer = self.get_client().payer;

        let transaction: Transaction = Transaction::new_signed_with_payer(
            &[
                set_governance_delegate(
                    &self.get_client().spl_governance_program_address,
                    &authority.pubkey(),
                    &self.realm.address,
                    &self.realm.community_mint,
                    &self.token_owner,
                    new_delegate,
                ),
            ],
            Some(&payer.pubkey()),
            &[
                payer,
                authority,
            ],
            self.get_client().solana_client.get_latest_blockhash().unwrap(),
        );

        println!("Transaction: {:?}", transaction);

        self.get_client().solana_client.send_and_confirm_transaction(&transaction)
    }
}
