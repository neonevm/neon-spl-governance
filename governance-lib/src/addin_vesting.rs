use {
    crate::{
        client::Client,
        realm::Realm,
    },
    solana_sdk::{
        pubkey::Pubkey,
        signer::Signer,
        instruction::Instruction,
        program_error::ProgramError,
    },
    spl_governance_addin_vesting::{
        state::VestingSchedule,
        instruction::{deposit, deposit_with_realm},
        voter_weight::get_voter_weight_record_address,
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
                &realm.realm_address,
                &realm.community_mint,
                owner)
    }

    pub fn deposit_instruction(&self, source_token_authority: &Pubkey, source_token_account: &Pubkey,
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
            &realm.realm_address,
            &realm.community_mint,
        )
    }
}
