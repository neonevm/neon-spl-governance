use crate::error::VestingError;
use solana_program::{
    pubkey::Pubkey,
    program_error::ProgramError,
    account_info::AccountInfo,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance::addins::max_voter_weight::get_max_voter_weight_record_data;
use spl_governance_tools::account::{
    get_account_data,
    create_and_serialize_account_signed,
};

pub use spl_governance_addin_api::max_voter_weight::MaxVoterWeightRecord;

/// Returns MaxVoterWeightRecord PDA seeds
pub fn get_max_voter_weight_record_seeds<'a>(
    realm: &'a Pubkey,
    mint: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [b"max_voter_weight", realm.as_ref(), mint.as_ref()]
}

/// Returns MaxVoterWeightRecord PDA address
pub fn get_max_voter_weight_record_address(program_id: &Pubkey, realm: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_max_voter_weight_record_seeds(realm, mint), program_id).0
}

/// Deserializes MaxVoterWeightRecord account and checks owner program
pub fn get_max_voter_weight_record_data_for_seeds(
    program_id: &Pubkey,
    max_voter_weight_record_info: &AccountInfo,
    max_voter_weight_record_seeds: &[&[u8]],
) -> Result<MaxVoterWeightRecord, ProgramError> {
    let (max_voter_weight_record_address, _) =
        Pubkey::find_program_address(max_voter_weight_record_seeds, program_id);

    if max_voter_weight_record_address != *max_voter_weight_record_info.key {
        return Err(VestingError::InvalidMaxVoterWeightRecordAccountAddress.into());
    }

    get_max_voter_weight_record_data(program_id, max_voter_weight_record_info)
}

/// Deserializes MaxVoterWeightRecord account and checks owner program and linkage
pub fn get_max_voter_weight_record_data_checked(
    program_id: &Pubkey,
    record_info: &AccountInfo,
    realm: &Pubkey,
    mint: &Pubkey,
) -> Result<MaxVoterWeightRecord, ProgramError> {
    let seeds = get_max_voter_weight_record_seeds(realm, mint);
    let record = get_max_voter_weight_record_data_for_seeds(program_id, record_info, &seeds)?;
    if record.realm != *realm ||
       record.governing_token_mint != *mint {
           return Err(VestingError::InvalidMaxVoterWeightRecordLinkage.into())
    }
    Ok(record)
}

/// Create Voter Weight Record
pub fn create_max_voter_weight_record<'a, I>(
    program_id: &Pubkey,
    realm: &Pubkey,
    mint: &Pubkey,
    payer_account: &AccountInfo<'a>,
    record_account: &AccountInfo<'a>,
    system_program_account: &AccountInfo<'a>,
    initialize_func: I
) -> Result<(), ProgramError>
where I: FnOnce(&mut MaxVoterWeightRecord) -> Result<(), ProgramError>
{
    let mut record_data = MaxVoterWeightRecord {
        account_discriminator: MaxVoterWeightRecord::ACCOUNT_DISCRIMINATOR,
        realm: *realm,
        governing_token_mint: *mint,
        max_voter_weight: 0,
        max_voter_weight_expiry: None,
        reserved: [0u8; 8],
    };
    initialize_func(&mut record_data)?;
    create_and_serialize_account_signed::<MaxVoterWeightRecord>(
        payer_account,
        record_account,
        &record_data,
        &get_max_voter_weight_record_seeds(realm, mint),
        program_id,
        system_program_account,
        &Rent::get()?
    )?;
    Ok(())
}

