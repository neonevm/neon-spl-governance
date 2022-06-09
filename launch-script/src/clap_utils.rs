use solana_clap_utils::input_validators::is_valid_pubkey;
use std::{fmt::Display};

pub fn is_valid_pubkey_or_none<T>(string: T) -> Result<(), String>
where
    T: AsRef<str> + Display,
{
    is_valid_pubkey(string.as_ref()).or_else(|_| 
        if string.as_ref() == "NONE" {
            Ok(())
        } else {
            Err("Should be a valid pubkey or NONE keyword".to_string())
        }
    )
}
