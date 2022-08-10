//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    bpf_loader_upgradeable,
    instruction::{AccountMeta, Instruction},
    hash::Hash,
    pubkey::Pubkey,
    system_program,
    sysvar,
};

use crate::{
    processor::get_maintenance_record_seeds,
};

/// Instructions supported by the Maintenance program
/// This program is a mock program used by spl-governance for testing and not real addin
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum MaintenanceInstruction {
    /// Creates MaintenanceRecord owned by the program
    ///
    /// 0. `[]` Bpf Loader Upgradeable Program Id
    /// 1. `[]` Maintained program account
    /// 2. `[writable]` Maintained program data account
    /// 3. `[writable]` MaintenanceRecord
    /// 4. `[signer]` Authority (current program upgrade-authority)
    /// 5. `[signer]` Payer
    /// 6. `[]` System
    /// 7. `[]` Maintenance record authority
    CreateMaintenance { },

    /// Sets Delegates into MaintenanceRecord
    ///
    /// 0. `[writable]` MaintenanceRecord
    /// 1. `[signer]` Authority
    SetDelegates {
        #[allow(dead_code)]
        delegate: Vec<Pubkey>,
    },

    /// Sets Code Hashes into MaintenanceRecord
    ///
    /// 0. `[writable]` MaintenanceRecord
    /// 1. `[signer]` Authority
    SetCodeHashes {
        #[allow(dead_code)]
        hashes: Vec<Hash>,
    },

    /// Upgrades the Maintained program from buffer
    ///
    /// 0. `[]` Bpf Loader Upgradeable Program Id
    /// 1. `[]` Sysvar Rent Program Id
    /// 2. `[]` Sysvar Clock Program Id
    /// 3. `[writable]` Maintained program account
    /// 4. `[writable]` Maintained program data account
    /// 5. `[writable]` Upgrade buffer account
    /// 6. `[]` MaintenanceRecord
    /// 7. `[signer]` Authority
    /// 8. `[writable]` Spill account
    Upgrade { },

    /// Revokes Authority from the program
    ///
    /// 0. `[]` Bpf Loader Upgradeable Program Id
    /// 1. `[]` Maintained program account
    /// 2. `[writable]` Maintained program data account
    /// 3. `[]` MaintenanceRecord
    /// 4. `[signer]` Authority
    /// 5. `[]` New Authority
    SetProgramAuthority { },

    /// Closes MaintenanceRecord owned by the program
    ///
    /// 0. `[]` Bpf Loader Upgradeable Program Id
    /// 1. `[writable]` MaintenanceRecord
    /// 2. `[]` Maintained program account
    /// 3. `[]` Maintained program data account
    /// 4. `[signer]` Authority
    /// 5. `[writable]` Spill destination
    CloseMaintenance { },

    /// Change MaintenanceRecord Authority
    ///
    /// 0. `[writable]` MaintenanceRecord
    /// 1. `[signer]` Current authority
    /// 2. `[]` New authority
    SetAuthority { },
}


/// Get MaintenanceRecord account address and bump seed
pub fn get_maintenance_record_address(program_id: &Pubkey, maintenance: &Pubkey) -> (Pubkey, u8) {
    let seeds: &[&[u8]] = &get_maintenance_record_seeds(maintenance);
    Pubkey::find_program_address(seeds, program_id)
}

/// Creates 'Create Maintenance' instruction
pub fn create_maintenance(
    program_id: &Pubkey,
    // Accounts
    program_address: &Pubkey,
    program_authority: &Pubkey,
    new_authority: &Pubkey,
    payer: &Pubkey,
) -> Instruction {

    let (programdata_address, _) = Pubkey::find_program_address(&[program_address.as_ref()], &bpf_loader_upgradeable::id());
    let (maintenance_record, _): (Pubkey, u8) = get_maintenance_record_address(program_id, program_address);

    let accounts = vec![
        AccountMeta::new_readonly(bpf_loader_upgradeable::id(), false),
        AccountMeta::new_readonly(*program_address, false),
        AccountMeta::new(programdata_address, false),
        AccountMeta::new(maintenance_record, false),
        AccountMeta::new_readonly(*program_authority, true),
        AccountMeta::new_readonly(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*new_authority, false),
    ];

    let instruction = MaintenanceInstruction::CreateMaintenance { };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates 'Set Delegate' instruction
pub fn set_delegate(
    program_id: &Pubkey,
    // Accounts
    address: &Pubkey,
    delegate: Vec<Pubkey>,
    authority: &Pubkey,
) -> Instruction {

    let (maintenance_record, _): (Pubkey, u8) = get_maintenance_record_address(program_id, address);

    let accounts = vec![
        AccountMeta::new(maintenance_record, false),
        AccountMeta::new_readonly(*authority, true),
    ];

    let instruction = MaintenanceInstruction::SetDelegates { delegate };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates 'Set Code Hashes' instruction
pub fn set_code_hashes(
    program_id: &Pubkey,
    // Accounts
    address: &Pubkey,
    hashes: Vec<Hash>,
    authority: &Pubkey,
) -> Instruction {

    let (maintenance_record, _): (Pubkey, u8) = get_maintenance_record_address(program_id, address);

    let accounts = vec![
        AccountMeta::new(maintenance_record, false),
        AccountMeta::new_readonly(*authority, true),
    ];

    let instruction = MaintenanceInstruction::SetCodeHashes { hashes };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates 'Set Authority' instruction
pub fn upgrade(
    program_id: &Pubkey,
    // Accounts
    program_address: &Pubkey,
    authority: &Pubkey,
    buffer: &Pubkey,
    spill: &Pubkey,
) -> Instruction {

    let (programdata_address, _) = Pubkey::find_program_address(&[program_address.as_ref()], &bpf_loader_upgradeable::id());
    let (maintenance_record, _): (Pubkey, u8) = get_maintenance_record_address(program_id, program_address);

    let accounts = vec![
        AccountMeta::new_readonly(bpf_loader_upgradeable::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new(*program_address, false),
        AccountMeta::new(programdata_address, false),
        AccountMeta::new(*buffer, false),
        AccountMeta::new_readonly(maintenance_record, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new(*spill, false),
    ];

    let instruction = MaintenanceInstruction::Upgrade { };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates 'Set Program Authority' instruction
pub fn set_program_authority(
    program_id: &Pubkey,
    // Accounts
    program_address: &Pubkey,
    authority: &Pubkey,
    new_authority: &Pubkey,
) -> Instruction {

    let (programdata_address, _) = Pubkey::find_program_address(&[program_address.as_ref()], &bpf_loader_upgradeable::id());
    let (maintenance_record, _): (Pubkey, u8) = get_maintenance_record_address(program_id, program_address);

    let accounts = vec![
        AccountMeta::new_readonly(bpf_loader_upgradeable::id(), false),
        AccountMeta::new_readonly(*program_address, false),
        AccountMeta::new(programdata_address, false),
        AccountMeta::new_readonly(maintenance_record, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(*new_authority, false),
    ];

    let instruction = MaintenanceInstruction::SetProgramAuthority { };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates 'Close Maintenance' instruction
pub fn close_maintenance(
    program_id: &Pubkey,
    // Accounts
    program_address: &Pubkey,
    authority: &Pubkey,
    spill: &Pubkey,
) -> Instruction {

    let (programdata_address, _) = Pubkey::find_program_address(&[program_address.as_ref()], &bpf_loader_upgradeable::id());
    let (maintenance_record, _): (Pubkey, u8) = get_maintenance_record_address(program_id, program_address);

    let accounts = vec![
        AccountMeta::new_readonly(bpf_loader_upgradeable::id(), false),
        AccountMeta::new(maintenance_record, false),
        AccountMeta::new_readonly(*program_address, false),
        AccountMeta::new_readonly(programdata_address, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new(*spill, false),
    ];

    let instruction = MaintenanceInstruction::CloseMaintenance { };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates 'Set Authority' instruction
pub fn set_authority(
    program_id: &Pubkey,
    // Accounts
    program_address: &Pubkey,
    authority: &Pubkey,
    new_authority: &Pubkey,
) -> Instruction {

    let (maintenance_record, _): (Pubkey, u8) = get_maintenance_record_address(program_id, program_address);

    let accounts = vec![
        AccountMeta::new(maintenance_record, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(*new_authority, false),
    ];

    let instruction = MaintenanceInstruction::SetAuthority { };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_instruction_packing() {
        let original_create_maintenance = MaintenanceInstruction::CreateMaintenance {
        };
        assert_eq!(
            original_create_maintenance,
            MaintenanceInstruction::try_from_slice(&original_create_maintenance.try_to_vec().unwrap()).unwrap()
        );

        let original_set_authority = MaintenanceInstruction::SetAuthority { };
        assert_eq!(
            original_set_authority,
            MaintenanceInstruction::try_from_slice(&original_set_authority.try_to_vec().unwrap()).unwrap()
        );

        let delegate1: Pubkey = Pubkey::new_unique();
        let delegate2: Pubkey = Pubkey::new_unique();
        let delegates: Vec<Pubkey> = vec![delegate1, delegate2];
        let original_set_delegates = MaintenanceInstruction::SetDelegates { delegate: delegates };
        assert_eq!(
            original_set_delegates,
            MaintenanceInstruction::try_from_slice(&original_set_delegates.try_to_vec().unwrap()).unwrap()
        );

        let hashes: Vec<Hash> = vec![];
        let original_set_code_hashes = MaintenanceInstruction::SetCodeHashes { hashes: hashes };
        assert_eq!(
            original_set_code_hashes,
            MaintenanceInstruction::try_from_slice(&original_set_code_hashes.try_to_vec().unwrap()).unwrap()
        );

        let original_close_maintenance = MaintenanceInstruction::CloseMaintenance { };
        assert_eq!(
            original_close_maintenance,
            MaintenanceInstruction::try_from_slice(&original_close_maintenance.try_to_vec().unwrap()).unwrap()
        );

    }
}
