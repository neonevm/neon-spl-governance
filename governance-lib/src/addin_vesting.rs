use {
    crate::{
        client::Client,
        realm::Realm,
    },
    borsh::{BorshSerialize},
    solana_sdk::{
        pubkey::Pubkey,
        signer::Signer,
        instruction::Instruction,
        program_error::ProgramError,
    },
    spl_governance_addin_vesting::{
        state::{
            VestingAccountType,
            VestingSchedule,
            VestingRecord,
        },
        instruction::{deposit, deposit_with_realm},
        voter_weight::{
            VoterWeightRecord,
            ExtendedVoterWeightRecord,
            get_voter_weight_record_address,
        },
        max_voter_weight::{
            MaxVoterWeightRecord,
            get_max_voter_weight_record_address,
        },
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

    pub fn get_vesting_account_size(&self, number_of_schedules: u32, realm: bool) -> usize {
        let record = VestingRecord {
            account_type: VestingAccountType::VestingRecord,
            owner: Pubkey::default(),
            mint: Pubkey::default(),
            token: Pubkey::default(),
            realm: if realm {Some(Pubkey::default())} else {None},
            schedule: Vec::new(),
        };
        let schedule = VestingSchedule {
            release_time: 0,
            amount: 0,
        };
        record.try_to_vec().unwrap().len() + number_of_schedules as usize * schedule.try_to_vec().unwrap().len()
    }

    pub fn get_voter_weight_account_size(&self) -> usize {
        let record_data = ExtendedVoterWeightRecord {
            base: VoterWeightRecord {
                account_discriminator: VoterWeightRecord::ACCOUNT_DISCRIMINATOR,
                realm: Pubkey::default(),
                governing_token_mint: Pubkey::default(),
                governing_token_owner: Pubkey::default(),
                voter_weight: 0,
                voter_weight_expiry: None,
                weight_action: None,
                weight_action_target: None,
                reserved: [0u8; 8],
            },
            account_discriminator: ExtendedVoterWeightRecord::ACCOUNT_DISCRIMINATOR,
            total_amount: 0,
            vote_percentage: 0,
        };
        record_data.try_to_vec().unwrap().len()
    }

    pub fn get_max_voter_weight_account_size(&self) -> usize {
        let record_data = MaxVoterWeightRecord {
            account_discriminator: MaxVoterWeightRecord::ACCOUNT_DISCRIMINATOR,
            realm: Pubkey::default(),
            governing_token_mint: Pubkey::default(),
            max_voter_weight: 0,
            max_voter_weight_expiry: None,
            reserved: [0u8; 8],
        };
        record_data.try_to_vec().unwrap().len()
    }

    pub fn get_voter_weight_record_address(&self, owner: &Pubkey, realm: &Realm) -> Pubkey {
        get_voter_weight_record_address(
                &self.program_id,
                &realm.realm_address,
                &realm.community_mint,
                owner)
    }

    pub fn get_max_voter_weight_record_address(&self, realm: &Realm) -> Pubkey {
        get_max_voter_weight_record_address(
                &self.program_id,
                &realm.realm_address,
                &realm.community_mint)
    }

    pub fn deposit_instruction(&self, source_token_authority: &Pubkey, source_token_account: &Pubkey,
            vesting_owner: &Pubkey, vesting_token_account: &Pubkey, schedules: Vec<VestingSchedule>, payer: Option<Pubkey>) -> Result<Instruction, ProgramError>
    {
        deposit(
            &self.program_id,
            &spl_token::id(),
            vesting_token_account,
            &source_token_authority,
            source_token_account,
            vesting_owner,
            &payer.unwrap_or(self.client.payer.pubkey()),
            schedules,
        )
    }

    pub fn deposit_with_realm_instruction(&self, source_token_authority: &Pubkey, source_token_account: &Pubkey,
            vesting_owner: &Pubkey, vesting_token_account: &Pubkey, schedules: Vec<VestingSchedule>, realm: &Realm, payer: Option<Pubkey>) -> Result<Instruction, ProgramError>
    {
        deposit_with_realm(
            &self.program_id,
            &spl_token::id(),
            vesting_token_account,
            &source_token_authority,
            source_token_account,
            vesting_owner,
            &payer.unwrap_or(self.client.payer.pubkey()),
            schedules,
            &realm.program_id,
            &realm.realm_address,
            &realm.community_mint,
        )
    }
}
