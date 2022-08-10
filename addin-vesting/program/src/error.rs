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
    #[error("Invalid Vesting account")]
    InvalidVestingAccount,

    #[error("Vesting account already exists")]
    VestingAccountAlreadyExists,

    #[error("Missing required signer")]
    MissingRequiredSigner,

    #[error("Invalid Vesting token account")]
    InvalidVestingTokenAccount,

    #[error("InvalidOwnerForVestingAccount")]
    InvalidOwnerForVestingAccount,

    #[error("NotReachedReleaseTime")]
    NotReachedReleaseTime,

    #[error("OverflowAmount")]
    OverflowAmount,

    #[error("UnderflowAmount")]
    UnderflowAmount,

    #[error("Insufficient funds on source token account")]
    InsufficientFunds,

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

    #[error("VestingIsNotUnderRealm")]
    VestingIsNotUnderRealm,

    #[error("InvalidPercentage")]
    InvalidPercentage,

    #[error("Invalid TokenOwnerRecord")]
    InvalidTokenOwnerRecord,

    #[error("Vesting not empty")]
    VestingNotEmpty,

    #[error("Voter Weight Record not empty")]
    VoterWeightRecordNotEmpty,
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
