use crate::error::VestingError;
use solana_program::{
    pubkey::Pubkey,
    program_error::ProgramError,
    account_info::AccountInfo,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance::addins::voter_weight::get_voter_weight_record_data;
use spl_governance_tools::account::{
    get_account_data,
    create_and_serialize_account_signed,
};

pub use spl_governance_addin_api::voter_weight::VoterWeightRecord;

/// Returns VoterWeightRecord PDA seeds
pub fn get_voter_weight_record_seeds<'a>(
    realm: &'a Pubkey,
    mint: &'a Pubkey,
    owner: &'a Pubkey,
) -> [&'a [u8]; 4] {
    [b"voter_weight", realm.as_ref(), mint.as_ref(), owner.as_ref()]
}

/// Returns VoterWeightRecord PDA address
pub fn get_voter_weight_record_address(program_id: &Pubkey, realm: &Pubkey, mint: &Pubkey, owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_voter_weight_record_seeds(realm, mint, owner), program_id).0
}

/// Deserializes VoterWeightRecord account and checks owner program
pub fn get_voter_weight_record_data_for_seeds(
    program_id: &Pubkey,
    voter_weight_record_info: &AccountInfo,
    voter_weight_record_seeds: &[&[u8]],
) -> Result<VoterWeightRecord, ProgramError> {
    let (voter_weight_record_address, _) =
        Pubkey::find_program_address(voter_weight_record_seeds, program_id);

    if voter_weight_record_address != *voter_weight_record_info.key {
        return Err(VestingError::InvalidVoterWeightRecordAccountAddress.into());
    }

    get_voter_weight_record_data(program_id, voter_weight_record_info)
}

/// Deserialize VoterWeightRecord account and checks owner program and linkage
pub fn get_voter_weight_record_data_checked(
    program_id: &Pubkey,
    record_info: &AccountInfo,
    realm: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<VoterWeightRecord, ProgramError> {
    let seeds = get_voter_weight_record_seeds(realm, mint, owner);
    let record = get_voter_weight_record_data_for_seeds(program_id, record_info, &seeds)?;
    if record.realm != *realm ||
       record.governing_token_mint != *mint ||
       record.governing_token_owner != *owner {
           return Err(VestingError::InvalidVoterWeightRecordLinkage.into())
    }
    Ok(record)
}

/// Create Voter Weight Record
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
where I: FnOnce(&mut VoterWeightRecord) -> Result<(), ProgramError>
{
    let mut record_data = VoterWeightRecord {
        account_discriminator: VoterWeightRecord::ACCOUNT_DISCRIMINATOR,
        realm: *realm,
        governing_token_mint: *mint,
        governing_token_owner: *owner,
        voter_weight: 0,
        voter_weight_expiry: None,
        weight_action: None,
        weight_action_target: None,
        reserved: [0u8; 8],
    };
    initialize_func(&mut record_data)?;
    create_and_serialize_account_signed::<VoterWeightRecord>(
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
