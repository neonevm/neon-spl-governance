use {
    crate::{
        client::SplGovernanceInteractor,
        realm::Realm,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signer::{
            Signer,
            keypair::Keypair,
        },
        instruction::Instruction,
        transaction::Transaction,
    },
};

#[derive(Debug)]
pub struct AddinFixedWeights<'a> {
    interactor: &'a SplGovernanceInteractor<'a>,
    pub program_id: Pubkey,
}

impl<'a> AddinFixedWeights<'a> {
    pub fn new(interactor: &'a SplGovernanceInteractor, program_id: Pubkey) -> Self {
        AddinFixedWeights {
            interactor,
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

        if !self.interactor.account_exists(&max_voter_weight_record_pubkey) {
            let setup_max_voter_weight_record_instruction: Instruction =
                instruction::setup_max_voter_weight_record(
                    &self.program_id,
                    &realm.address,
                    &realm.community_mint,
                    &self.interactor.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_max_voter_weight_record_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        self.interactor.payer,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction)
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

        if !self.interactor.account_exists(&voter_weight_record_pubkey) {
            let setup_voter_weight_record_instruction: Instruction =
                spl_governance_addin_fixed_weights::instruction::setup_voter_weight_record(
                    &self.program_id,
                    &realm.address,
                    &realm.data.community_mint,
                    token_owner,
                    &self.interactor.payer.pubkey(),
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        setup_voter_weight_record_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        self.interactor.payer,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction).unwrap();
        }
        
        Ok(voter_weight_record_pubkey)
    }

    pub fn set_voter_weight_partial_voting_fixed(&self, realm: &Realm, token_owner: &Keypair, percentage: u16) -> Result<Pubkey,()> {
        let (voter_weight_record_pubkey,_): (Pubkey,u8) = spl_governance_addin_fixed_weights::instruction::get_voter_weight_address(
                &self.program_id,
                &realm.address,
                &realm.community_mint,
                &token_owner.pubkey());

        if self.interactor.account_exists(&voter_weight_record_pubkey) {
            let set_partial_voting_instruction: Instruction =
                spl_governance_addin_fixed_weights::instruction::set_partial_voting(
                    &self.program_id,
                    &realm.address,
                    &realm.data.community_mint,
                    &token_owner.pubkey(),
                    &self.interactor.payer.pubkey(),
                    percentage,
                );
            
            let transaction: Transaction =
                Transaction::new_signed_with_payer(
                    &[
                        set_partial_voting_instruction,
                    ],
                    Some(&self.interactor.payer.pubkey()),
                    &[
                        self.interactor.payer,
                        token_owner,
                    ],
                    self.interactor.solana_client.get_latest_blockhash().unwrap(),
                );
            
            self.interactor.solana_client.send_and_confirm_transaction(&transaction)
                .map_err(|_|())?;
        }
        
        Ok(voter_weight_record_pubkey)
    }
}
