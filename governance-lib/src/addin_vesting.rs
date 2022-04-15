use {
    crate::{
        client::Client,
        realm::Realm,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signer::{Signer, keypair::Keypair},
        instruction::Instruction,
        transaction::Transaction,
        program_error::ProgramError,
    },
    spl_governance_addin_vesting::{
        state::VestingSchedule,
        instruction::{deposit, deposit_with_realm},
        voter_weight::get_voter_weight_record_address,
    },
    solana_client::{
        client_error::ClientError,
    },
};

#[derive(Debug)]
pub struct AddinVesting<'a> {
    client: &'a Client<'a>,
    pub program_id: Pubkey,
}


impl<'a> AddinVesting<'a> {
    pub fn new(client: &'a Client, program_id: Pubkey) -> Self {
        AddinVesting {
            client,
            program_id,
        }
    }

    pub fn find_vesting_account(&self, vesting_token_account: &Pubkey) -> Pubkey {
        let (vesting_account,_) = Pubkey::find_program_address(
                &[&vesting_token_account.as_ref()],
                &self.program_id,
            );
        vesting_account
    }

    pub fn get_voter_weight_record_address(&self, owner: &Pubkey, realm: &Realm) -> Pubkey {
        get_voter_weight_record_address(
                &self.program_id,
                &realm.address,
                &realm.community_mint,
                owner)
    }

    pub fn deposit(&self, source_token_authority: &Pubkey, source_token_account: &Pubkey,
            vesting_owner: &Pubkey, vesting_token_account: &Pubkey, schedules: Vec<VestingSchedule>) -> Result<Instruction, ProgramError>
    {
        deposit(
            &self.program_id,
            &spl_token::id(),
            vesting_token_account,
            &source_token_authority,
            source_token_account,
            vesting_owner,
            &self.client.payer.pubkey(),
            schedules,
        )
    }

    pub fn deposit_with_realm_instruction(&self, source_token_authority: &Pubkey, source_token_account: &Pubkey,
            vesting_owner: &Pubkey, vesting_token_account: &Pubkey, schedules: Vec<VestingSchedule>, realm: &Realm) -> Result<Instruction, ProgramError>
    {
        deposit_with_realm(
            &self.program_id,
            &spl_token::id(),
            vesting_token_account,
            &source_token_authority,
            source_token_account,
            vesting_owner,
            &self.client.payer.pubkey(),
            schedules,
            &realm.program_id,
            &realm.address,
            &realm.community_mint,
        )
    }

/*    pub fn setup_max_voter_weight_record(&self, realm: &Realm) -> Result<Pubkey, ()> {
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
                    &realm.data.community_mint,
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
    }*/
}
