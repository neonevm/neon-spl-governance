use crate::error::VestingError;
use std::convert::TryInto;
use solana_program::{
    pubkey::Pubkey,
    program_error::ProgramError,
    program_pack::IsInitialized,
    account_info::AccountInfo,
    rent::Rent,
    sysvar::Sysvar,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use spl_governance_tools::account::{
    AccountMaxSize,
    create_and_serialize_account_signed,
    get_account_data,
};

use spl_governance_addin_api::voter_weight::VoterWeightRecord;

/// ExtendedVoterWeightRecord account
/// The account is used as an api interface to provide voting power to the governance program
/// and to save information about total amount of deposited token
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ExtendedVoterWeightRecord {
    base: VoterWeightRecord,

    /// ExtendedVoterWeightRecord discriminator sha256("account:ExtendedVoterWeightRecord")[..8]
    /// Note: The discriminator size must match the addin implementing program discriminator size
    /// to ensure it's stored in the private space of the account data and it's unique
    account_discriminator: [u8; 8],

    /// Total number of tokens owned by the account
    total_amount: u64,

    /// Percentage of the total number of tokens for calculating the voting weight
    /// (in hundredths of a percent)
    vote_percentage: u16,
}

impl ExtendedVoterWeightRecord {
    /// sha256("account:ExtendedVoterWeightRecord")[..8]
    pub const ACCOUNT_DISCRIMINATOR: [u8; 8] = [0x49, 0x6b, 0x79, 0x9a, 0xfd, 0x90, 0x5d, 0xe7];

    fn recalculate_voter_weight(&mut self) -> Result<(), ProgramError> {
        let voter_weight = (self.total_amount as u128)
                .checked_mul(self.vote_percentage.into()).ok_or(VestingError::OverflowAmount)?
                .checked_div(10000).ok_or(VestingError::OverflowAmount)?;
        self.base.voter_weight = voter_weight.try_into().map_err(|_| VestingError::OverflowAmount)?;
        Ok(())
    }

    /// Increase total_amount to specified value and recalculate current voter_weight
    pub fn increase_total_amount(&mut self, value: u64) -> Result<(), ProgramError> {
        self.total_amount = self.total_amount.checked_add(value).ok_or(VestingError::OverflowAmount)?;
        self.recalculate_voter_weight()?;
        Ok(())
    }

    /// Decrease total_amount to specified value and recalculate current voter_weight
    pub fn decrease_total_amount(&mut self, value: u64) -> Result<(), ProgramError> {
        self.total_amount = self.total_amount.checked_sub(value).ok_or(VestingError::UnderflowAmount)?;
        self.recalculate_voter_weight()?;
        Ok(())
    }

    /// Set new value for vote_percentage and recalculate current voter_weight
    pub fn set_vote_percentage(&mut self, value: u16) -> Result<(), ProgramError> {
        if value > 10000 {
            return Err(VestingError::InvalidPercentage.into());
        }
        self.vote_percentage = value;
        self.recalculate_voter_weight()?;
        Ok(())
    }
}

impl AccountMaxSize for ExtendedVoterWeightRecord {}

impl IsInitialized for ExtendedVoterWeightRecord {
    fn is_initialized(&self) -> bool {
        self.account_discriminator == ExtendedVoterWeightRecord::ACCOUNT_DISCRIMINATOR
            // Check for legacy discriminator which is not compatible with Anchor but is used by older plugins
            || self.account_discriminator == *b"496b799a"
    }
}

/// Returns ExtendedVoterWeightRecord PDA seeds
pub fn get_voter_weight_record_seeds<'a>(
    realm: &'a Pubkey,
    mint: &'a Pubkey,
    owner: &'a Pubkey,
) -> [&'a [u8]; 4] {
    [b"voter-weight-record", realm.as_ref(), mint.as_ref(), owner.as_ref()]
}

/// Returns ExtendedVoterWeightRecord PDA address
pub fn get_voter_weight_record_address(program_id: &Pubkey, realm: &Pubkey, mint: &Pubkey, owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_voter_weight_record_seeds(realm, mint, owner), program_id).0
}

/// Deserializes ExtendedVoterWeightRecord account and checks owner program
pub fn get_voter_weight_record_data(
    program_id: &Pubkey,
    voter_weight_record_info: &AccountInfo,
) -> Result<ExtendedVoterWeightRecord, ProgramError> {
    get_account_data::<ExtendedVoterWeightRecord>(program_id, voter_weight_record_info)
}

/// Deserializes ExtendedVoterWeightRecord account and checks owner program
pub fn get_voter_weight_record_data_for_seeds(
    program_id: &Pubkey,
    voter_weight_record_info: &AccountInfo,
    voter_weight_record_seeds: &[&[u8]],
) -> Result<ExtendedVoterWeightRecord, ProgramError> {
    let (voter_weight_record_address, _) =
        Pubkey::find_program_address(voter_weight_record_seeds, program_id);

    if voter_weight_record_address != *voter_weight_record_info.key {
        return Err(VestingError::InvalidVoterWeightRecordAccountAddress.into());
    }

    get_voter_weight_record_data(program_id, voter_weight_record_info)
}

/// Deserialize ExtendedVoterWeightRecord account and checks owner program and linkage
pub fn get_voter_weight_record_data_checked(
    program_id: &Pubkey,
    record_info: &AccountInfo,
    realm: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<ExtendedVoterWeightRecord, ProgramError> {
    let seeds = get_voter_weight_record_seeds(realm, mint, owner);
    let record = get_voter_weight_record_data_for_seeds(program_id, record_info, &seeds)?;
    if record.base.realm != *realm ||
       record.base.governing_token_mint != *mint ||
       record.base.governing_token_owner != *owner {
           return Err(VestingError::InvalidVoterWeightRecordLinkage.into())
    }
    Ok(record)
}

/// Create Voter Weight Record
#[allow(clippy::too_many_arguments)]
pub fn create_voter_weight_record<'a, I>(
    program_id: &Pubkey,
    realm: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    payer_account: &AccountInfo<'a>,
    record_account: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    initialize_func: I
) -> Result<(), ProgramError>
where I: FnOnce(&mut ExtendedVoterWeightRecord) -> Result<(), ProgramError>
{
    let mut record_data = ExtendedVoterWeightRecord {
        base: VoterWeightRecord {
            account_discriminator: VoterWeightRecord::ACCOUNT_DISCRIMINATOR,
            realm: *realm,
            governing_token_mint: *mint,
            governing_token_owner: *owner,
            voter_weight: 0,
            voter_weight_expiry: None,
            weight_action: None,
            weight_action_target: None,
            reserved: [0u8; 8],
        },
        account_discriminator: ExtendedVoterWeightRecord::ACCOUNT_DISCRIMINATOR,
        total_amount: 0,
        vote_percentage: 10_000,
    };
    initialize_func(&mut record_data)?;
    create_and_serialize_account_signed::<ExtendedVoterWeightRecord>(
        payer_account,
        record_account,
        &record_data,
        &get_voter_weight_record_seeds(realm, mint, owner),
        program_id,
        system_program_account,
        &Rent::get()?
    )?;
    Ok(())
}
