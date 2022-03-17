//! Program processor

use borsh::BorshDeserialize;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_addin_api::{
    max_voter_weight::MaxVoterWeightRecord,
    voter_weight::{VoterWeightRecord},
};
use spl_governance_tools::account::create_and_serialize_account_signed;

use crate::{
    config::VOTER_WEIGHT_SEED_VERSION,
    error::VoterWeightAddinError,
    instruction::VoterWeightAddinInstruction
};

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = VoterWeightAddinInstruction::try_from_slice(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("GOVERNANCE-VOTER-WEIGHT-INSTRUCTION: {:?}", instruction);

    match instruction {
        VoterWeightAddinInstruction::SetupVoterWeightRecord { } => process_setup_voter_weight_record(
            program_id,
            accounts,
        ),
        VoterWeightAddinInstruction::SetupMaxVoterWeightRecord { } => process_setup_max_voter_weight_record(
            program_id,
            accounts,
        ),
    }
}

/// Processes SetupVoterWeightRecord instruction
pub fn process_setup_voter_weight_record(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 1
    let governing_token_owner_info = next_account_info(account_info_iter)?; // 2
    let voter_weight_record_info = next_account_info(account_info_iter)?; // 3
    let payer_info = next_account_info(account_info_iter)?; // 4
    let system_info = next_account_info(account_info_iter)?; // 5

    /// Check:
    /// 1. realm_info.key
    /// 2. governing_token_mint_info.key

    let voter_weight: u64 = get_voter_weight_fixed(governing_token_owner_info.key)?;

    let voter_weight_record_data = VoterWeightRecord {
        account_discriminator: VoterWeightRecord::ACCOUNT_DISCRIMINATOR,
        realm: *realm_info.key,
        governing_token_mint: *governing_token_mint_info.key,
        governing_token_owner: *governing_token_owner_info.key,
        voter_weight,
        voter_weight_expiry: None,
        weight_action: None,
        weight_action_target: None,
        reserved: [0; 8],
    };

    let seeds: &[&[u8]] = &[b"voter_weight", &[VOTER_WEIGHT_SEED_VERSION], &realm_info.key.to_bytes(), &governing_token_mint_info.key.to_bytes(), &governing_token_owner_info.key.to_bytes()];
    let rent = Rent::get()?;

    create_and_serialize_account_signed(
        payer_info,
        voter_weight_record_info,
        &voter_weight_record_data,
        seeds,
        program_id,
        system_info,
        &rent,
    )?;

    Ok(())
}

/// Processes SetupMaxVoterWeightRecord instruction
pub fn process_setup_max_voter_weight_record(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 1
    let max_voter_weight_record_info = next_account_info(account_info_iter)?; // 2
    let payer_info = next_account_info(account_info_iter)?; // 3
    let system_info = next_account_info(account_info_iter)?; // 4

    let max_voter_weight: u64 = get_max_voter_weight_fixed();

    /// Check:
    /// 1. realm_info.key
    /// 2. governing_token_mint_info.key

    let max_voter_weight_record_data = MaxVoterWeightRecord {
        account_discriminator: MaxVoterWeightRecord::ACCOUNT_DISCRIMINATOR,
        realm: *realm_info.key,
        governing_token_mint: *governing_token_mint_info.key,
        max_voter_weight,
        max_voter_weight_expiry: None,
        reserved: [0; 8],
    };

    let seeds: &[&[u8]] = &[b"max_voter_weight", &[VOTER_WEIGHT_SEED_VERSION], &realm_info.key.to_bytes(), &governing_token_mint_info.key.to_bytes()];
    let rent = Rent::get()?;

    create_and_serialize_account_signed(
        payer_info,
        max_voter_weight_record_info,
        &max_voter_weight_record_data,
        seeds,
        program_id,
        system_info,
        &rent,
    )?;

    Ok(())
}

/// Get Fixed Voter Weight
fn get_max_voter_weight_fixed() -> u64 {
    crate::config::VOTER_LIST
        .iter()
        .fold(0, |acc, item| acc + item.1)
}

/// Get Fixed Voter Weight
fn get_voter_weight_fixed(token_owner: &Pubkey) -> Result<u64,ProgramError> {
    crate::config::VOTER_LIST
        .iter()
        .find(|&&item| item.0 == *token_owner )
        .map(|item| item.1 )
        .ok_or(VoterWeightAddinError::WrongTokenOwner.into())
}
