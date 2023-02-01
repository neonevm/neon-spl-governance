use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::{clock::Clock, Sysvar},
};

use borsh::BorshSerialize;
use spl_token::{
    instruction::{
        close_account,
        set_authority,
        AuthorityType,
    },
    state::Account,
};
use spl_governance_tools::account::{
    get_account_data,
    create_and_serialize_account_signed,
    dispose_account,
};
use spl_governance::state::{
    realm::get_realm_data,
    token_owner_record::{
        get_token_owner_record_address_seeds,
        get_token_owner_record_data_for_seeds,
    },
};

use crate::{
    error::VestingError,
    instruction::VestingInstruction,
    state::{VestingAccountType, VestingRecord, VestingSchedule},
    voter_weight::{
        create_voter_weight_record,
        get_voter_weight_record_data_checked,
    },
    max_voter_weight::{
        create_max_voter_weight_record,
        get_max_voter_weight_record_data_checked,
    },
    token_owner_record::{
        get_token_owner_record_data_if_exists,
    },
};

pub struct Processor {}

impl Processor {

    pub fn process_deposit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        schedules: Vec<VestingSchedule>,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let system_program_account = next_account_info(accounts_iter)?;
        let spl_token_account = next_account_info(accounts_iter)?;
        let vesting_account = next_account_info(accounts_iter)?;
        let vesting_token_account = next_account_info(accounts_iter)?;
        let source_token_account_owner = next_account_info(accounts_iter)?;
        let source_token_account = next_account_info(accounts_iter)?;
        let vesting_owner_account = next_account_info(accounts_iter)?;
        let payer_account = next_account_info(accounts_iter)?;

        let realm_info = if let Some(realm) = accounts_iter.next() {
            let voter_weight = next_account_info(accounts_iter)?;
            let max_voter_weight = next_account_info(accounts_iter)?;
            Some((realm, voter_weight, max_voter_weight,))
        } else {
            None
        };

        if !source_token_account_owner.is_signer {
            return Err(VestingError::MissingRequiredSigner.into());
        }

        verify_schedule(&schedules)?;

        let vesting_token_account_data = Account::unpack(&vesting_token_account.data.borrow())?;
        verify_token_account_owned_by_vesting(vesting_account, vesting_token_account_data)?;

        let total_amount = schedules.iter()
                .try_fold(0u64, |acc, item| acc.checked_add(item.amount))
                .ok_or(VestingError::OverflowAmount)?;
        
        let vesting_record = VestingRecord {
            account_type: VestingAccountType::VestingRecord,
            owner: *vesting_owner_account.key,
            mint: vesting_token_account_data.mint,
            token: *vesting_token_account.key,
            realm: realm_info.map(|v| *v.0.key),
            schedule: schedules
        };
        create_and_serialize_account_signed::<VestingRecord>(
            payer_account,
            vesting_account,
            &vesting_record,
            &[vesting_token_account.key.as_ref()],
            program_id,
            system_program_account,
            &Rent::get()?,
        )?;

        if Account::unpack(&source_token_account.data.borrow())?.amount < total_amount {
            return Err(VestingError::InsufficientFunds.into());
        };

        invoke_transfer_signed(
            spl_token_account,
            source_token_account,
            vesting_token_account,
            source_token_account_owner,
            total_amount,
            &[]
        )?;

        if let Some((realm_account, voter_weight_record_account, max_voter_weight_record_account)) = realm_info {
            create_or_increase_voter_weight_record(
                realm_account.key,
                &vesting_token_account_data.mint,
                vesting_owner_account.key,
                voter_weight_record_account,
                total_amount,
                program_id,
                system_program_account,
                payer_account
            )?;
            
            create_or_increase_max_voter_weight_record(
                realm_account.key,
                &vesting_token_account_data.mint,
                max_voter_weight_record_account,
                total_amount,
                program_id,
                system_program_account,
                payer_account
            )?;
        }

        Ok(())
    }

    pub fn process_withdraw(
        program_id: &Pubkey,
        _accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut _accounts.iter();

        let spl_token_account = next_account_info(accounts_iter)?;
        let vesting_account = next_account_info(accounts_iter)?;
        let vesting_token_account = next_account_info(accounts_iter)?;
        let destination_token_account = next_account_info(accounts_iter)?;
        let vesting_owner_account = next_account_info(accounts_iter)?;

        let realm_info = if let Some(governance) = accounts_iter.next() {
            let realm = next_account_info(accounts_iter)?;
            let owner_record = next_account_info(accounts_iter)?;
            let voter_weight = next_account_info(accounts_iter)?;
            let max_voter_weight = next_account_info(accounts_iter)?;
            Some((governance, realm, owner_record, voter_weight, max_voter_weight,))
        } else {
            None
        };

        let (vesting_account_key,vesting_account_seed) = Pubkey::find_program_address(&[vesting_token_account.key.as_ref()], program_id);
        if vesting_account_key != *vesting_account.key {
            return Err(VestingError::InvalidVestingAccount.into());
        }

        let mut vesting_record = get_account_data::<VestingRecord>(program_id, vesting_account)?;
        let vesting_token_account_data = Account::unpack(&vesting_token_account.data.borrow())?;
        verify_vesting_owner(&vesting_record, vesting_owner_account)?;
        verify_vesting_token_account(&vesting_record, vesting_token_account, vesting_token_account_data, vesting_account_key)?;

        // Unlock the schedules that have reached maturity
        let clock = Clock::get()?;
        let mut total_amount_to_transfer = 0u64;
        for s in vesting_record.schedule.iter_mut() {
            if clock.unix_timestamp as u64 >= s.release_time {
                total_amount_to_transfer = total_amount_to_transfer.checked_add(s.amount)
                        .ok_or(VestingError::OverflowAmount)?;
                s.amount = 0;
            }
        }
        if total_amount_to_transfer == 0 {
            return Err(VestingError::NotReachedReleaseTime.into());
        }

        invoke_transfer_signed(
            spl_token_account,
            vesting_token_account,
            destination_token_account,
            vesting_account,
            total_amount_to_transfer,
            &[&[vesting_token_account.key.as_ref(), &[vesting_account_seed]]],
        )?;

        // Reset released amounts to 0. This makes the simple unlock safe with complex scheduling contracts
        vesting_record.serialize(&mut *vesting_account.data.borrow_mut())?;

        if let Some(expected_realm_account) = vesting_record.realm {
            let (governance_account,
                 realm_account,
                 owner_record_account,
                 voter_weight_record_account,
                 max_voter_weight_record_account) = realm_info.ok_or(VestingError::MissingRealmAccounts)?;

            if *realm_account.key != expected_realm_account {
                return Err(VestingError::InvalidRealmAccount.into())
            };

            let realm_data = get_realm_data(governance_account.key, realm_account)?;
            realm_data.assert_is_valid_governing_token_mint(&vesting_record.mint)?;

            let owner_record_optional_data = get_token_owner_record_data_if_exists(
                governance_account.key,
                owner_record_account,
                &get_token_owner_record_address_seeds(
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key,
                ),
            )?;
            if let Some(owner_record_data) = owner_record_optional_data {
                owner_record_data.assert_can_withdraw_governing_tokens()?;
            }

            let mut voter_weight_record = get_voter_weight_record_data_checked(
                    program_id,
                    voter_weight_record_account,
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key)?;

            voter_weight_record.decrease_total_amount(total_amount_to_transfer)?;
            voter_weight_record.serialize(&mut *voter_weight_record_account.data.borrow_mut())?;

            let mut max_voter_weight_record = get_max_voter_weight_record_data_checked(
                    program_id,
                    max_voter_weight_record_account,
                    realm_account.key,
                    &vesting_record.mint)?;

            let max_voter_weight = &mut max_voter_weight_record.max_voter_weight;
            *max_voter_weight = max_voter_weight.checked_sub(total_amount_to_transfer).ok_or(VestingError::UnderflowAmount)?;
            max_voter_weight_record.serialize(&mut *max_voter_weight_record_account.data.borrow_mut())?;
        }

        Ok(())
    }

    pub fn process_change_owner(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let vesting_account = next_account_info(accounts_iter)?;
        let vesting_owner_account = next_account_info(accounts_iter)?;
        let new_vesting_owner_account = next_account_info(accounts_iter)?;
        let realm_info = if let Some(governance) = accounts_iter.next() {
            let realm = next_account_info(accounts_iter)?;
            let current_owner_record = next_account_info(accounts_iter)?;
            let current_voter_weight = next_account_info(accounts_iter)?;
            let new_voter_weight = next_account_info(accounts_iter)?;
            Some((governance, realm, current_owner_record, current_voter_weight, new_voter_weight,))
        } else {
            None
        };

        msg!("Change owner {} -> {}", vesting_owner_account.key, new_vesting_owner_account.key);

        let mut vesting_record = get_account_data::<VestingRecord>(program_id, vesting_account)?;
        verify_vesting_owner(&vesting_record, vesting_owner_account)?;

        let total_amount = vesting_record.schedule.iter()
                .try_fold(0u64, |acc, item| acc.checked_add(item.amount))
                .ok_or(VestingError::OverflowAmount)?;

        vesting_record.owner = *new_vesting_owner_account.key;
        vesting_record.serialize(&mut *vesting_account.data.borrow_mut())?;

        if let Some(expected_realm_account) = vesting_record.realm {
            let (governance_account,
                 realm_account,
                 owner_record_account,
                 voter_weight_record_account,
                 new_voter_weight_record_account) = realm_info.ok_or(VestingError::MissingRealmAccounts)?;

            if *realm_account.key != expected_realm_account {
                return Err(VestingError::InvalidRealmAccount.into())
            };

            let realm_data = get_realm_data(governance_account.key, realm_account)?;
            realm_data.assert_is_valid_governing_token_mint(&vesting_record.mint)?;

            let owner_record_optional_data = get_token_owner_record_data_if_exists(
                governance_account.key,
                owner_record_account,
                &get_token_owner_record_address_seeds(
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key,
                ),
            )?;
            if let Some(owner_record_data) = owner_record_optional_data {
                owner_record_data.assert_can_withdraw_governing_tokens()?;
            }

            let mut voter_weight_record = get_voter_weight_record_data_checked(
                    program_id,
                    voter_weight_record_account,
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key)?;

            voter_weight_record.decrease_total_amount(total_amount)?;
            voter_weight_record.serialize(&mut *voter_weight_record_account.data.borrow_mut())?;

            let mut new_voter_weight_record = get_voter_weight_record_data_checked(
                    program_id,
                    new_voter_weight_record_account,
                    realm_account.key,
                    &vesting_record.mint,
                    new_vesting_owner_account.key)?;

            new_voter_weight_record.increase_total_amount(total_amount)?;
            new_voter_weight_record.serialize(&mut *new_voter_weight_record_account.data.borrow_mut())?;

        }

        Ok(())
    }

    pub fn process_create_voter_weight_record(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let system_program_account = next_account_info(accounts_iter)?;
        let record_owner_account = next_account_info(accounts_iter)?;
        let payer_account = next_account_info(accounts_iter)?;
        let realm_account = next_account_info(accounts_iter)?;
        let mint_account = next_account_info(accounts_iter)?;
        let voter_weight_record_account = next_account_info(accounts_iter)?;

        create_voter_weight_record(
            program_id,
            realm_account.key,
            mint_account.key,
            record_owner_account.key,
            payer_account,
            voter_weight_record_account,
            system_program_account,
            |_| {Ok(())},
        )?;

        Ok(())
    }

    pub fn process_set_vote_percentage(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        vote_percentage: u16,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let vesting_mint_account = next_account_info(accounts_iter)?;
        let vesting_owner_account = next_account_info(accounts_iter)?;
        let vesting_authority_account = next_account_info(accounts_iter)?;
        let governance_account = next_account_info(accounts_iter)?;
        let realm_account = next_account_info(accounts_iter)?;
        let owner_record_account = next_account_info(accounts_iter)?;
        let voter_weight_record_account = next_account_info(accounts_iter)?;

        let realm_data = get_realm_data(governance_account.key, realm_account)?;
        realm_data.assert_is_valid_governing_token_mint(vesting_mint_account.key)?;

        let owner_record_data = get_token_owner_record_data_for_seeds(
            governance_account.key,
            owner_record_account,
            &get_token_owner_record_address_seeds(
                realm_account.key,
                vesting_mint_account.key,
                vesting_owner_account.key,
            ),
        )?;
        owner_record_data.assert_token_owner_or_delegate_is_signer(vesting_authority_account)?;

        let mut voter_weight_record = get_voter_weight_record_data_checked(
                program_id,
                voter_weight_record_account,
                realm_account.key,
                vesting_mint_account.key,
                vesting_owner_account.key)?;

        voter_weight_record.set_vote_percentage(vote_percentage)?;
        voter_weight_record.serialize(&mut *voter_weight_record_account.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_close(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let spl_token_account = next_account_info(accounts_iter)?;
        let vesting_account = next_account_info(accounts_iter)?;
        let vesting_token_account = next_account_info(accounts_iter)?;
        let vesting_owner_account = next_account_info(accounts_iter)?;
        let spill_account = next_account_info(accounts_iter)?;

        let (vesting_account_key, vesting_account_seed) = Pubkey::find_program_address(&[vesting_token_account.key.as_ref()], program_id);
        if vesting_account_key != *vesting_account.key {
            return Err(VestingError::InvalidVestingAccount.into());
        }

        let mut vesting_record = get_account_data::<VestingRecord>(program_id, vesting_account)?;
        let vesting_token_account_data = Account::unpack(&vesting_token_account.data.borrow())?;
        verify_vesting_owner(&vesting_record, vesting_owner_account)?;
        verify_vesting_token_account(&vesting_record, vesting_token_account, vesting_token_account_data, vesting_account_key)?;

        let mut total_amount = 0u64;
        for s in vesting_record.schedule.iter_mut() {
            total_amount = total_amount.checked_add(s.amount).ok_or(VestingError::OverflowAmount)?;
        }
        if total_amount != 0 {
            return Err(VestingError::VestingNotEmpty.into());
        }

        let release_token_account_instruction = if vesting_token_account_data.amount == 0 {
            close_account(
                spl_token_account.key,
                vesting_token_account.key,
                spill_account.key,
                vesting_account.key,
                &[],
            )?
        } else {
            set_authority(
                spl_token_account.key,
                vesting_token_account.key,
                Some(vesting_owner_account.key),
                AuthorityType::AccountOwner,
                vesting_account.key,
                &[],
            )?
        };

        invoke_signed(
            &release_token_account_instruction,
            &[
                spl_token_account.clone(),
                vesting_token_account.clone(),
                spill_account.clone(),
                vesting_account.clone(),
            ],
            &[&[vesting_token_account.key.as_ref(), &[vesting_account_seed]]],
        )?;

        dispose_account(vesting_account, spill_account);

        Ok(())
    }

    pub fn process_close_voter_weight_record(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let owner_account = next_account_info(accounts_iter)?;
        let realm_account = next_account_info(accounts_iter)?;
        let mint_account = next_account_info(accounts_iter)?;
        let voter_weight_record_account = next_account_info(accounts_iter)?;
        let spill_account = next_account_info(accounts_iter)?;

        if !owner_account.is_signer {
            return Err(VestingError::MissingRequiredSigner.into());
        }

        let voter_weight_record = get_voter_weight_record_data_checked(
                program_id,
                voter_weight_record_account,
                realm_account.key,
                mint_account.key,
                owner_account.key)?;

        if voter_weight_record.total_amount != 0 {
            return Err(VestingError::VoterWeightRecordNotEmpty.into());
        }

        dispose_account(voter_weight_record_account, spill_account);

        Ok(())
    }

    pub fn process_split(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        schedules: Vec<VestingSchedule>,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let system_program_account = next_account_info(accounts_iter)?;
        let spl_token_account = next_account_info(accounts_iter)?;
        let vesting_account = next_account_info(accounts_iter)?;
        let vesting_token_account = next_account_info(accounts_iter)?;
        let vesting_owner_account = next_account_info(accounts_iter)?;
        let new_vesting_account = next_account_info(accounts_iter)?;
        let new_vesting_token_account = next_account_info(accounts_iter)?;
        let new_vesting_owner_account = next_account_info(accounts_iter)?;
        let payer_account = next_account_info(accounts_iter)?;

        let realm_info = if let Some(governance) = accounts_iter.next() {
            let realm = next_account_info(accounts_iter)?;
            let owner_record = next_account_info(accounts_iter)?;
            let voter_weight = next_account_info(accounts_iter)?;
            let new_voter_weight = next_account_info(accounts_iter)?;
            Some((governance, realm, owner_record, voter_weight, new_voter_weight,))
        } else {
            None
        };

        verify_schedule(&schedules)?;

        let (vesting_account_key, vesting_account_seed) = Pubkey::find_program_address(&[vesting_token_account.key.as_ref()], program_id);
        if vesting_account_key != *vesting_account.key {
            return Err(VestingError::InvalidVestingAccount.into());
        }

        // ================== Verify accounts related to the existing vesting =====================
        let mut vesting_record = get_account_data::<VestingRecord>(program_id, vesting_account)?;
        let vesting_token_account_data = Account::unpack(&vesting_token_account.data.borrow())?;
        verify_vesting_owner(&vesting_record, vesting_owner_account)?;
        verify_vesting_token_account(&vesting_record, vesting_token_account, vesting_token_account_data, vesting_account_key)?;

        // ================== Verify accounts related to new vesting record =======================
        let new_vesting_token_account_data = Account::unpack(&new_vesting_token_account.data.borrow())?;
        verify_token_account_owned_by_vesting(new_vesting_account, new_vesting_token_account_data)?;

        let mut total_amount_to_transfer = 0u64;
        let mut source_schedule_iterator = vesting_record.schedule.iter_mut().rev();
        let mut source_schedule = source_schedule_iterator.next().ok_or(VestingError::InsufficientFunds)?;
        for item in schedules.iter().rev() {
            let mut rest_amount = item.amount;
            total_amount_to_transfer = total_amount_to_transfer.checked_add(item.amount)
                    .ok_or(VestingError::OverflowAmount)?;
            while rest_amount != 0 {
                while item.release_time < source_schedule.release_time || source_schedule.amount == 0 {
                    source_schedule = source_schedule_iterator.next()
                        .ok_or(VestingError::InsufficientFunds)?;
                }
                let available_amount = rest_amount.min(source_schedule.amount);
                source_schedule.amount -= available_amount;
                rest_amount -= available_amount;
            }
        }

        vesting_record.serialize(&mut *vesting_account.data.borrow_mut())?;

        let new_vesting_record = VestingRecord {
            account_type: VestingAccountType::VestingRecord,
            owner: *new_vesting_owner_account.key,
            mint: new_vesting_token_account_data.mint,
            token: *new_vesting_token_account.key,
            realm: realm_info.map(|v| *v.1.key),
            schedule: schedules
        };
        create_and_serialize_account_signed::<VestingRecord>(
            payer_account,
            new_vesting_account,
            &new_vesting_record,
            &[new_vesting_token_account.key.as_ref()],
            program_id,
            system_program_account,
            &Rent::get()?,
        )?;

        invoke_transfer_signed(
            spl_token_account,
            vesting_token_account,
            new_vesting_token_account,
            vesting_account,
            total_amount_to_transfer,
            &[&[vesting_token_account.key.as_ref(), &[vesting_account_seed]]]
        )?;

        if let Some(expected_realm_account) = vesting_record.realm {
            let (governance_account, 
                 realm_account,
                 owner_record_account,
                 voter_weight_record_account,
                 new_voter_weight_record_account) = realm_info.ok_or(VestingError::MissingRealmAccounts)?;

            if *realm_account.key != expected_realm_account {
                return Err(VestingError::InvalidRealmAccount.into())
            };

            let realm_data = get_realm_data(governance_account.key, realm_account)?;
            realm_data.assert_is_valid_governing_token_mint(&vesting_record.mint)?;

            let owner_record_optional_data = get_token_owner_record_data_if_exists(
                governance_account.key,
                owner_record_account,
                &get_token_owner_record_address_seeds(
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key,
                ),
            )?;
            if let Some(owner_record_data) = owner_record_optional_data {
                owner_record_data.assert_can_withdraw_governing_tokens()?;
            }

            let mut voter_weight_record = get_voter_weight_record_data_checked(
                program_id,
                voter_weight_record_account,
                realm_account.key,
                &vesting_record.mint,
                vesting_owner_account.key)?;
            voter_weight_record.decrease_total_amount(total_amount_to_transfer)?;
            voter_weight_record.serialize(&mut *voter_weight_record_account.data.borrow_mut())?;

            create_or_increase_voter_weight_record(
                realm_account.key,
                &vesting_record.mint,
                new_vesting_owner_account.key,
                new_voter_weight_record_account,
                total_amount_to_transfer,
                program_id,
                system_program_account,
                payer_account)?;
        }

        Ok(())
    }

    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        msg!("VERSION:{:?}", env!("CARGO_PKG_VERSION"));
        let instruction: VestingInstruction =
                try_from_slice_unchecked(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;
        msg!("VESTING-INSTRUCTION: {:?}", instruction);

        match instruction {
            VestingInstruction::Deposit {schedules} => {
                Self::process_deposit(program_id, accounts, schedules)
            }
            VestingInstruction::Withdraw => {
                Self::process_withdraw(program_id, accounts)
            }
            VestingInstruction::ChangeOwner => {
                Self::process_change_owner(program_id, accounts)
            }
            VestingInstruction::CreateVoterWeightRecord => {
                Self::process_create_voter_weight_record(program_id, accounts)
            }
            VestingInstruction::SetVotePercentage {vote_percentage} => {
                Self::process_set_vote_percentage(program_id, accounts, vote_percentage)
            }
            VestingInstruction::Close => {
                Self::process_close(program_id, accounts)
            }
            VestingInstruction::CloseVoterWeightRecord => {
                Self::process_close_voter_weight_record(program_id, accounts)
            }
            VestingInstruction::Split {schedules} => {
                Self::process_split(program_id, accounts, schedules)
            }
        }
    }
}

fn invoke_transfer_signed<'a>(
        spl_token_account: &AccountInfo<'a>,
        source_account: &AccountInfo<'a>,
        destination_account: &AccountInfo<'a>,
        authority_account: &AccountInfo<'a>,
        amount_to_transfer: u64,
        signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = spl_token::instruction::transfer(
        spl_token_account.key,
        source_account.key,
        destination_account.key,
        authority_account.key,
        &[],
        amount_to_transfer,
    )?;
    invoke_signed(
        &instruction,
        &[
            spl_token_account.clone(),
            source_account.clone(),
            destination_account.clone(),
            authority_account.clone(),
        ],
        signers_seeds,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_or_increase_voter_weight_record<'a>(
        realm: &Pubkey, mint: &Pubkey, vesting_owner: &Pubkey,
        voter_weight_record_account: &AccountInfo<'a>,
        total_amount: u64,
        program_id: &Pubkey,
        system_program_account: &AccountInfo<'a>,
        payer_account: &AccountInfo<'a>,
) -> Result<(), ProgramError> {
    if voter_weight_record_account.data_is_empty() {
        create_voter_weight_record(
            program_id,
            realm,
            mint,
            vesting_owner,
            payer_account,
            voter_weight_record_account,
            system_program_account,
            |record| {record.increase_total_amount(total_amount)},
        )?;
    } else {
        let mut voter_weight_record = get_voter_weight_record_data_checked(
                program_id,
                voter_weight_record_account,
                realm,
                mint,
                vesting_owner)?;

        voter_weight_record.increase_total_amount(total_amount)?;
        voter_weight_record.serialize(&mut *voter_weight_record_account.data.borrow_mut())?;
    }
    Ok(())
}

fn create_or_increase_max_voter_weight_record<'a>(
    realm: &Pubkey, mint: &Pubkey,
    max_voter_weight_record_account: &AccountInfo<'a>,
    total_amount: u64,
    program_id: &Pubkey,
    system_program_account: &AccountInfo<'a>,
    payer_account: &AccountInfo<'a>,
) -> Result<(), ProgramError> {
    if max_voter_weight_record_account.data_is_empty() {
        create_max_voter_weight_record(
            program_id,
            realm,
            mint,
            payer_account,
            max_voter_weight_record_account,
            system_program_account,
            |record| {record.max_voter_weight = total_amount; Ok(())},
        )?;
    } else {
        let mut max_voter_weight_record = get_max_voter_weight_record_data_checked(
                program_id,
                max_voter_weight_record_account,
                realm,
                mint)?;

        let max_voter_weight = &mut max_voter_weight_record.max_voter_weight;
        *max_voter_weight = max_voter_weight.checked_add(total_amount).ok_or(VestingError::OverflowAmount)?;
        max_voter_weight_record.serialize(&mut *max_voter_weight_record_account.data.borrow_mut())?;
    }
    Ok(())
}

fn verify_token_account_owned_by_vesting(vesting_account: &AccountInfo, vesting_token_account_data: Account) -> Result<(), ProgramError> {
    if !vesting_account.data_is_empty() {
        return Err(VestingError::VestingAccountAlreadyExists.into());
    }
    if vesting_token_account_data.owner != *vesting_account.key ||
       vesting_token_account_data.delegate.is_some() ||
       vesting_token_account_data.close_authority.is_some() {
           return Err(VestingError::InvalidVestingTokenAccount.into());
    }
    Ok(())
}

fn verify_vesting_token_account(vesting_record: &VestingRecord, vesting_token_account: &AccountInfo, vesting_token_account_data: Account, vesting_account_key: Pubkey) -> Result<(), ProgramError> {
    if vesting_record.token != *vesting_token_account.key {
        return Err(VestingError::InvalidVestingTokenAccount.into());
    }
    if vesting_token_account_data.owner != vesting_account_key {
        return Err(VestingError::InvalidVestingTokenAccount.into());
    }

    Ok(())
}

fn verify_vesting_owner(vesting_record: &VestingRecord, vesting_owner_account: &AccountInfo) -> Result<(), ProgramError> {
    if !vesting_owner_account.is_signer {
        return Err(VestingError::MissingRequiredSigner.into());
    }
    if vesting_record.owner != *vesting_owner_account.key {
        return Err(VestingError::InvalidOwnerForVestingAccount.into());
    }
    Ok(())
}

fn verify_schedule(schedule: &[VestingSchedule]) -> Result<(), ProgramError> {
    let mut release_time = None;
    for item in schedule.iter() {
        match release_time {
            Some(release_time) if item.release_time <= release_time =>
                return Err(VestingError::InvalidSchedule.into()),
            _ => release_time = Some(item.release_time),
        }
    }
    Ok(())
}
