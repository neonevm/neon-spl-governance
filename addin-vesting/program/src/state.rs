use solana_program::{
    program_pack::IsInitialized,
    pubkey::Pubkey,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use spl_governance_tools::account::AccountMaxSize;

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VestingAccountType {
    /// Default uninitialized state
    Unitialized,

    /// Vesting info account
    VestingRecord,
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VestingSchedule {
    pub release_time: u64,
    pub amount: u64,
}

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VestingRecord {
    pub account_type: VestingAccountType,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub token: Pubkey,
    pub realm: Option<Pubkey>,
    pub schedule: Vec<VestingSchedule>,
}

impl IsInitialized for VestingRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == VestingAccountType::VestingRecord
    }
}

impl AccountMaxSize for VestingRecord {}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;
    use solana_program::account_info::AccountInfo;
    use spl_governance_tools::account::get_account_data;
    use solana_program::clock::Epoch;

    #[test]
    fn test_vesting_record_packing() {
        let vesting_record_source = VestingRecord {
            account_type: VestingAccountType::VestingRecord,
            owner: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            token: Pubkey::new_unique(),
            realm: Some(Pubkey::new_unique()),
            schedule: vec!(
                VestingSchedule {release_time: 30767976, amount: 969},
                VestingSchedule {release_time: 32767076, amount: 420},
            ),
        };

        let mut vesting_data = vesting_record_source.try_to_vec().unwrap();
        println!("UNPACKED: {:?}", vesting_record_source);
        println!("PACKED: {}", hex::encode(&vesting_data));

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let account_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut vesting_data[..],
            &program_id,
            false,
            Epoch::default(),
        );
        let vesting_record_target = get_account_data::<VestingRecord>(&program_id, &account_info).unwrap();
        assert_eq!(vesting_record_source, vesting_record_target);
    }
}
