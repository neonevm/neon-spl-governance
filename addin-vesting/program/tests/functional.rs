#![cfg(feature = "test-bpf")]
use std::str::FromStr;

use solana_program::{
    borsh::try_from_slice_unchecked,
    hash::Hash,
    pubkey::Pubkey,
    rent::Rent,
};
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    account::Account,
    signature::Keypair,
    signature::Signer,
    system_instruction,
    transaction::Transaction,
};
use token_vesting::{entrypoint::process_instruction, state::VestingSchedule, state::VestingRecord};
use token_vesting::instruction::{deposit, withdraw, change_owner};
use spl_token::{self, instruction::{initialize_mint, initialize_account, mint_to}};

#[tokio::test]
async fn test_token_vesting() {

    // Create program and test environment
    let program_id = Pubkey::from_str("VestingbGKPFXCWuBvfkegQfZyiNwAJb9Ss623VQ5DA").unwrap();
    let mint_authority = Keypair::new();
    let mint = Keypair::new();

    let source_account = Keypair::new();
    let source_token_account = Keypair::new();

    let destination_account = Keypair::new();
    let destination_token_account = Keypair::new();

    let new_destination_account = Keypair::new();
    let new_destination_token_account = Keypair::new();

    let mut seeds = [42u8; 32];
    let (vesting_account_key, bump) = Pubkey::find_program_address(&[&seeds[..31]], &program_id);
    seeds[31] = bump;
    let vesting_token_account = Keypair::new();
    
    let mut program_test = ProgramTest::new(
        "token_vesting",
        program_id,
        processor!(process_instruction),
    );

    // Add accounts         
    program_test.add_account(
        source_account.pubkey(),
        Account {
            lamports: 5000000,
            ..Account::default()
        },
    );

    // Start and process transactions on the test network
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    //let mut context = program_test.start_with_context().await;
    //let payer = &context.payer;
    //let recent_blockhash = context.last_blockhash;

    // Initialize the token accounts
    banks_client.process_transaction(mint_init_transaction(
        &payer,
        &mint,
        &mint_authority,
        recent_blockhash
    )).await.unwrap();

    banks_client.process_transaction(
        create_token_account(&payer, &mint, recent_blockhash, &source_token_account, &source_account.pubkey())
    ).await.unwrap();
    banks_client.process_transaction(
        create_token_account(&payer, &mint, recent_blockhash, &vesting_token_account, &vesting_account_key)
    ).await.unwrap();
    banks_client.process_transaction(
        create_token_account(&payer, &mint, recent_blockhash, &destination_token_account, &destination_account.pubkey())
    ).await.unwrap();
    banks_client.process_transaction(
        create_token_account(&payer, &mint, recent_blockhash, &new_destination_token_account, &new_destination_account.pubkey())
    ).await.unwrap();


    // Create and process the vesting transactions
    let setup_instructions = [
        mint_to(
            &spl_token::id(), 
            &mint.pubkey(), 
            &source_token_account.pubkey(), 
            &mint_authority.pubkey(), 
            &[], 
            60
        ).unwrap()
    ];
    
    // Process transaction on test network
    let mut setup_transaction = Transaction::new_with_payer(
        &setup_instructions,
        Some(&payer.pubkey()),
    );
    setup_transaction.partial_sign(&[&payer, &mint_authority], recent_blockhash);
    banks_client.process_transaction(setup_transaction).await.unwrap();

    let schedules = vec![
        VestingSchedule {amount: 20, release_time: 0},
        VestingSchedule {amount: 20, release_time: 2},
        VestingSchedule {amount: 20, release_time: 5}
    ];

    let deposit_instructions = [
        deposit(
            &program_id,
            &spl_token::id(),
            seeds.clone(),
            &vesting_token_account.pubkey(),
            &source_account.pubkey(),
            &source_token_account.pubkey(),
            &destination_account.pubkey(),
            &payer.pubkey(),
            schedules,
        ).unwrap(),
    ];
    // Process transaction on test network
    let mut deposit_transaction = Transaction::new_with_payer(
        &deposit_instructions,
        Some(&payer.pubkey()),
    );
    deposit_transaction.partial_sign(&[&payer, &source_account], recent_blockhash);
    banks_client.process_transaction(deposit_transaction).await.unwrap();


    let change_owner_instructions = [
        change_owner(
            &program_id,
            seeds.clone(),
            &destination_account.pubkey(),
            &new_destination_account.pubkey(),
        ).unwrap(),
    ];
    let mut change_owner_transaction = Transaction::new_with_payer(
        &change_owner_instructions,
        Some(&payer.pubkey()),
    );
    change_owner_transaction.partial_sign(&[&payer, &destination_account], recent_blockhash);
    banks_client.process_transaction(change_owner_transaction).await.unwrap();


    let withdraw_instrictions = [
        withdraw(
            &program_id,
            &spl_token::id(),
            seeds.clone(),
            &vesting_token_account.pubkey(),
            &destination_token_account.pubkey(),
            &new_destination_account.pubkey(),
        ).unwrap(),
    ];

    let mut withdraw_transaction = Transaction::new_with_payer(
        &withdraw_instrictions,
        Some(&payer.pubkey()),
    );
    withdraw_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
    banks_client.process_transaction(withdraw_transaction).await.unwrap();


    let acc = banks_client.get_account(vesting_account_key).await.unwrap();
    println!("Vesting: {:?}", acc);

    if let Some(a) = acc {
        let acc_record: VestingRecord = try_from_slice_unchecked(&a.data).unwrap();
        println!("         {:?}", acc_record);
    }
}

fn mint_init_transaction(
    payer: &Keypair, 
    mint:&Keypair, 
    mint_authority: &Keypair, 
    recent_blockhash: Hash) -> Transaction{
    let instructions = [
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            Rent::default().minimum_balance(82),
            82,
            &spl_token::id()
    
        ),
        initialize_mint(
            &spl_token::id(), 
            &mint.pubkey(), 
            &mint_authority.pubkey(),
            None, 
            0
        ).unwrap(),
    ];
    let mut transaction = Transaction::new_with_payer(
        &instructions,
        Some(&payer.pubkey()),
    );
    transaction.partial_sign(
        &[
            payer,
            mint
            ], 
        recent_blockhash
    );
    transaction
}

fn create_token_account(
    payer: &Keypair, 
    mint:&Keypair, 
    recent_blockhash: Hash,
    token_account:&Keypair,
    token_account_owner: &Pubkey
) -> Transaction {
    let instructions = [
        system_instruction::create_account(
            &payer.pubkey(),
            &token_account.pubkey(),
            Rent::default().minimum_balance(165),
            165,
            &spl_token::id()
        ),
        initialize_account(
            &spl_token::id(), 
            &token_account.pubkey(), 
            &mint.pubkey(), 
            token_account_owner
        ).unwrap()
   ];
   let mut transaction = Transaction::new_with_payer(
    &instructions,
    Some(&payer.pubkey()),
    );
    transaction.partial_sign(
        &[
            payer,
            token_account
            ], 
        recent_blockhash
    );
    transaction
}
