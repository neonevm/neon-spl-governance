//! Program processor

use borsh::{ BorshDeserialize, BorshSerialize };

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    bpf_loader_upgradeable::{
        self,
        set_upgrade_authority,
        upgrade,
        UpgradeableLoaderState,
    },
    hash::{ hash, Hash, },
    entrypoint::ProgramResult,
    msg,
    program::{ invoke, invoke_signed },
    program_error::ProgramError,
    pubkey::{ Pubkey },
    rent::Rent,
    sysvar::Sysvar,
};

use spl_governance_tools::account::{
    get_account_data,
    create_and_serialize_account_signed,
    dispose_account,
};

use crate::{
    error::MaintenanceError,
    instruction::MaintenanceInstruction,
    state::{ MaintenanceAccountType, MaintenanceRecord, },
};

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = MaintenanceInstruction::try_from_slice(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("MAINTENANCE-INSTRUCTION: {:?}", instruction);

    match instruction {
        MaintenanceInstruction::CreateMaintenance { } => process_create_maintenance(
            program_id,
            accounts,
        ),
        MaintenanceInstruction::SetDelegates { delegate } => process_set_delegates(
            program_id,
            accounts,
            delegate,
        ),
        MaintenanceInstruction::SetCodeHashes { hashes } => process_set_code_hashes(
            program_id,
            accounts,
            hashes,
        ),
        MaintenanceInstruction::Upgrade { } => process_upgrade(
            program_id,
            accounts,
        ),
        MaintenanceInstruction::SetProgramAuthority { } => process_set_program_authority(
            program_id,
            accounts,
        ),
        MaintenanceInstruction::CloseMaintenance { } => process_close_maintenance(
            program_id,
            accounts,
        ),
        MaintenanceInstruction::SetAuthority { } => process_set_authority(
            program_id,
            accounts,
        ),
    }
}

/// Get MaintenanceRecord account seeds
pub fn get_maintenance_record_seeds(maintenance: &Pubkey) -> [& [u8]; 2] {
    [b"maintenance", maintenance.as_ref()]
}

/// Processes CreateMaintenance instruction
pub fn process_create_maintenance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let bpf_loader_program_info = next_account_info(account_info_iter)?;       // 0
    let maintained_program_info = next_account_info(account_info_iter)?;       // 1
    let maintained_program_data_info = next_account_info(account_info_iter)?;  // 2
    let maintenance_record_info = next_account_info(account_info_iter)?;       // 3
    let authority_info = next_account_info(account_info_iter)?;                // 4
    let payer_info = next_account_info(account_info_iter)?;                    // 5
    let system_info = next_account_info(account_info_iter)?;                   // 6
    let new_authority_info = next_account_info(account_info_iter)?;            // 7

    if !bpf_loader_upgradeable::check_id(bpf_loader_program_info.key) {
        return Err(MaintenanceError::IncorrectBpfLoaderProgramId.into());
    }

    let (maintained_program_data_address, _) = Pubkey::find_program_address(&[maintained_program_info.key.as_ref()], bpf_loader_program_info.key);
    if *maintained_program_data_info.key != maintained_program_data_address {
        return Err(MaintenanceError::WrongProgramDataForMaintenanceRecord.into());
    }

    let upgradeable_loader_state: UpgradeableLoaderState =
        bincode::deserialize(&maintained_program_data_info.data.borrow())
        .map_err(|_| ProgramError::from(MaintenanceError::AuthorityDeserializationError) )?;
    
    let program_authority: Pubkey = 
        match upgradeable_loader_state {
            UpgradeableLoaderState::ProgramData { slot: _, upgrade_authority_address } => 
                upgrade_authority_address.ok_or_else(|| ProgramError::from(MaintenanceError::AuthorityDeserializationError) )?,
            _ => 
                return Err(ProgramError::from(MaintenanceError::AuthorityDeserializationError)),
        };

    if *authority_info.key != program_authority {
        return Err(MaintenanceError::WrongAuthority.into());
    }

    let maintenance_record_data = MaintenanceRecord {
        account_type: MaintenanceAccountType::MaintenanceRecord,
        maintained_address: *maintained_program_info.key,
        authority: *new_authority_info.key,
        delegate: Vec::new(),
        hashes: Vec::new(),
    };

    invoke(
        &set_upgrade_authority(
            maintained_program_info.key,
            authority_info.key,
            Some(maintenance_record_info.key),
        ),
        &[
            bpf_loader_program_info.clone(),
            maintained_program_info.clone(),
            maintained_program_data_info.clone(),
            maintenance_record_info.clone(),
            authority_info.clone(),
        ],
    )?;

    let seeds: &[&[u8]] = &get_maintenance_record_seeds(maintained_program_info.key);
    let rent = Rent::get()?;

    create_and_serialize_account_signed(
        payer_info,
        maintenance_record_info,
        &maintenance_record_data,
        seeds,
        program_id,
        system_info,
        &rent,
    )?;

    Ok(())
}

/// Processes Set Delegate instruction
pub fn process_set_delegates(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    delegate: Vec<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let maintenance_record_info = next_account_info(account_info_iter)?; // 0
    let authority_info = next_account_info(account_info_iter)?; // 1

    if !authority_info.is_signer {
        return Err(MaintenanceError::MissingRequiredSigner.into());
    }

    if delegate.len() > 10 {
        return Err(MaintenanceError::NumberOfDelegatesExceedsLimit.into());
    }

    let mut maintenance_record = get_account_data::<MaintenanceRecord>(program_id, maintenance_record_info)?;

    if *authority_info.key != maintenance_record.authority {
        return Err(MaintenanceError::WrongAuthority.into());
    }

    maintenance_record.delegate = delegate;
    
    maintenance_record.serialize(&mut *maintenance_record_info.data.borrow_mut())?;

    Ok(())
}

/// Processes Set Code Hashes instruction
pub fn process_set_code_hashes(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    hashes: Vec<Hash>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let maintenance_record_info = next_account_info(account_info_iter)?; // 0
    let authority_info = next_account_info(account_info_iter)?; // 1

    if !authority_info.is_signer {
        return Err(MaintenanceError::MissingRequiredSigner.into());
    }

    if hashes.len() > 10 {
        return Err(MaintenanceError::NumberOfCodeHashesExceedsLimit.into());
    }

    let mut maintenance_record = get_account_data::<MaintenanceRecord>(program_id, maintenance_record_info)?;

    if *authority_info.key != maintenance_record.authority {
        return Err(MaintenanceError::WrongAuthority.into());
    }

    maintenance_record.hashes = hashes;
    
    maintenance_record.serialize(&mut *maintenance_record_info.data.borrow_mut())?;

    Ok(())
}

/// Processes Upgrade instruction
pub fn process_upgrade(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let bpf_loader_program_info = next_account_info(account_info_iter)?; // 0
    let sysvar_rent_program_info = next_account_info(account_info_iter)?; // 1
    let sysvar_clock_program_info = next_account_info(account_info_iter)?; // 2
    let maintained_program_info = next_account_info(account_info_iter)?; // 3
    let maintained_program_data_info = next_account_info(account_info_iter)?; // 4
    let upgrade_buffer_info = next_account_info(account_info_iter)?; // 5
    let maintenance_record_info = next_account_info(account_info_iter)?; // 6
    let authority_info = next_account_info(account_info_iter)?; // 7
    let spill_info = next_account_info(account_info_iter)?; // 8

    if !authority_info.is_signer {
        return Err(MaintenanceError::MissingRequiredSigner.into());
    }

    let maintenance_record = get_account_data::<MaintenanceRecord>(program_id, maintenance_record_info)?;

    if maintenance_record.authority != *authority_info.key &&
        !maintenance_record.delegate.iter().any(|&item| item == *authority_info.key )
    {
        return Err(MaintenanceError::WrongDelegate.into());
    }

    let buffer_hash: Hash = {
        let buffer_data_offset = UpgradeableLoaderState::size_of_buffer_metadata();
        let program_buffer: &[u8] = &upgrade_buffer_info.data.borrow();
        let program_buffer_data: &[u8] = program_buffer.get(buffer_data_offset..).ok_or(MaintenanceError::BufferDataOffsetError)?;
        hash(program_buffer_data)
    };
    // msg!("MAINTENANCE-INSTRUCTION: UPGRADE Buffer Hash {:?}", buffer_hash);

    if !maintenance_record.hashes.iter().any(|&item| item == buffer_hash ) {
        return Err(MaintenanceError::WrongCodeHash.into());
    }

    let maintenance_seeds = get_maintenance_record_seeds(maintained_program_info.key);
    let (_maintenance_address, bump_seed) = Pubkey::find_program_address(&maintenance_seeds, program_id);

    let mut signers_seeds = maintenance_seeds.to_vec();
    let bump = &[bump_seed];
    signers_seeds.push(bump);

    let upgrade_instruction = upgrade(
        maintained_program_info.key,
        upgrade_buffer_info.key,
        maintenance_record_info.key,
        spill_info.key
    );

    invoke_signed(
        &upgrade_instruction,
        &[
            bpf_loader_program_info.clone(),
            sysvar_rent_program_info.clone(),
            sysvar_clock_program_info.clone(),
            maintained_program_info.clone(),
            maintained_program_data_info.clone(),
            upgrade_buffer_info.clone(),
            maintenance_record_info.clone(),
            spill_info.clone()
        ],
        &[&signers_seeds[..]],
    )?;

    Ok(())
}

/// Processes Set Authority instruction
pub fn process_set_program_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let bpf_loader_program_info = next_account_info(account_info_iter)?; // 0
    let maintained_program_info = next_account_info(account_info_iter)?; // 1
    let maintained_program_data_info = next_account_info(account_info_iter)?; // 2
    let maintenance_record_info = next_account_info(account_info_iter)?; // 3
    let authority_info = next_account_info(account_info_iter)?; // 4
    let new_authority_info = next_account_info(account_info_iter)?; // 5

    if !authority_info.is_signer {
        return Err(MaintenanceError::MissingRequiredSigner.into());
    }

    let maintenance_record = get_account_data::<MaintenanceRecord>(program_id, maintenance_record_info)?;

    if *authority_info.key != maintenance_record.authority {
        return Err(MaintenanceError::WrongAuthority.into());
    }

    let maintenance_seeds = get_maintenance_record_seeds(maintained_program_info.key);
    let (_maintenance_address, bump_seed) = Pubkey::find_program_address(&maintenance_seeds, program_id);

    let mut signers_seeds = maintenance_seeds.to_vec();
    let bump = &[bump_seed];
    signers_seeds.push(bump);

    let upgrade_authority_instruction = set_upgrade_authority(
        maintained_program_info.key,
        maintenance_record_info.key,
        Some(new_authority_info.key),
    );

    invoke_signed(
        &upgrade_authority_instruction,
        &[
            bpf_loader_program_info.clone(),
            maintained_program_info.clone(),
            maintained_program_data_info.clone(),
            maintenance_record_info.clone(),
            new_authority_info.clone(),
        ],
        &[&signers_seeds[..]],
    )?;

    Ok(())
}

/// Processes CloseMaintenance instruction
pub fn process_close_maintenance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let bpf_loader_program_info = next_account_info(account_info_iter)?; // 0
    let maintenance_record_info = next_account_info(account_info_iter)?; // 1
    let maintained_program_info = next_account_info(account_info_iter)?; // 2
    let maintained_program_data_info = next_account_info(account_info_iter)?; // 3
    let authority_info = next_account_info(account_info_iter)?; // 4
    let spill_info = next_account_info(account_info_iter)?; // 5

    if !authority_info.is_signer {
        return Err(MaintenanceError::MissingRequiredSigner.into());
    }

    if !bpf_loader_upgradeable::check_id(bpf_loader_program_info.key) {
        return Err(MaintenanceError::IncorrectBpfLoaderProgramId.into());
    }

    if maintenance_record_info.key == spill_info.key {
        return Err(MaintenanceError::MaintenanceRecordAccountMatchesSpillAccount.into());
    }

    let (maintained_program_data, _) = Pubkey::find_program_address(&[maintained_program_info.key.as_ref()], bpf_loader_program_info.key);
    let maintenance_record = get_account_data::<MaintenanceRecord>(program_id, maintenance_record_info)?;

    if *maintained_program_data_info.key != maintained_program_data ||
        *maintained_program_info.key != maintenance_record.maintained_address {
        return Err(MaintenanceError::WrongProgramDataForMaintenanceRecord.into());
    }

    if *authority_info.key != maintenance_record.authority {
        return Err(MaintenanceError::WrongAuthority.into());
    }

    let upgradeable_loader_state: UpgradeableLoaderState =
        bincode::deserialize(&maintained_program_data_info.data.borrow()).map_err(|_| ProgramError::from(MaintenanceError::AuthorityDeserializationError) )?;
    
    let program_authority: Pubkey = 
        match upgradeable_loader_state {
            UpgradeableLoaderState::ProgramData { slot: _, upgrade_authority_address } => 
                upgrade_authority_address.ok_or_else(|| ProgramError::from(MaintenanceError::AuthorityDeserializationError) )?,
            _ => 
                return Err(ProgramError::from(MaintenanceError::AuthorityDeserializationError)),
        };
    // msg!("MAINTENANCE-INSTRUCTION: CLOSE MAINTENANCE program authority {:?}", program_authority);

    if *maintenance_record_info.key == program_authority {
        return Err(MaintenanceError::RecordMustBeInactive.into());
    }

    dispose_account(maintenance_record_info, spill_info);

    Ok(())
}

/// Processes Set Authority instruction
pub fn process_set_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let maintenance_record_info = next_account_info(account_info_iter)?; // 0
    let authority_info = next_account_info(account_info_iter)?;          // 1
    let new_authority_info = next_account_info(account_info_iter)?;      // 2

    if !authority_info.is_signer {
        return Err(MaintenanceError::MissingRequiredSigner.into());
    }

    let mut maintenance_record = get_account_data::<MaintenanceRecord>(program_id, maintenance_record_info)?;

    if *authority_info.key != maintenance_record.authority {
        return Err(MaintenanceError::WrongAuthority.into());
    }

    maintenance_record.authority = *new_authority_info.key;
    
    maintenance_record.serialize(&mut *maintenance_record_info.data.borrow_mut())?;

    Ok(())
}

