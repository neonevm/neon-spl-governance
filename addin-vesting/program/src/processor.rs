use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::{clock::Clock, Sysvar},
};

use num_traits::FromPrimitive;
use borsh::BorshSerialize;
use spl_token::{instruction::transfer, state::Account};
use spl_governance_tools::account::{
    get_account_data,
    create_and_serialize_account_signed,
};
use spl_governance::state::{
    realm::{get_realm_address_seeds, get_realm_data},
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
        VoterWeightRecord,
        create_voter_weight_record,
        get_voter_weight_record_data_checked,
    },
    max_voter_weight::{
        MaxVoterWeightRecord,
        create_max_voter_weight_record,
        get_max_voter_weight_record_data_checked,
    },
};

pub struct Processor {}

impl Processor {

    pub fn process_deposit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        seeds: [u8; 32],
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
        let rent_sysvar_info = next_account_info(accounts_iter)?;

        let realm_info = if let Some(governance) = accounts_iter.next() {
            let realm = next_account_info(accounts_iter)?;
            let voter_weight = next_account_info(accounts_iter)?;
            let max_voter_weight = next_account_info(accounts_iter)?;
            Some((governance, realm, voter_weight, max_voter_weight,))
        } else {
            None
        };

        let vesting_account_key = Pubkey::create_program_address(&[&seeds], program_id)?;
        if vesting_account_key != *vesting_account.key {
            msg!("Provided vesting account is invalid");
            return Err(ProgramError::InvalidArgument);
        }

        if !source_token_account_owner.is_signer {
            msg!("Source token account owner should be a signer.");
            return Err(ProgramError::InvalidArgument);
        }

        if !vesting_account.data_is_empty() {
            msg!("Vesting account already exists");
            return Err(ProgramError::InvalidArgument);
        }

        let vesting_token_account_data = Account::unpack(&vesting_token_account.data.borrow())?;

        if vesting_token_account_data.owner != vesting_account_key {
            msg!("The vesting token account should be owned by the vesting account.");
            return Err(ProgramError::InvalidArgument);
        }

        if vesting_token_account_data.delegate.is_some() {
            msg!("The vesting token account should not have a delegate authority");
            return Err(ProgramError::InvalidAccountData);
        }

        if vesting_token_account_data.close_authority.is_some() {
            msg!("The vesting token account should not have a close authority");
            return Err(ProgramError::InvalidAccountData);
        }

        let mut total_amount: u64 = 0;
        for s in schedules.iter() {
            total_amount = total_amount.checked_add(s.amount).ok_or_else(|| ProgramError::InvalidInstructionData)?;
        }
        
        let rent = &Rent::from_account_info(rent_sysvar_info)?;
        let vesting_record = VestingRecord {
            account_type: VestingAccountType::VestingRecord,
            owner: *vesting_owner_account.key,
            mint: vesting_token_account_data.mint,
            realm: realm_info.map(|v| *v.1.key),
            schedule: schedules
        };
        create_and_serialize_account_signed::<VestingRecord>(
            payer_account,
            vesting_account,
            &vesting_record,
            &[&seeds[..31]],
            program_id,
            system_program_account,
            rent
        )?;

        if Account::unpack(&source_token_account.data.borrow())?.amount < total_amount {
            msg!("The source token account has insufficient funds.");
            return Err(ProgramError::InsufficientFunds)
        };

        let transfer_tokens_to_vesting_account = transfer(
            spl_token_account.key,
            source_token_account.key,
            vesting_token_account.key,
            source_token_account_owner.key,
            &[],
            total_amount,
        )?;

        invoke(
            &transfer_tokens_to_vesting_account,
            &[
                source_token_account.clone(),
                vesting_token_account.clone(),
                spl_token_account.clone(),
                source_token_account_owner.clone(),
            ],
        )?;

        if let Some((_governance, realm_account, voter_weight_record_account, max_voter_weight_record_account)) = realm_info {
            if voter_weight_record_account.data_is_empty() {
                create_voter_weight_record(
                    program_id,
                    realm_account.key,
                    &vesting_token_account_data.mint,
                    vesting_owner_account.key,
                    payer_account,
                    voter_weight_record_account,
                    system_program_account,
                    |record| {Ok(record.voter_weight = total_amount)},
                )?;
            } else {
                let mut voter_weight_record = get_voter_weight_record_data_checked(
                        &program_id,
                        voter_weight_record_account,
                        realm_account.key,
                        &vesting_token_account_data.mint,
                        vesting_owner_account.key)?;

                let voter_weight = &mut voter_weight_record.voter_weight;
                *voter_weight = voter_weight.checked_add(total_amount).ok_or_else(|| ProgramError::InvalidInstructionData)?;
                voter_weight_record.serialize(&mut *voter_weight_record_account.data.borrow_mut())?;
            }

            if max_voter_weight_record_account.data_is_empty() {
                create_max_voter_weight_record(
                    program_id,
                    realm_account.key,
                    &vesting_token_account_data.mint,
                    payer_account,
                    max_voter_weight_record_account,
                    system_program_account,
                    |record| Ok(record.max_voter_weight = total_amount)
                )?;
            } else {
                let mut max_voter_weight_record = get_max_voter_weight_record_data_checked(
                        &program_id,
                        max_voter_weight_record_account,
                        realm_account.key,
                        &vesting_token_account_data.mint)?;

                let max_voter_weight = &mut max_voter_weight_record.max_voter_weight;
                *max_voter_weight = max_voter_weight.checked_add(total_amount).ok_or_else(|| ProgramError::InvalidInstructionData)?;
                max_voter_weight_record.serialize(&mut *max_voter_weight_record_account.data.borrow_mut())?;
            }
        }

        Ok(())
    }

    pub fn process_withdraw(
        program_id: &Pubkey,
        _accounts: &[AccountInfo],
        seeds: [u8; 32],
    ) -> ProgramResult {
        let accounts_iter = &mut _accounts.iter();

        let spl_token_account = next_account_info(accounts_iter)?;
        let clock_sysvar_account = next_account_info(accounts_iter)?;
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

        let vesting_account_key = Pubkey::create_program_address(&[&seeds], program_id)?;
        if vesting_account_key != *vesting_account.key {
            msg!("Invalid vesting account key");
            return Err(ProgramError::InvalidArgument);
        }

        if spl_token_account.key != &spl_token::id() {
            msg!("The provided spl token program account is invalid");
            return Err(ProgramError::InvalidArgument)
        }

        let mut vesting_record = get_account_data::<VestingRecord>(&program_id, vesting_account)?;

        if !vesting_owner_account.is_signer {
            msg!("Vesting owner should be a signer");
            return Err(ProgramError::InvalidArgument);
        }

        if vesting_record.owner != *vesting_owner_account.key {
            msg!("Vesting owner does not matched provided account");
            return Err(ProgramError::InvalidArgument);
        }

        let vesting_token_account_data = Account::unpack(&vesting_token_account.data.borrow())?;
        if vesting_token_account_data.owner != vesting_account_key {
            msg!("The vesting token account should be owned by the vesting account.");
            return Err(ProgramError::InvalidArgument);
        }

        // Unlock the schedules that have reached maturity
        let clock = Clock::get()?;
        let mut total_amount_to_transfer = 0;
        for s in vesting_record.schedule.iter_mut() {
            if clock.unix_timestamp as u64 >= s.release_time {
                total_amount_to_transfer += s.amount;
                s.amount = 0;
            }
        }
        if total_amount_to_transfer == 0 {
            msg!("Vesting contract has not yet reached release time");
            return Err(ProgramError::InvalidArgument);
        }

        let transfer_tokens_from_vesting_account = transfer(
            &spl_token_account.key,
            &vesting_token_account.key,
            destination_token_account.key,
            &vesting_account_key,
            &[],
            total_amount_to_transfer,
        )?;

        invoke_signed(
            &transfer_tokens_from_vesting_account,
            &[
                spl_token_account.clone(),
                vesting_token_account.clone(),
                destination_token_account.clone(),
                vesting_account.clone(),
            ],
            &[&[&seeds]],
        )?;

        // Reset released amounts to 0. This makes the simple unlock safe with complex scheduling contracts
        vesting_record.serialize(&mut *vesting_account.data.borrow_mut())?;

        if let Some(expected_realm_account) = vesting_record.realm {
            let (governance_account,
                 realm_account,
                 owner_record_account,
                 voter_weight_record_account,
                 max_voter_weight_record_account) = realm_info.ok_or_else(|| {VestingError::MissingRealmAccounts})?;

            if *realm_account.key != expected_realm_account {
                return Err(VestingError::InvalidRealmAccount.into())
            };

            let realm_data = get_realm_data(governance_account.key, realm_account)?;
            realm_data.assert_is_valid_governing_token_mint(&vesting_record.mint)?;

            let owner_record_data = get_token_owner_record_data_for_seeds(
                governance_account.key,
                owner_record_account,
                &get_token_owner_record_address_seeds(
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key,
                ),
            )?;
            owner_record_data.assert_can_withdraw_governing_tokens()?;

            let mut voter_weight_record = get_voter_weight_record_data_checked(
                    &program_id,
                    voter_weight_record_account,
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key)?;

            let voter_weight = &mut voter_weight_record.voter_weight;
            *voter_weight = voter_weight.checked_sub(total_amount_to_transfer).ok_or_else(|| ProgramError::InvalidInstructionData)?;
            voter_weight_record.serialize(&mut *voter_weight_record_account.data.borrow_mut())?;

            let mut max_voter_weight_record = get_max_voter_weight_record_data_checked(
                    &program_id,
                    max_voter_weight_record_account,
                    realm_account.key,
                    &vesting_record.mint)?;

            let max_voter_weight = &mut max_voter_weight_record.max_voter_weight;
            *max_voter_weight = max_voter_weight.checked_sub(total_amount_to_transfer).ok_or_else(|| ProgramError::InvalidInstructionData)?;
            max_voter_weight_record.serialize(&mut *max_voter_weight_record_account.data.borrow_mut())?;
        }

        Ok(())
    }

    pub fn process_change_owner(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        seeds: [u8; 32],
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

        let vesting_account_key = Pubkey::create_program_address(&[&seeds], program_id)?;
        if vesting_account_key != *vesting_account.key {
            msg!("Invalid vesting account key");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vesting_record = get_account_data::<VestingRecord>(&program_id, vesting_account)?;

        if vesting_record.owner != *vesting_owner_account.key {
            msg!("Vesting owner account does not matched provided account");
            return Err(ProgramError::InvalidArgument);
        }

        if !vesting_owner_account.is_signer {
            msg!("Vesting owner account should be a signer.");
            return Err(ProgramError::InvalidArgument);
        }

        let mut total_amount = 0;
        for s in vesting_record.schedule.iter_mut() {
            total_amount += s.amount;
        }

        vesting_record.owner = *new_vesting_owner_account.key;
        vesting_record.serialize(&mut *vesting_account.data.borrow_mut())?;

        if let Some(expected_realm_account) = vesting_record.realm {
            let (governance_account,
                 realm_account,
                 owner_record_account,
                 voter_weight_record_account,
                 new_voter_weight_record_account) = realm_info.ok_or_else(|| {VestingError::MissingRealmAccounts})?;

            if *realm_account.key != expected_realm_account {
                return Err(VestingError::InvalidRealmAccount.into())
            };

            let realm_data = get_realm_data(governance_account.key, realm_account)?;
            realm_data.assert_is_valid_governing_token_mint(&vesting_record.mint)?;

            let owner_record_data = get_token_owner_record_data_for_seeds(
                governance_account.key,
                owner_record_account,
                &get_token_owner_record_address_seeds(
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key,
                ),
            )?;
            owner_record_data.assert_can_withdraw_governing_tokens()?;

            let mut voter_weight_record = get_voter_weight_record_data_checked(
                    &program_id,
                    voter_weight_record_account,
                    realm_account.key,
                    &vesting_record.mint,
                    vesting_owner_account.key)?;
            let voter_weight = &mut voter_weight_record.voter_weight;
            *voter_weight = voter_weight.checked_sub(total_amount).ok_or_else(|| ProgramError::InvalidInstructionData)?;
            voter_weight_record.serialize(&mut *voter_weight_record_account.data.borrow_mut())?;

            // TODO create new_voter_weight_record if missing
            let mut new_voter_weight_record = get_voter_weight_record_data_checked(
                    &program_id,
                    new_voter_weight_record_account,
                    realm_account.key,
                    &vesting_record.mint,
                    new_vesting_owner_account.key)?;

            let new_voter_weight = &mut new_voter_weight_record.voter_weight;
            *new_voter_weight = new_voter_weight.checked_add(total_amount).ok_or_else(|| ProgramError::InvalidInstructionData)?;
            new_voter_weight_record.serialize(&mut *new_voter_weight_record_account.data.borrow_mut())?;

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
            VestingInstruction::Deposit {seeds, schedules} => {
                Self::process_deposit(program_id, accounts, seeds, schedules)
            }
            VestingInstruction::Withdraw {seeds} => {
                Self::process_withdraw(program_id, accounts, seeds)
            }
            VestingInstruction::ChangeOwner {seeds} => {
                Self::process_change_owner(program_id, accounts, seeds)
            }
        }
    }
}
