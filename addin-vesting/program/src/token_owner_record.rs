use crate::error::VestingError;
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

use spl_governance::state::{
    token_owner_record::{
        TokenOwnerRecordV2,
        get_token_owner_record_data_for_seeds,
    },
};

pub fn get_token_owner_record_data_if_exists(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    token_owner_record_seeds: &[&[u8]],
) -> Result<Option<TokenOwnerRecordV2>, ProgramError> {
    let (token_owner_record_address, _) = 
        Pubkey::find_program_address(token_owner_record_seeds, program_id);

    if token_owner_record_address != *token_owner_record_info.key {
        return Err(VestingError::InvalidTokenOwnerRecord.into());
    }

    if token_owner_record_info.data_is_empty() {
        if *token_owner_record_info.owner != system_program::id() {
            return Err(VestingError::InvalidTokenOwnerRecord.into());
        }

        Ok(None)
    } else {
        Ok(Some(get_token_owner_record_data_for_seeds(
            program_id,
            token_owner_record_info,
            token_owner_record_seeds)?
        ))
    }
}
