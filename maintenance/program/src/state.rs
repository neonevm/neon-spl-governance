use solana_program::{
    hash::Hash,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use spl_governance_tools::account::AccountMaxSize;

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum MaintenanceAccountType {
    /// Default uninitialized state
    Unitialized,

    /// Maintenance info account
    MaintenanceRecord,
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct MaintenanceRecord {
    pub account_type: MaintenanceAccountType,
    pub authority: Pubkey,
    pub delegate: Vec<Pubkey>,
    pub hashes: Vec<Hash>,
}

impl IsInitialized for MaintenanceRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == MaintenanceAccountType::MaintenanceRecord
    }
}

impl AccountMaxSize for MaintenanceRecord {
    fn get_max_size(&self) -> Option<usize> {
        Some(649)   // for delegate size = 10, hashes size = 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;
    use solana_program::account_info::AccountInfo;
    use spl_governance_tools::account::get_account_data;
    use solana_program::clock::Epoch;

    #[test]
    fn test_maintenance_record_packing() {
        let maintenance_record_source = MaintenanceRecord {
            account_type: MaintenanceAccountType::MaintenanceRecord,
            authority: Pubkey::new_unique(),
            delegate: Vec::new(),
            hashes: Vec::new(),
        };

        let mut maintenance_data = maintenance_record_source.try_to_vec().unwrap();
        println!("UNPACKED: {:?}", maintenance_record_source);
        println!("PACKED: {}", hex::encode(&maintenance_data));

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let account_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut maintenance_data[..],
            &program_id,
            false,
            Epoch::default(),
        );
        let vesting_record_target = get_account_data::<MaintenanceRecord>(&program_id, &account_info).unwrap();
        assert_eq!(maintenance_record_source, vesting_record_target);
    }
}
