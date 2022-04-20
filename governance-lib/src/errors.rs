use log::error;

use solana_sdk::pubkey::Pubkey;
use solana_sdk::decode_error::DecodeError;
use solana_client::client_error::ClientError as SolanaClientError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GovernanceLibError {
    #[error("Solana client error: {0:?}")]
    ClientError(SolanaClientError),

    #[error("State error on account {0:?}: {1:?}")]
    StateError(Pubkey,String),

    #[error("Invalid Elf data")]
    InvalidElfData(String),
    
    #[error("Std I/O error. {0:?}")]
    StdIoError(std::io::Error),

    #[error("Unknown error")]
    UnknownError,
}

impl<T> DecodeError<T> for GovernanceLibError {
    fn type_of() -> &'static str {
        "GovernanceLibError"
    }
}

impl From<SolanaClientError> for GovernanceLibError {
    fn from(e: SolanaClientError) -> GovernanceLibError {
        GovernanceLibError::ClientError(e)
    }
}

impl From<std::io::Error> for GovernanceLibError {
    fn from(e: std::io::Error) -> GovernanceLibError {
        GovernanceLibError::StdIoError(e)
    }
}
