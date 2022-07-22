//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the VoterWeightAddin program
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum VoterWeightAddinError {
    /// Wrong Token Owner
    #[error("Wrong Token Owner")]
    WrongTokenOwner,
    /// Wrong Voter Weight Record Ownership
    #[error("Wrong Voter Weight Record Ownership")]
    WrongVoterWeightRecordOwnership,
    /// Missing Required Signer
    #[error("Missing Required Signer")]
    MissingRequiredSigner,
    /// Overflow Voter Weight
    #[error("Overflow Voter Weight")]
    OverflowVoterWeight,
    /// Invalid Percentage Value
    #[error("Invalid Precentage")]
    InvalidPercentage,
}

impl PrintProgramError for VoterWeightAddinError {
    fn print<E>(&self) {
        msg!("NEON-GOVERNANCE-ADDIN-FIXED-WEIGHTS-ERROR: {}", &self.to_string());
    }
}

impl From<VoterWeightAddinError> for ProgramError {
    fn from(e: VoterWeightAddinError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for VoterWeightAddinError {
    fn type_of() -> &'static str {
        "Neon Governance Addin Fixed Weights Error"
    }
}
