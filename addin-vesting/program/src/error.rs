use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    program_error::{ProgramError, PrintProgramError},
    msg,
};
use thiserror::Error;

/// Errors that may be returned by the Token vesting program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum VestingError {
    #[error("Invalid Instruction")]
    InvalidInstruction,

    #[error("Missing realm accounts")]
    MissingRealmAccounts,

    #[error("Invalid Realm account")]
    InvalidRealmAccount,

    #[error("Invalid VoterWeightRecord account address")]
    InvalidVoterWeightRecordAccountAddress,

    #[error("Invalid MaxVoterWeightRecord account address")]
    InvalidMaxVoterWeightRecordAccountAddress,

    #[error("Invalid VoterWeightRecord linkage")]
    InvalidVoterWeightRecordLinkage,

    #[error("Invalid MaxVoterWeightRecord linkage")]
    InvalidMaxVoterWeightRecordLinkage,
}

impl From<VestingError> for ProgramError {
    fn from(e: VestingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for VestingError {
    fn type_of() -> &'static str {
        "VestingError"
    }
}

impl PrintProgramError for VestingError {
    fn print<E>(&self)
    {
        msg!("VESTING-ERROR: {}", &self.to_string());
    }
}
