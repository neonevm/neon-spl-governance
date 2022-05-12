//! Program entrypoint

use crate::{error::VoterWeightAddinError, processor};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult,
    program_error::PrintProgramError, pubkey::Pubkey,
};

#[cfg(all(target_arch = "bpf", not(feature = "no-entrypoint")))]
use solana_program::entrypoint;

#[cfg(all(target_arch = "bpf", not(feature = "no-entrypoint")))]
entrypoint!(process_instruction);

/// Entrypoint
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = processor::process_instruction(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<VoterWeightAddinError>();
        return Err(error);
    }
    Ok(())
}
