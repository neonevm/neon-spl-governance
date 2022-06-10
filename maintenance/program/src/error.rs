//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the MaintenanceProgram program
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum MaintenanceError {
    /// Missing Required Signer
    #[error("Missing Required Signer")]
    MissingRequiredSigner,
    /// Wrong Authority
    #[error("Wrong Authority")]
    WrongAuthority,
    /// Buffer Data Offset Error
    #[error("Buffer Data Offset Error")]
    BufferDataOffsetError,
    /// Authority Deserialization Error
    #[error("Authority Deserialization Error")]
    AuthorityDeserializationError,
    /// Number of Delegates Exceeds Limit
    #[error("Number of Delegates Exceeds Limit")]
    NumberOfDelegatesExceedsLimit,
    /// Number of Code Hashes Exceeds Limit
    #[error("Number of Code Hashes Exceeds Limit")]
    NumberOfCodeHashesExceedsLimit,
    /// Wrong Authority Delegate
    #[error("Wrong Authority Delegate")]
    WrongDelegate,
    /// Wrong Upgrade Code Hash
    #[error("Wrong Upgrade Code Hash")]
    WrongCodeHash,
    /// Maintenance Record Account Matches Spill Account And Can't Be Closed
    #[error("Maintenance Record Account Matches Spill Account")]
    MaintenanceRecordAccountMatchesSpillAccount,
    /// Wrong Program Data For Maintenance Record
    #[error("Wrong Program Data For Maintenance Record")]
    WrongProgramDataForMaintenanceRecord,
    /// Maintenance Record Must be Inactive
    #[error("Maintenance record must be inactive")]
    RecordMustBeInactive,
}

impl PrintProgramError for MaintenanceError {
    fn print<E>(&self) {
        msg!("NEON-MAINTENANCE-ERROR: {}", &self.to_string());
    }
}

impl From<MaintenanceError> for ProgramError {
    fn from(e: MaintenanceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for MaintenanceError {
    fn type_of() -> &'static str {
        "Neon Maintenance Error"
    }
}
