use std::str::FromStr;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::{ hash, Hash, },
    pubkey::{ Pubkey },
    instruction::{ Instruction },
    transaction::{ Transaction },
    signer::{
        Signer,
        keypair::{ Keypair, read_keypair_file },
    },
    bpf_loader_upgradeable::{
        self,
        set_buffer_authority,
        set_upgrade_authority,
        UpgradeableLoaderState,
    },
};

use solana_client::rpc_client::RpcClient;

use borsh::{BorshDeserialize};

use maintenance::{
    instruction::{
        create_maintenance,
        set_delegate,
        set_code_hashes,
        upgrade,
        set_authority,
        close_maintenance,
        get_maintenance_record_address,
    },
    state::{
        MaintenanceRecord,
    },
};

const MAINTENANCE_KEY_FILE_PATH: &'static str = "../artifacts/maintenance.keypair";
const MAINTAIN_AUTHORITY_KEY_FILE_PATH: &'static str = "../artifacts/voter1.keypair";
const NEW_MAINTAIN_AUTHORITY_KEY_FILE_PATH: &'static str = "../artifacts/voter2.keypair";
const ADDIN_INITIAL_AUTHORITY_KEY_FILE_PATH: &'static str = "../artifacts/voter3.keypair";
const ADDIN_KEY_FILE_PATH: &'static str = "../artifacts/addin-fixed-weights.keypair";
const BUFFER_ADDRESS: &'static str = "447xS9Kc6m1Dg8UEnyMFrnao5ZL82HTaq7UEjfLZDWEb";

fn main() {

    let solana_client = RpcClient::new_with_commitment("http://localhost:8899".to_string(),CommitmentConfig::confirmed());

    let maintenance_keypair: Keypair = read_keypair_file(MAINTENANCE_KEY_FILE_PATH).unwrap();
    let maintenance_pubkey: Pubkey = maintenance_keypair.pubkey();
    println!("Maintenance Program Id: {}", maintenance_pubkey);

    let maintain_authority_keypair: Keypair = read_keypair_file(MAINTAIN_AUTHORITY_KEY_FILE_PATH).unwrap();
    let maintain_authority_pubkey: Pubkey = maintain_authority_keypair.pubkey();
    println!("Maintenance Engeneer Authority Pubkey: {}", maintain_authority_pubkey);

    let new_maintain_authority_keypair: Keypair = read_keypair_file(NEW_MAINTAIN_AUTHORITY_KEY_FILE_PATH).unwrap();
    let new_maintain_authority_pubkey: Pubkey = new_maintain_authority_keypair.pubkey();
    println!("New Maintenance Authority Pubkey: {}", new_maintain_authority_pubkey);

    let maintained_addin_initial_authority_keypair: Keypair = read_keypair_file(ADDIN_INITIAL_AUTHORITY_KEY_FILE_PATH).unwrap();
    let maintained_addin_initial_authority_pubkey = maintained_addin_initial_authority_keypair.pubkey();
    println!("Maintained Addin Initial Authority Pubkey: {}", maintained_addin_initial_authority_pubkey);

    let maintained_addin_keypair: Keypair = read_keypair_file(ADDIN_KEY_FILE_PATH).unwrap();
    let maintained_addin_pubkey = maintained_addin_keypair.pubkey();
    println!("Maintained Addin Pubkey: {}", maintained_addin_pubkey);
    // let maintained_address = Pubkey::new_unique();
    // println!("Maintained Address: {}", maintained_address);
    let (maintained_addin_programdata_address, _) = Pubkey::find_program_address(&[maintained_addin_pubkey.as_ref()], &bpf_loader_upgradeable::id());
    println!("Maintained Addin Programdata Address: {}", maintained_addin_programdata_address);
    
    let buffer_pubkey: Pubkey = Pubkey::from_str(BUFFER_ADDRESS).unwrap();
    println!("Buffer Address: {}", buffer_pubkey);

    let balance = solana_client.get_balance(&maintain_authority_pubkey).unwrap();
    println!("Payer Balance <before>: {}", balance);

    let create_maintenance_instruction: Instruction =
        create_maintenance(
            &maintenance_pubkey,
            &maintained_addin_pubkey,
            &maintain_authority_pubkey,
            &maintain_authority_pubkey,
        );
    
    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                create_maintenance_instruction,
            ],
            Some(&maintain_authority_pubkey),
            &[
                &maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Create Maintenance: \n{:?}", result);

    let balance = solana_client.get_balance(&maintain_authority_pubkey).unwrap();
    println!("Payer Balance <after>: {}", balance);

    let (maintenance_record_pubkey,_) = get_maintenance_record_address(&maintenance_pubkey, &maintained_addin_pubkey);
    println!("Maintenance Record Address: {}", maintenance_record_pubkey);

    let mut dt: &[u8] = &solana_client.get_account_data(&maintenance_record_pubkey).unwrap();
    let maintenance_record = MaintenanceRecord::deserialize(&mut dt).unwrap();
    println!("{:?}", maintenance_record);

    let set_upgrade_authority_instruction = set_upgrade_authority(
        &maintained_addin_pubkey,
        &maintained_addin_initial_authority_pubkey,
        Some(&maintenance_record_pubkey),
    );

    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                set_upgrade_authority_instruction,
            ],
            Some(&maintain_authority_pubkey),
            &[
                &maintained_addin_initial_authority_keypair,
                &maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Set Upgrade Authority to Maintenance Record: \n{:?}", result);


    let set_delegate_instruction: Instruction =
        set_delegate(
            &maintenance_pubkey,
            &maintained_addin_pubkey,
            vec![new_maintain_authority_pubkey.clone()],
            &maintain_authority_pubkey,
        );
    
    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                set_delegate_instruction,
            ],
            Some(&maintain_authority_pubkey),
            &[
                &maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Set Delegate: \n{:?}", result);

    let mut dt: &[u8] = &solana_client.get_account_data(&maintenance_record_pubkey).unwrap();
    let maintenance_record = MaintenanceRecord::deserialize(&mut dt).unwrap();
    println!("{:?}", maintenance_record);

    let buffer_data_offset = UpgradeableLoaderState::buffer_data_offset().unwrap();
    // let program_len = UpgradeableLoaderState::program_len().unwrap();
    // let programdata_data_offset = UpgradeableLoaderState::programdata_data_offset().unwrap();
    // println!("Buffer: Data offset = {}; Program len = {}; ProgramData data offset = {}", buffer_data_offset, program_len, programdata_data_offset);
    let program_buffer = &solana_client.get_account_data(&buffer_pubkey).unwrap();
    let (_,program_buffer_data): (&[u8], &[u8]) = program_buffer.split_at(buffer_data_offset);
    let buffer_hash: Hash = hash(&program_buffer_data);
    println!("Buffer Hash: {:?}", buffer_hash);

    let set_code_hashes_instruction: Instruction =
        set_code_hashes(
            &maintenance_pubkey,
            &maintained_addin_pubkey,
            vec![buffer_hash],
            &maintain_authority_pubkey,
        );
    
    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                set_code_hashes_instruction,
            ],
            Some(&maintain_authority_pubkey),
            &[
                &maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Set Code Hashes: \n{:?}", result);

    let mut dt: &[u8] = &solana_client.get_account_data(&maintenance_record_pubkey).unwrap();
    let maintenance_record = MaintenanceRecord::deserialize(&mut dt).unwrap();
    println!("{:?}", maintenance_record);


    let set_buffer_authority_instruction = set_buffer_authority(
        &buffer_pubkey,
        &maintained_addin_initial_authority_pubkey,
        &maintenance_record_pubkey,
    );

    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                set_buffer_authority_instruction,
            ],
            Some(&maintain_authority_pubkey),
            &[
                &maintained_addin_initial_authority_keypair,
                &maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Set Upgrade Buffer Authority to Maintenance Record: \n{:?}", result);

    let balance = solana_client.get_balance(&maintain_authority_pubkey).unwrap();
    println!("Spill Balance <before upgrade>: {}", balance);

    let upgrade_instruction: Instruction =
        upgrade(
            &maintenance_pubkey,
            &maintained_addin_pubkey,
            &maintain_authority_pubkey,
            &buffer_pubkey,
            &maintain_authority_pubkey,
        );
    
    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                upgrade_instruction,
            ],
            Some(&maintain_authority_pubkey),
            &[
                &maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Upgrade \n{:?}", result);

    let balance = solana_client.get_balance(&maintain_authority_pubkey).unwrap();
    println!("Spill Balance <after upgrade>: {}", balance);

    // let close_maintenance_instruction: Instruction =
    //     close_maintenance(
    //         &maintenance_pubkey,
    //         &maintained_addin_pubkey,
    //         &maintain_authority_pubkey,
    //     );
    
    // let transaction: Transaction =
    //     Transaction::new_signed_with_payer(
    //         &[
    //             close_maintenance_instruction,
    //         ],
    //         Some(&maintain_authority_pubkey),
    //         &[
    //             &maintain_authority_keypair,
    //         ],
    //         solana_client.get_latest_blockhash().unwrap(),
    //     );
    
    // let result = solana_client.send_and_confirm_transaction(&transaction);
    // println!("Close Maintenance (Must be error): \n{:?}", result);

    let set_authority_instruction: Instruction =
        set_authority(
            &maintenance_pubkey,
            &maintained_addin_pubkey,
            &maintain_authority_pubkey,
            &new_maintain_authority_pubkey,
            // &maintain_authority_pubkey,
        );
    
    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                set_authority_instruction,
            ],
            Some(&maintain_authority_pubkey),
            &[
                &maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Set Authority from Maintenance Record: \n{:?}", result);

    let mut dt: &[u8] = &solana_client.get_account_data(&maintenance_record_pubkey).unwrap();
    let maintenance_record = MaintenanceRecord::deserialize(&mut dt).unwrap();
    println!("{:?}", maintenance_record);

    let balance = solana_client.get_balance(&maintain_authority_pubkey).unwrap();
    println!("Spill Balance <before close>: {}", balance);

    let close_maintenance_instruction: Instruction =
        close_maintenance(
            &maintenance_pubkey,
            &maintained_addin_pubkey,
            &maintain_authority_pubkey,
            &maintain_authority_pubkey,
        );
    
    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                close_maintenance_instruction,
            ],
            Some(&maintain_authority_pubkey),
            &[
                &maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Close Maintenance: \n{:?}", result);

    let balance = solana_client.get_balance(&maintain_authority_pubkey).unwrap();
    println!("Spill Balance <after close>: {}", balance);

    let set_upgrade_authority_instruction = set_upgrade_authority(
        &maintained_addin_pubkey,
        &new_maintain_authority_pubkey,
        Some(&maintained_addin_initial_authority_pubkey),
    );

    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &[
                set_upgrade_authority_instruction,
            ],
            Some(&new_maintain_authority_pubkey),
            &[
                &new_maintain_authority_keypair,
            ],
            solana_client.get_latest_blockhash().unwrap(),
        );
    
    let result = solana_client.send_and_confirm_transaction(&transaction);
    println!("Set Upgrade Authority to Initial: \n{:?}", result);
}
