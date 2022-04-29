//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

/// Instructions supported by the VoterWeight addin program
/// This program is a mock program used by spl-governance for testing and not real addin
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum VoterWeightAddinInstruction {
    /// Sets up VoterWeightRecord owned by the program
    ///
    /// 0. `[]` Realm account
    /// 1. `[]` Governing Token mint
    /// 2. `[]` Governing token owner
    /// 3. `[writable]` VoterWeightRecord
    /// 4. `[signer]` Payer
    /// 5. `[]` System
    SetupVoterWeightRecord { },
    /// Sets up MaxVoterWeightRecord owned by the program
    ///
    /// 0. `[]` Realm account
    /// 1. `[]` Governing Token mint
    /// 2. `[writable]` MaxVoterWeightRecord
    /// 3. `[signer]` Payer
    /// 4. `[]` System
    SetupMaxVoterWeightRecord { },
}


/// Get VoterVeightRecord account address and bump seed
pub fn get_voter_weight_address(program_id: &Pubkey, realm: &Pubkey, governing_token_mint: &Pubkey, governing_token_owner: &Pubkey) -> (Pubkey, u8) {
    let seeds: &[&[u8]] = &[b"voter-weight-record", &realm.to_bytes(), &governing_token_mint.to_bytes(), &governing_token_owner.to_bytes()];
    Pubkey::find_program_address(seeds, program_id)
}

/// Get MaxVoterVeightRecord account address and bump seed
pub fn get_max_voter_weight_address(program_id: &Pubkey, realm: &Pubkey, governing_token_mint: &Pubkey) -> (Pubkey, u8) {
    let seeds: &[&[u8]] = &[b"max-voter-weight-record", &realm.to_bytes(), &governing_token_mint.to_bytes()];
    Pubkey::find_program_address(seeds, program_id)
}

/// Creates SetupVoterWeightRecord instruction
#[allow(clippy::too_many_arguments)]
pub fn setup_voter_weight_record(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
    payer: &Pubkey,
) -> Instruction {

    let (voter_weight_record, _): (Pubkey, u8) = get_voter_weight_address(program_id, realm, governing_token_mint, governing_token_owner);

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new_readonly(*governing_token_owner, false),
        AccountMeta::new(voter_weight_record, false),
        AccountMeta::new_readonly(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = VoterWeightAddinInstruction::SetupVoterWeightRecord { };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates SetupMaxVoterWeightRecord instruction
#[allow(clippy::too_many_arguments)]
pub fn setup_max_voter_weight_record(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    payer: &Pubkey,
) -> Instruction {

    let (max_voter_weight_record, _): (Pubkey, u8) = get_max_voter_weight_address(program_id, realm, governing_token_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new(max_voter_weight_record, false),
        AccountMeta::new_readonly(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = VoterWeightAddinInstruction::SetupMaxVoterWeightRecord { };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}
