use crate::state::VestingSchedule;

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
    sysvar,
};

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

#[cfg(feature = "fuzz")]
use arbitrary::Arbitrary;

#[cfg(feature = "fuzz")]
impl Arbitrary for VestingInstruction {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let seeds: [u8; 32] = u.arbitrary()?;
        let choice = u.choose(&[0, 1, 2, 3])?;
        match choice {
            0 => {
                let number_of_schedules = u.arbitrary()?;
                return Ok(Self::Init {
                    seeds,
                    number_of_schedules,
                });
            }
            1 => {
                let schedules: [Schedule; 10] = u.arbitrary()?;
                let key_bytes: [u8; 32] = u.arbitrary()?;
                let mint_address: Pubkey = Pubkey::new(&key_bytes);
                let key_bytes: [u8; 32] = u.arbitrary()?;
                let destination_token_address: Pubkey = Pubkey::new(&key_bytes);
                return Ok(Self::Create {
                    seeds,
                    mint_address,
                    destination_token_address,
                    schedules: schedules.to_vec(),
                });
            }
            2 => return Ok(Self::Unlock { seeds }),
            _ => return Ok(Self::ChangeDestination { seeds }),
        }
    }
}

#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Schedule {
    // Schedule release time in unix timestamp
    pub release_time: u64,
    pub amount: u64,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VestingInstruction {

    /// Creates a new vesting schedule contract
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[]` The system program account
    ///   1. `[]` The spl-token program account
    ///   2. `[writable]` The vesting account                (vesting owner account: PDA seeds: [seeds])
    ///   3. `[writable]` The vesting spl-token account      (vesting balance account)
    ///   4. `[signer]` The source spl-token account owner   (from account owner)
    ///   5. `[writable]` The source spl-token account       (from account)
    ///   6. `[]` The Vesting Owner account
    ///   7. `[signer]` Payer
    ///   8. `[]` Rent sysvar
    ///
    ///  Optional part (vesting for Realm)
    ///   8. `[]` The Governance program account
    ///   9. `[]` The Realm account
    ///  10. `[writable]` The VoterWeightRecord. PDA seeds: ['voter_weight', realm, token_mint, token_owner]
    ///  11. `[writable]` The MaxVoterWeightRecord. PDA seeds: ['max_voter_weight', realm, token_mint]
    ///
    Deposit {
        #[allow(dead_code)]
        seeds: [u8; 32],
        #[allow(dead_code)]
        schedules: Vec<VestingSchedule>,
    },



    /// Unlocks a simple vesting contract (SVC) - can only be invoked by the program itself
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[]` The spl-token program account
    ///   1. `[]` The clock sysvar account
    ///   2. `[writable]` The vesting account               (vesting owner account: PDA [seeds])
    ///   3. `[writable]` The vesting spl-token account     (vesting balance account)
    ///   4. `[writable]` The destination spl-token account
    ///   5. `[signer]` The Vesting Owner account
    ///
    ///  Optional part (vesting for Realm)
    ///   6. `[]` The Governance program account
    ///   7. `[]` The Realm account
    ///   8. `[]` Governing Owner Record. PDA seeds (governance program): ['governance', realm, token_mint, vesting_owner]
    ///   9. `[writable]` The VoterWeightRecord. PDA seeds: ['voter_weight', realm, token_mint, vesting_owner]
    ///  10. `[writable]` The MaxVoterWeightRecord. PDA seeds: ['max_voter_weight', realm, token_mint]
    ///
    Withdraw {
        #[allow(dead_code)]
        seeds: [u8; 32],
    },



    /// Change the destination account of a given simple vesting contract (SVC)
    /// - can only be invoked by the present destination address of the contract.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The Vesting account      (PDA seeds: [seeds] / [realm, seeds])
    ///   1. `[signer]` The Current Vesting Owner account
    ///   2. `[]` The New Vesting Owner account
    ///
    ///  Optional part (vesting for Realm)
    ///   3. `[]` The Governance program account
    ///   4. `[]` The Realm account
    ///   5. `[]` Governing Owner Record. PDA seeds (governance program): ['governance', realm, token_mint, current_vesting_owner]
    ///   6. `[writable]` The from VoterWeight Record. PDA seeds: ['voter_weight', realm, token_mint, current_vesting_owner]
    ///   7. `[writable]` The to VoterWeight Record. PDA seeds: ['voter_weight', realm, token_mint, new_vesting_owner]
    ChangeOwner {
        #[allow(dead_code)]
        seeds: [u8; 32],
    },
}

/// Creates a `Deposit` instruction to create and initialize the vesting token account
pub fn deposit(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    seeds: [u8; 32],
    vesting_token_account: &Pubkey,
    source_token_owner: &Pubkey,
    source_token_account: &Pubkey,
    vesting_owner: &Pubkey,
    payer: &Pubkey,
    schedules: Vec<VestingSchedule>,
) -> Result<Instruction, ProgramError> {
    let vesting_account = Pubkey::create_program_address(&[&seeds], program_id)?;
    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new(vesting_account, false),
        AccountMeta::new(*vesting_token_account, false),
        AccountMeta::new_readonly(*source_token_owner, true),
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*vesting_owner, false),
        AccountMeta::new_readonly(*payer, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let instruction = VestingInstruction::Deposit { seeds, schedules };

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

/// Creates a `Withdraw` instruction
pub fn withdraw(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    seeds: [u8; 32],
    vesting_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    vesting_owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let vesting_account = Pubkey::create_program_address(&[&seeds], program_id)?;
    let accounts = vec![
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new(vesting_account, false),
        AccountMeta::new(*vesting_token_account, false),
        AccountMeta::new(*destination_token_account, false),
        AccountMeta::new_readonly(*vesting_owner, true),
    ];

    let instruction = VestingInstruction::Withdraw { seeds };

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

/// Creates a `Withdraw` instruction
pub fn change_owner(
    program_id: &Pubkey,
    seeds: [u8; 32],
    vesting_owner: &Pubkey,
    new_vesting_owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let vesting_account = Pubkey::create_program_address(&[&seeds], program_id)?;
    let accounts = vec![
        AccountMeta::new(vesting_account, false),
        AccountMeta::new(*vesting_owner, true),
        AccountMeta::new(*new_vesting_owner, false),
    ];

    let instruction = VestingInstruction::ChangeOwner { seeds };

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_instruction_packing() {
        let original_deposit = VestingInstruction::Deposit {
            seeds: [50u8; 32],
            schedules: vec![VestingSchedule {
                amount: 42,
                release_time: 250,
            }],
        };
        assert_eq!(
            original_deposit,
            VestingInstruction::try_from_slice(&original_deposit.try_to_vec().unwrap()).unwrap()
        );


        let original_withdraw = VestingInstruction::Withdraw { seeds: [50u8; 32] };
        assert_eq!(
            original_withdraw,
            VestingInstruction::try_from_slice(&original_withdraw.try_to_vec().unwrap()).unwrap()
        );


        let original_change = VestingInstruction::ChangeOwner { seeds: [50u8; 32] };
        assert_eq!(
            original_change,
            VestingInstruction::try_from_slice(&original_change.try_to_vec().unwrap()).unwrap()
        );
    }
}
