use {
    crate::realm::Realm,
    solana_sdk::pubkey::Pubkey,
    spl_governance::state::token_owner_record::TokenOwnerRecordV2,
};

#[derive(Debug)]
pub struct TokenOwner<'a> {
    pub realm: &'a Realm<'a>,
    pub token_owner_record_address: Pubkey,
    pub token_owner_record: TokenOwnerRecordV2,
    pub voter_weight_record_address: Option<Pubkey>,
}

impl<'a> TokenOwner<'a> {
    pub fn set_voter_weight_record_address(&mut self, voter_weight_record_address: Option<Pubkey>) {
        self.voter_weight_record_address = voter_weight_record_address;
    }
}
