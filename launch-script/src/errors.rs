//! Error types
#![allow(clippy::use_self)]
#![allow(clippy::cast_possible_wrap)]

use log::error;

use solana_sdk::pubkey::{Pubkey, PubkeyError};
//use solana_sdk::signer::SignerError as SolanaSignerError;
use solana_sdk::decode_error::DecodeError;
use solana_sdk::program_option::COption;
use solana_sdk::program_error::ProgramError as SolanaProgramError;
use solana_client::client_error::ClientError as SolanaClientError;
//use solana_client::tpu_client::TpuSenderError as SolanaTpuSenderError;
use thiserror::Error;
use governance_lib::errors::GovernanceLibError;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Invalid mint-authority {1:?} for mint {0:?}")]
    InvalidMintAuthority(Pubkey,COption<Pubkey>),

    #[error("Invalid freeze-authority {1:?} for mint {0:?}")]
    InvalidFreezeAuthority(Pubkey,COption<Pubkey>),

    #[error("Invalid community mint {1:?} for realm {0:?}")]
    InvalidRealmCommunityMint(Pubkey,Pubkey),

    #[error("Invalid authority {1:?} for realm {0:?}")]
    InvalidRealmAuthority(Pubkey,Option<Pubkey>),

    #[error("Missing token_owner record for {0:?}")]
    MissingTokenOwnerRecord(Pubkey),

    #[error("Invalid delegate {1:?} for {0:?}")]
    InvalidDelegate(Pubkey,Option<Pubkey>),

    #[error("Missing Mint {0:?}")]
    MissingMint(Pubkey),

    #[error("Missing spl-token account {0:?}")]
    MissingSplTokenAccount(Pubkey),

    #[error("Missing realm {0:?}")]
    MissingRealm(Pubkey),

    #[error("Missing proposal {0:?}")]
    MissingProposal(Pubkey),

    #[error("Invalid program upgrade authority {1:?} for {0:?}")]
    InvalidProgramUpgradeAuthority(Pubkey,Option<Pubkey>),

    #[error("Invalid token account mint {1:?} for {0:?}")]
    InvalidTokenAccountMint(Pubkey,Pubkey),

    #[error("Invalid token account owner {1:?} for {0:?}")]
    InvalidTokenAccountOwner(Pubkey,Pubkey),

    #[error("Invalid proposal_index")]
    InvalidProposalIndex,

    #[error("Invalid proposal")]
    InvalidProposal,

    #[error("Invalid proposal transaction {0:?}")]
    InvalidProposalTransaction(u16),

    #[error("Missing addin keypair for {0:?}")]
    MissingAddinKeypair(String),

    #[error("Proposal transaction require additional signer {0:?}")]
    RequireAdditionalSigner(Pubkey),

    #[error("Unknown MultiSig {0:?}")]
    UnknownMultiSig(String),

    #[error("Invalid voter list")]
    InvalidVoterList,
}

/// Errors that may be returned by the neon-cli program.
#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("Std error {0:?}")]
    StdError(Box<dyn std::error::Error>),

    /// Std IO Error
    #[error("Std I/O error. {0:?}")]
    StdIoError(std::io::Error),

    /// Solana Client Error
    #[error("Solana program error. {0:?}")]
    ProgramError(SolanaProgramError),

    /// Solana Client Error
    #[error("Solana client error. {0:?}")]
    ClientError(SolanaClientError),

    #[error("Governance lib error {0:?}")]
    GovernanceLibError(GovernanceLibError),

    #[error("State error: {0:?}")]
    StateError(StateError),

    #[error("Pubkey error: {0:?}")]
    PubkeyError(PubkeyError),

//    /// Solana Signer Error
//    #[error("Solana signer error. {0:?}")]
//    SignerError(SolanaSignerError),
//
//    /// TPU Sender Error
//    #[error("TPU sender error. {0:?}")]
//    TpuSenderError(SolanaTpuSenderError),
//
//    /// Need specify evm_loader
//    #[error("EVM loader must be specified.")]
//    EvmLoaderNotSpecified,
//
//    /// Need specify fee payer
//    #[error("Fee payer must be specified.")]
//    FeePayerNotSpecified,
//
//    /// Account is already initialized.
//    #[error("Account is already initialized.  account={0:?}, code_account={1:?}")]
//    AccountAlreadyInitialized(Pubkey,Pubkey),
//
//    /// Invalid storage account owner
//    #[error("Invalid storage account owner {0:?}.")]
//    InvalidStorageAccountOwner(Pubkey),
//
//    /// Account data too small
//    #[error("Account data too small. account_data.len()={0:?} < end={1:?}")]
//    AccountDataTooSmall(usize,usize),
//
//    /// Account not found
//    #[error("Account not found {0:?}.")]
//    AccountNotFound(Pubkey),
//
//    /// Account is not BFP
//    #[error("Account is not BPF {0:?}.")]
//    AccountIsNotBpf(Pubkey),
//
//    /// Account is not upgradeable
//    #[error("Account is not upgradeable {0:?}.")]
//    AccountIsNotUpgradeable(Pubkey),
//
//    /// Program data account not found
//    #[error("Associated PDA not found {0:?} for Program {1:?}.")]
//    AssociatedPdaNotFound(Pubkey,Pubkey),
//
//    /// Program data account not found
//    #[error("Invalid Associated PDA {0:?} for Program {1:?}.")]
//    InvalidAssociatedPda(Pubkey,Pubkey),
//
//    /// Invalid message verbosity
//    #[error("Invalid verbosity message.")]
//    InvalidVerbosityMessage,
//
//    /// Transaction failed
//    #[error("Transaction failed.")]
//    TransactionFailed,
//
//    /// too many steps
//    #[error("Too many steps")]
//    TooManySteps,
//
//    // Account nonce exceeds u64::max
//    #[error("Transaction count overflow")]
//    TrxCountOverflow,

    /// Unknown Error.
    #[error("Unknown error.")]
    UnknownError
}

/*impl ScriptError {
    pub fn error_code(&self) -> u32 {
        match self {
//            ScriptError::StdIoError(_)                     => 102, // => 1002,
//            ScriptError::ProgramError(_)                   => 111, // => 1011,
//            ScriptError::SignerError(_)                    => 112, // => 1012,
            ScriptError::ClientError(_)                    => 113, // => 1013,
//            ScriptError::TpuSenderError(_)                 => 115, // => 1015,
//            ScriptError::EvmLoaderNotSpecified             => 201, // => 4001,
//            ScriptError::FeePayerNotSpecified              => 202, // => 4002,
//            ScriptError::AccountNotFound(_)                => 205, // => 4005,
//            ScriptError::AccountAlreadyInitialized(_,_)    => 213, // => 4013,
//            ScriptError::InvalidStorageAccountOwner(_)     => 222, // => 4022,
//            ScriptError::AccountDataTooSmall(_,_)          => 225, // => 4025,
//            ScriptError::AccountIsNotBpf(_)                => 226, // => 4026,
//            ScriptError::AccountIsNotUpgradeable(_)        => 227, // => 4027,
//            ScriptError::AssociatedPdaNotFound(_,_)        => 241, // => 4041,
//            ScriptError::InvalidAssociatedPda(_,_)         => 242, // => 4042,
//            ScriptError::InvalidVerbosityMessage           => 243, // => 4100,
//            ScriptError::TransactionFailed                 => 244, // => 4200,
//            ScriptError::TooManySteps                      => 245,
//            ScriptError::TrxCountOverflow                  => 246,
            ScriptError::UnknownError                      => 249, // => 4900,
        }
    }
}*/

impl From<std::io::Error> for ScriptError {
    fn from(e: std::io::Error) -> ScriptError {
        ScriptError::StdIoError(e)
    }
}

impl From<Box<dyn std::error::Error>> for ScriptError {
    fn from(e: Box<dyn std::error::Error>) -> ScriptError {
        ScriptError::StdError(e)
    }
}

impl From<SolanaClientError> for ScriptError {
    fn from(e: SolanaClientError) -> ScriptError {
        ScriptError::ClientError(e)
    }
}

impl From<GovernanceLibError> for ScriptError {
    fn from(e: GovernanceLibError) -> ScriptError {
        ScriptError::GovernanceLibError(e)
    }
}

impl From<StateError> for ScriptError {
    fn from(e: StateError) -> ScriptError {
        ScriptError::StateError(e)
    }
}

impl From<PubkeyError> for ScriptError {
    fn from(e: PubkeyError) -> ScriptError {
        ScriptError::PubkeyError(e)
    }
}

impl From<SolanaProgramError> for ScriptError {
    fn from(e: SolanaProgramError) -> ScriptError {
        ScriptError::ProgramError(e)
    }
}

//impl From<SolanaSignerError> for ScriptError {
//    fn from(e: SolanaSignerError) -> ScriptError {
//        ScriptError::SignerError(e)
//    }
//}
//
//impl From<SolanaTpuSenderError> for ScriptError {
//    fn from(e: SolanaTpuSenderError) -> ScriptError {
//        ScriptError::TpuSenderError(e)
//    }
//}

impl<T> DecodeError<T> for ScriptError {
    fn type_of() -> &'static str {
        "ScriptError"
    }
}

