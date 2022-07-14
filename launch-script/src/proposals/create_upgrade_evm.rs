// =========================================================================
// Deploy evm_loader from buffer account
// =========================================================================
use std::str::FromStr;
use std::collections::HashMap;
use solana_sdk::{
    hash::{ hash, Hash, },
};
use goblin::elf::{ Elf };
use crate::prelude::*;

fn parse_elf_params(elf: Elf, program_data: &Vec<u8>) -> HashMap<String,String> {
    let mut elf_params: HashMap::<String,String> = HashMap::new();

    elf.dynsyms.iter().for_each(|sym| {
        let name = String::from(&elf.dynstrtab[sym.st_name]);
        if name.starts_with("NEON_") {
            let end = program_data.len();
            let from: usize = usize::try_from(sym.st_value).unwrap_or_else(|_| panic!("Unable to cast usize from u64:{:?}", sym.st_value));
            let to: usize = usize::try_from(sym.st_value + sym.st_size).unwrap_or_else(|err| panic!("Unable to cast usize from u64:{:?}. Error: {}", sym.st_value + sym.st_size, err));
            if to < end && from < end {
                let buf = &program_data[from..to];
                let value = std::str::from_utf8(buf).unwrap();
                elf_params.insert(name, String::from(value));
            } else {
                panic!("{} is out of bounds", name);
            }
        };
    });
    elf_params
}

pub fn create_upgrade_evm(wallet: &Wallet, client: &Client,
        transaction_inserter: &mut ProposalTransactionInserter,
        cfg: &Configuration,
        buffer_pubkey: Pubkey,
) -> Result<(), ScriptError> {

    let realm = Realm::new(client, &wallet.governance_program_id, REALM_NAME, &wallet.community_pubkey);
    let maintenance_governance = realm.governance(&wallet.neon_evm_program_id);
    let maintenance_governance_pubkey: Pubkey = maintenance_governance.governance_address;

    let (maintenance_record_pubkey,_): (Pubkey,u8) =
        get_maintenance_record_address(&wallet.maintenance_program_id, &wallet.neon_evm_program_id);

    println!("Maintenance DAO Address: {:?}", maintenance_governance_pubkey);
    println!("Maintenance Record Address: {:?}", maintenance_record_pubkey);

    let executor = TransactionExecutor {
        client,
        setup: transaction_inserter.setup,
        verbose: transaction_inserter.verbose,
    };

    let upgrade_authority_opt = client.get_program_upgrade_authority(&wallet.neon_evm_program_id)?;
    executor.check_and_create_object(
        &format!("EVM loader upgrade authority: {:?}", upgrade_authority_opt),
        upgrade_authority_opt,
        |upgrade_authority| {
            if *upgrade_authority != maintenance_record_pubkey {
                return Err( StateError::WrongEvmLoaderUpgradeAuthority.into() );
            }
            Ok(None)
        },
        || Err( StateError::WrongEvmLoaderUpgradeAuthority.into() ),
    )?;

    executor.check_and_create_object(
        "Program Buffer",
        Some(client.get_program_data(&buffer_pubkey)?),
        |program_data| {
            let elf = Elf::parse(program_data).expect("Can't parse Elf data");
            let elf_params = parse_elf_params(elf, program_data);

            // println!("elf_params: {:?}", elf_params);
            if elf_params.get("NEON_TOKEN_MINT")
                    .and_then(|value| Pubkey::from_str(value.as_str()).ok() )
                    .map(|neon_mint| neon_mint != wallet.community_pubkey )
                    .unwrap_or(true) {
                return Err( StateError::InvalidNeonMint.into() );
            }
            if elf_params.get("NEON_TOKEN_MINT_DECIMALS")
                    .and_then(|value| value.parse().ok() )
                    .map(|decimals: u32| decimals != 9 )
                    .unwrap_or(true) {
                return Err( StateError::WrongNeonDecimals.into() );
            }
            if elf_params.get("NEON_POOL_BASE")
                    .and_then(|value| Pubkey::from_str(value.as_str()).ok() )
                    .map(|neon_pool_base| neon_pool_base != wallet.maintenance_program_id )
                    .unwrap_or(true) {
                return Err( StateError::InvalidPoolBase.into() );
            }
            if elf_params.get("NEON_CHAIN_ID")
                    .and_then(|value| value.parse().ok() )
                    .map(|chain_id: u64| chain_id != cfg.chain_id )
                    .unwrap_or(true) {
                return Err( StateError::WrongChainId.into() );
            }
            let buffer_hash: Hash = hash(program_data);
            if !cfg.code_hashes
                    .iter()
                    .any(|&code_hash| code_hash == buffer_hash ) {
                return Err( StateError::WrongCodeHash.into() );
            }
            Ok(None)
        },
        || Err( StateError::MissingProgramBuffer(buffer_pubkey).into() ),
    )?;

    transaction_inserter.insert_transaction_checked(
            "Set delegates for Neon EVM upgrade",
            vec![
                maintenance::instruction::set_delegate(
                    &wallet.maintenance_program_id,
                    &wallet.neon_evm_program_id,
                    cfg.delegates.clone(),
                    &maintenance_governance_pubkey,
                ).into(),
            ],
        )?;

    transaction_inserter.insert_transaction_checked(
            "Set code hashes for Neon EVM upgrade",
            vec![
                maintenance::instruction::set_code_hashes(
                    &wallet.maintenance_program_id,
                    &wallet.neon_evm_program_id,
                    cfg.code_hashes.clone(),
                    &maintenance_governance_pubkey,
                ).into(),
            ],
        )?;

    transaction_inserter.insert_transaction_checked(
            &format!("Upgrade evm_loader from buffer at address {}", buffer_pubkey),
            vec![
                maintenance::instruction::upgrade(
                    &wallet.maintenance_program_id,
                    &wallet.neon_evm_program_id,
                    &maintenance_governance_pubkey,
                    &buffer_pubkey,
                    &client.payer.pubkey(),
                ).into(),
            ],
        )

}
