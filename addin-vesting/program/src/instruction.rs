use crate::{
    state::VestingSchedule,
    voter_weight::get_voter_weight_record_address,
    max_voter_weight::get_max_voter_weight_record_address,
};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use spl_governance::state::token_owner_record::get_token_owner_record_address;

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
    ///   2. `[writable]` The vesting account. PDA seeds: [vesting spl-token account]
    ///   3. `[writable]` The vesting spl-token account
    ///   4. `[signer]` The source spl-token account owner
    ///   5. `[writable]` The source spl-token account
    ///   6. `[]` The Vesting Owner account
    ///   7. `[signer]` Payer
    ///
    ///  Optional part (vesting for Realm)
    ///   8. `[]` The Governance program account
    ///   9. `[]` The Realm account
    ///  10. `[writable]` The VoterWeightRecord. PDA seeds: ['voter_weight', realm, token_mint, token_owner]
    ///  11. `[writable]` The MaxVoterWeightRecord. PDA seeds: ['max_voter_weight', realm, token_mint]
    ///
    Deposit {
        #[allow(dead_code)]
        schedules: Vec<VestingSchedule>,
    },


    /// Unlocks a simple vesting contract (SVC) - can only be invoked by the program itself
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[]` The spl-token program account
    ///   1. `[writable]` The vesting account. PDA seeds: [vesting spl-token account]
    ///   2. `[writable]` The vesting spl-token account
    ///   3. `[writable]` The destination spl-token account
    ///   4. `[signer]` The Vesting Owner account
    ///
    ///  Optional part (vesting for Realm)
    ///   5. `[]` The Governance program account
    ///   6. `[]` The Realm account
    ///   7. `[]` Governing Owner Record. PDA seeds (governance program): ['governance', realm, token_mint, vesting_owner]
    ///   8. `[writable]` The VoterWeightRecord. PDA seeds: ['voter_weight', realm, token_mint, vesting_owner]
    ///   9. `[writable]` The MaxVoterWeightRecord. PDA seeds: ['max_voter_weight', realm, token_mint]
    ///
    Withdraw,


    /// Set Vote Percentage for calcalate voter_weight from total_amount of deposited tokens
    ///
    /// Accounts expected by this instruction:
    /// 
    ///  * Single owner
    ///   0. `[]` The Vesting Mint
    ///   1. `[]` The Vesting Owner account
    ///   2. `[signer]` The Vesting Authority account
    ///   3. `[]` The Governance program account
    ///   4. `[]` The Realm account
    ///   5. `[]` Governing Owner Record. PDA seeds (governance program): ['governance', realm, token_mint, vesting_owner]
    ///   6. `[writable]` The VoterWeight Record. PDA seeds: ['voter_weight', realm, token_mint, vesting_owner]
    SetVotePercentage {
        #[allow(dead_code)]
        vote_percentage: u16,
    },


    /// Change the destination account of a given simple vesting contract (SVC)
    /// - can only be invoked by the present destination address of the contract.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[writable]` The Vesting account. PDA seeds: [vesting spl-token account]
    ///   1. `[signer]` The Current Vesting Owner account
    ///   2. `[]` The New Vesting Owner account
    ///
    ///  Optional part (vesting for Realm)
    ///   3. `[]` The Governance program account
    ///   4. `[]` The Realm account
    ///   5. `[]` Governing Owner Record. PDA seeds (governance program): ['governance', realm, token_mint, current_vesting_owner]
    ///   6. `[writable]` The from VoterWeight Record. PDA seeds: ['voter_weight', realm, token_mint, current_vesting_owner]
    ///   7. `[writable]` The to VoterWeight Record. PDA seeds: ['voter_weight', realm, token_mint, new_vesting_owner]
    ChangeOwner,


    /// Create VoterWeightRecord for account
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner
    ///   0. `[]` The system program account
    ///   1. `[]` The Record Owner account
    ///   2. `[signer]` Payer
    ///   3. `[]` The Governance program account
    ///   4. `[]` The Realm account
    ///   5. `[]` The Mint account
    ///   6. `[writable]` The VoterWeightRecord. PDA seeds: ['voter_weight', realm, token_mint, token_owner]
    CreateVoterWeightRecord,
}

/// Creates a `Deposit` instruction to create and initialize the vesting token account
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    vesting_token_account: &Pubkey,
    source_token_owner: &Pubkey,
    source_token_account: &Pubkey,
    vesting_owner: &Pubkey,
    payer: &Pubkey,
    schedules: Vec<VestingSchedule>,
) -> Result<Instruction, ProgramError> {
    let (vesting_account, _) = Pubkey::find_program_address(&[vesting_token_account.as_ref()], program_id);
    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new(vesting_account, false),
        AccountMeta::new(*vesting_token_account, false),
        AccountMeta::new_readonly(*source_token_owner, true),
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*vesting_owner, false),
        AccountMeta::new_readonly(*payer, true),
    ];

    let instruction = VestingInstruction::Deposit { schedules };

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

/// Creates a `Deposit` instruction to create and initialize the vesting token account
/// inside the Realm
#[allow(clippy::too_many_arguments)]
pub fn deposit_with_realm(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    vesting_token_account: &Pubkey,
    source_token_owner: &Pubkey,
    source_token_account: &Pubkey,
    vesting_owner: &Pubkey,
    payer: &Pubkey,
    schedules: Vec<VestingSchedule>,
    governance_id: &Pubkey,
    realm: &Pubkey,
    mint: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (vesting_account, _) = Pubkey::find_program_address(&[vesting_token_account.as_ref()], program_id);
    let voting_weight_record_account = get_voter_weight_record_address(program_id, realm, mint, vesting_owner);
    let max_voting_weight_record_account = get_max_voter_weight_record_address(program_id, realm, mint);
    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new(vesting_account, false),
        AccountMeta::new(*vesting_token_account, false),
        AccountMeta::new_readonly(*source_token_owner, true),
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*vesting_owner, false),
        AccountMeta::new(*payer, true),

        AccountMeta::new_readonly(*governance_id, false),
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(voting_weight_record_account, false),
        AccountMeta::new(max_voting_weight_record_account, false),
    ];

    let instruction = VestingInstruction::Deposit { schedules };

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
    vesting_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    vesting_owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (vesting_account, _) = Pubkey::find_program_address(&[vesting_token_account.as_ref()], program_id);
    let accounts = vec![
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new(vesting_account, false),
        AccountMeta::new(*vesting_token_account, false),
        AccountMeta::new(*destination_token_account, false),
        AccountMeta::new_readonly(*vesting_owner, true),
    ];

    let instruction = VestingInstruction::Withdraw;

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

/// Creates a `Withdraw` instruction with realm
#[allow(clippy::too_many_arguments)]
pub fn withdraw_with_realm(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    vesting_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    vesting_owner: &Pubkey,
    governance_id: &Pubkey,
    realm: &Pubkey,
    mint: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (vesting_account, _) = Pubkey::find_program_address(&[vesting_token_account.as_ref()], program_id);
    let owner_record_account = get_token_owner_record_address(governance_id, realm, mint, vesting_owner);
    let voting_weight_record_account = get_voter_weight_record_address(program_id, realm, mint, vesting_owner);
    let max_voting_weight_record_account = get_max_voter_weight_record_address(program_id, realm, mint);
    let accounts = vec![
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new(vesting_account, false),
        AccountMeta::new(*vesting_token_account, false),
        AccountMeta::new(*destination_token_account, false),
        AccountMeta::new_readonly(*vesting_owner, true),

        AccountMeta::new_readonly(*governance_id, false),
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(owner_record_account, false),
        AccountMeta::new(voting_weight_record_account, false),
        AccountMeta::new(max_voting_weight_record_account, false),
    ];

    let instruction = VestingInstruction::Withdraw;

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

/// Creates a `ChangeOwner` instruction
pub fn change_owner(
    program_id: &Pubkey,
    vesting_token_account: &Pubkey,
    vesting_owner: &Pubkey,
    new_vesting_owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (vesting_account, _) = Pubkey::find_program_address(&[vesting_token_account.as_ref()], program_id);
    let accounts = vec![
        AccountMeta::new(vesting_account, false),
        AccountMeta::new_readonly(*vesting_owner, true),
        AccountMeta::new_readonly(*new_vesting_owner, false),
    ];

    let instruction = VestingInstruction::ChangeOwner;

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

/// Creates a `ChangeOwner` instruction with realm
pub fn change_owner_with_realm(
    program_id: &Pubkey,
    vesting_token_account: &Pubkey,
    vesting_owner: &Pubkey,
    new_vesting_owner: &Pubkey,
    governance_id: &Pubkey,
    realm: &Pubkey,
    mint: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let (vesting_account, _) = Pubkey::find_program_address(&[vesting_token_account.as_ref()], program_id);
    let current_owner_record_account = get_token_owner_record_address(governance_id, realm, mint, vesting_owner);
    let current_voter_weight_record_account = get_voter_weight_record_address(program_id, realm, mint, vesting_owner);
    let new_voter_weight_record_account = get_voter_weight_record_address(program_id, realm, mint, new_vesting_owner);
    let accounts = vec![
        AccountMeta::new(vesting_account, false),
        AccountMeta::new_readonly(*vesting_owner, true),
        AccountMeta::new_readonly(*new_vesting_owner, false),

        AccountMeta::new_readonly(*governance_id, false),
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(current_owner_record_account, false),
        AccountMeta::new(current_voter_weight_record_account, false),
        AccountMeta::new(new_voter_weight_record_account, false),
    ];

    let instruction = VestingInstruction::ChangeOwner;

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

/// Creates a `CreateVoterWeightRecord` instruction to create and initialize the VoterWeightRecord
#[allow(clippy::too_many_arguments)]
pub fn create_voter_weight_record(
    program_id: &Pubkey,
    record_owner: &Pubkey,
    payer: &Pubkey,
    governance_id: &Pubkey,
    realm: &Pubkey,
    mint: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let voting_weight_record_account = get_voter_weight_record_address(program_id, realm, mint, record_owner);
    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*record_owner, false),
        AccountMeta::new_readonly(*payer, true),

        AccountMeta::new_readonly(*governance_id, false),
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(voting_weight_record_account, false),
    ];

    let instruction = VestingInstruction::CreateVoterWeightRecord;

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    })
}

/// Creates a `ChangeVotePercentage` instruction with realm
#[allow(clippy::too_many_arguments)]
pub fn set_vote_percentage_with_realm(
    program_id: &Pubkey,
    vesting_owner: &Pubkey,
    vesting_authority: &Pubkey,
    governance_id: &Pubkey,
    realm: &Pubkey,
    mint: &Pubkey,
    vote_percentage: u16,
) -> Result<Instruction, ProgramError> {
    let token_owner_record_account = get_token_owner_record_address(governance_id, realm, mint, vesting_owner);
    let voter_weight_record_account = get_voter_weight_record_address(program_id, realm, mint, vesting_owner);
    let accounts = vec![
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*vesting_owner, false),
        AccountMeta::new_readonly(*vesting_authority, true),
        AccountMeta::new_readonly(*governance_id, false),
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(token_owner_record_account, false),
        AccountMeta::new(voter_weight_record_account, false),
    ];

    let instruction = VestingInstruction::SetVotePercentage { vote_percentage };

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
            schedules: vec![VestingSchedule {
                amount: 42,
                release_time: 250,
            }],
        };
        assert_eq!(
            original_deposit,
            VestingInstruction::try_from_slice(&original_deposit.try_to_vec().unwrap()).unwrap()
        );


        let original_withdraw = VestingInstruction::Withdraw;
        assert_eq!(
            original_withdraw,
            VestingInstruction::try_from_slice(&original_withdraw.try_to_vec().unwrap()).unwrap()
        );


        let original_change = VestingInstruction::ChangeOwner;
        assert_eq!(
            original_change,
            VestingInstruction::try_from_slice(&original_change.try_to_vec().unwrap()).unwrap()
        );

        let original_set_vote_percentage = VestingInstruction::SetVotePercentage { vote_percentage: 2500 };
        assert_eq!(
            original_set_vote_percentage,
            VestingInstruction::try_from_slice(&original_set_vote_percentage.try_to_vec().unwrap()).unwrap()
        );
    }
}
