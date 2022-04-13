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
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_governance_addin_vesting::{
    entrypoint::process_instruction,
    state::{VestingSchedule, VestingRecord},
    voter_weight::{ExtendedVoterWeightRecord, get_voter_weight_record_address},
    max_voter_weight::{MaxVoterWeightRecord, get_max_voter_weight_record_address},
    instruction::{
        deposit,
        withdraw,
        change_owner,

        deposit_with_realm,
        withdraw_with_realm,
        change_owner_with_realm,
        create_voter_weight_record,
        set_vote_percentage_with_realm,
    },
};
use spl_token::{self, instruction::{initialize_mint, initialize_account, mint_to}};
use spl_governance::{
    instruction::{
        create_realm,
        create_token_owner_record,
        set_governance_delegate,
    },
    state::{
        enums::MintMaxVoteWeightSource,
        realm::get_realm_address,
    },
};

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

    let vesting_token_account = Keypair::new();
    let (vesting_account_key,_) = Pubkey::find_program_address(&[&vesting_token_account.pubkey().as_ref()], &program_id);
    
    let mut program_test = ProgramTest::new(
        "spl_governance_addin_vesting",
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
            &vesting_token_account.pubkey(),
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

#[tokio::test]
async fn test_token_vesting_with_realm() {

    // Create program and test environment
    let program_id = Pubkey::from_str("VestingbGKPFXCWuBvfkegQfZyiNwAJb9Ss623VQ5DA").unwrap();
    let governance_id = Pubkey::from_str("5ZYgDTqLbYJ2UAtF7rbUboSt9Q6bunCQgGEwxDFrQrXb").unwrap();
    let mint_authority = Keypair::new();
    let mint = Keypair::new();

    let source_account = Keypair::new();
    let source_token_account = Keypair::new();

    let destination_account = Keypair::new();
    let destination_token_account = Keypair::new();
    let destination_delegate = Keypair::new();

    let new_destination_account = Keypair::new();
    let new_destination_token_account = Keypair::new();

    let vesting_token_account = Keypair::new();
    let (vesting_account_key,_) = Pubkey::find_program_address(&[&vesting_token_account.pubkey().as_ref()], &program_id);

    let mut program_test = ProgramTest::new(
        "spl_governance_addin_vesting",
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

    program_test.add_program(
        "spl_governance",
        governance_id,
        None,
    );

    // Start and process transactions on the test network
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

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
            120
        ).unwrap()
    ];
    
    // Process transaction on test network
    let mut setup_transaction = Transaction::new_with_payer(
        &setup_instructions,
        Some(&payer.pubkey()),
    );
    setup_transaction.partial_sign(&[&payer, &mint_authority], recent_blockhash);
    banks_client.process_transaction(setup_transaction).await.unwrap();

    // Create realm
    let realm_name = "testing realm".to_string();
    let realm_address = get_realm_address(&governance_id, &realm_name);
    let create_realm_instructions = [
        create_realm(
            &governance_id,
            &mint_authority.pubkey(),
            &mint.pubkey(),
            &payer.pubkey(),
            None, None, None,
            realm_name,
            1,
            MintMaxVoteWeightSource::SupplyFraction(10_000_000_000)
        ),
        create_token_owner_record(
            &governance_id,
            &realm_address,
            &destination_account.pubkey(),
            &mint.pubkey(),
            &payer.pubkey(),
        ),
    ];
    let mut create_realm_transaction = Transaction::new_with_payer(
        &create_realm_instructions,
        Some(&payer.pubkey()),
    );
    create_realm_transaction.partial_sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(create_realm_transaction).await.unwrap();

    // Create vesting account
    let schedules = vec![
        VestingSchedule {amount: 20, release_time: 0},
        VestingSchedule {amount: 20, release_time: 2},
        VestingSchedule {amount: 20, release_time: 5}
    ];

    let deposit_instructions = [
        deposit_with_realm(
            &program_id,
            &spl_token::id(),
            &vesting_token_account.pubkey(),
            &source_account.pubkey(),
            &source_token_account.pubkey(),
            &destination_account.pubkey(),
            &payer.pubkey(),
            schedules.clone(),
            &governance_id,
            &realm_address,
            &mint.pubkey(),
        ).unwrap(),
    ];
    let mut deposit_transaction = Transaction::new_with_payer(
        &deposit_instructions,
        Some(&payer.pubkey()),
    );
    deposit_transaction.partial_sign(&[&payer, &source_account], recent_blockhash);
    banks_client.process_transaction(deposit_transaction).await.unwrap();


    let set_governance_delegate_instructions = [
        set_governance_delegate(
            &governance_id,
            &destination_account.pubkey(),
            &realm_address,
            &mint.pubkey(),
            &destination_account.pubkey(),
            &Some(destination_delegate.pubkey()),
        ),
    ];
    let mut set_governance_delegate_transaction = Transaction::new_with_payer(
        &set_governance_delegate_instructions,
        Some(&payer.pubkey()),
    );
    set_governance_delegate_transaction.partial_sign(&[&payer, &destination_account], recent_blockhash);
    banks_client.process_transaction(set_governance_delegate_transaction).await.unwrap();


    let set_vote_percentage_instructions = [
        set_vote_percentage_with_realm(
            &program_id,
            &vesting_token_account.pubkey(),
            &destination_account.pubkey(),
            &destination_delegate.pubkey(),
            &governance_id,
            &realm_address,
            &mint.pubkey(),
            30*100,
        ).unwrap(),
    ];
    let mut set_vote_percentage_transaction = Transaction::new_with_payer(
        &set_vote_percentage_instructions,
        Some(&payer.pubkey()),
    );
    set_vote_percentage_transaction.partial_sign(&[&payer, &destination_delegate], recent_blockhash);
    banks_client.process_transaction(set_vote_percentage_transaction).await.unwrap();


    let init_new_destination_instructions = [
        create_voter_weight_record(
            &program_id,
            &new_destination_account.pubkey(),
            &payer.pubkey(),
            &governance_id,
            &realm_address,
            &mint.pubkey(),
        ).unwrap(),
    ];
    let mut init_new_destination_transaction = Transaction::new_with_payer(
        &init_new_destination_instructions,
        Some(&payer.pubkey()),
    );
    init_new_destination_transaction.partial_sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(init_new_destination_transaction).await.unwrap();


    let change_owner_instructions = [
        change_owner_with_realm(
            &program_id,
            &vesting_token_account.pubkey(),
            &destination_account.pubkey(),
            &new_destination_account.pubkey(),
            &governance_id,
            &realm_address,
            &mint.pubkey(),
        ).unwrap(),
    ];
    let mut change_owner_transaction = Transaction::new_with_payer(
        &change_owner_instructions,
        Some(&payer.pubkey()),
    );
    change_owner_transaction.partial_sign(&[&payer, &destination_account], recent_blockhash);
    banks_client.process_transaction(change_owner_transaction).await.unwrap();


    let voter_weight_record_address2 = get_voter_weight_record_address(
        &program_id,
        &realm_address,
        &mint.pubkey(),
        &new_destination_account.pubkey()
    );
    let voter_weight_record_account2 = banks_client.get_account(voter_weight_record_address2).await.unwrap().unwrap();
    let voter_weight_record2: ExtendedVoterWeightRecord = try_from_slice_unchecked(&voter_weight_record_account2.data).unwrap();
    println!("VoterWeightRecord before withdraw: {:?}", voter_weight_record2);
    

    let withdraw_instrictions = [
        withdraw_with_realm(
            &program_id,
            &spl_token::id(),
            &vesting_token_account.pubkey(),
            &destination_token_account.pubkey(),
            &new_destination_account.pubkey(),
            &governance_id,
            &realm_address,
            &mint.pubkey(),
        ).unwrap(),
    ];

    let mut withdraw_transaction = Transaction::new_with_payer(
        &withdraw_instrictions,
        Some(&payer.pubkey()),
    );
    withdraw_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
    banks_client.process_transaction(withdraw_transaction).await.unwrap();


    let vesting_record_account = banks_client.get_account(vesting_account_key).await.unwrap().unwrap();
    let vesting_record: VestingRecord = try_from_slice_unchecked(&vesting_record_account.data).unwrap();
    println!("VestingRecord: {:?}", vesting_record);

    let voter_weight_record_address = get_voter_weight_record_address(
        &program_id,
        &realm_address,
        &mint.pubkey(),
        &destination_account.pubkey()
    );
    let voter_weight_record_account = banks_client.get_account(voter_weight_record_address).await.unwrap().unwrap();
    let voter_weight_record: ExtendedVoterWeightRecord = try_from_slice_unchecked(&voter_weight_record_account.data).unwrap();
    println!("VoterWeightRecord: {:?}", voter_weight_record);

    let voter_weight_record_address2 = get_voter_weight_record_address(
        &program_id,
        &realm_address,
        &mint.pubkey(),
        &new_destination_account.pubkey()
    );
    let voter_weight_record_account2 = banks_client.get_account(voter_weight_record_address2).await.unwrap().unwrap();
    let voter_weight_record2: ExtendedVoterWeightRecord = try_from_slice_unchecked(&voter_weight_record_account2.data).unwrap();
    println!("VoterWeightRecord: {:?}", voter_weight_record2);

    let max_voter_weight_record_address = get_max_voter_weight_record_address(
        &program_id,
        &realm_address,
        &mint.pubkey(),
    );
    let max_voter_weight_record_account = banks_client.get_account(max_voter_weight_record_address).await.unwrap().unwrap();
    let max_voter_weight_record: MaxVoterWeightRecord = try_from_slice_unchecked(&max_voter_weight_record_account.data).unwrap();
    println!("MaxVoterWeightRecord: {:?}", max_voter_weight_record);
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
