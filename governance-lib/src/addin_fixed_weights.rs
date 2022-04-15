use {
    crate::{
        client::Client,
        realm::Realm,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signer::Signer,
        instruction::Instruction,
        transaction::Transaction,
    },
};

#[derive(Debug)]
pub struct AddinFixedWeights<'a> {
    client: &'a Client<'a>,
    pub program_id: Pubkey,
}

impl<'a> AddinFixedWeights<'a> {
    pub fn new(client: &'a Client, program_id: Pubkey) -> Self {
        AddinFixedWeights {
            client,
            program_id,
        }
    }

    pub fn setup_max_voter_weight_record(&self, realm: &Realm) -> Result<Pubkey, ()> {
        use spl_governance_addin_fixed_weights::instruction;
        let (max_voter_weight_record_pubkey,_): (Pubkey,u8) = instruction::get_max_voter_weight_address(
                &self.program_id,
                &realm.address,
                &realm.community_mint,
            );

        if !self.client.account_exists(&max_voter_weight_record_pubkey) {
            let setup_max_voter_weight_record_instruction: Instruction =
                instruction::setup_max_voter_weight_record(
                    &self.program_id,
                    &realm.address,
                    &realm.community_mint,
                    &self.client.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_max_voter_weight_record_instruction,
                    ],
                    Some(&self.client.payer.pubkey()),
                    &[
                        self.client.payer,
                    ],
                    self.client.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.client.solana_client.send_and_confirm_transaction(&transaction)
                .map_err(|_|())?;
        }
        Ok(max_voter_weight_record_pubkey)
    }

    pub fn setup_voter_weight_record(&self, realm: &Realm, token_owner: &Pubkey) -> Result<Pubkey,()> {
        let (voter_weight_record_pubkey,_): (Pubkey,u8) = spl_governance_addin_fixed_weights::instruction::get_voter_weight_address(
                &self.program_id,
                &realm.address,
                &realm.community_mint,
                token_owner);

        if !self.client.account_exists(&voter_weight_record_pubkey) {
            let setup_voter_weight_record_instruction: Instruction =
                spl_governance_addin_fixed_weights::instruction::setup_voter_weight_record(
                    &self.program_id,
                    &realm.address,
                    &realm.community_mint,
                    token_owner,
                    &self.client.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_voter_weight_record_instruction,
                    ],
                    Some(&self.client.payer.pubkey()),
                    &[
                        self.client.payer,
                    ],
                    self.client.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.client.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        }
        Ok(voter_weight_record_pubkey)
    }
}
