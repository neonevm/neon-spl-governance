#![cfg(feature = "test-bpf")]
use std::str::FromStr;

use solana_program::{
    borsh::try_from_slice_unchecked,
    hash::Hash,
    pubkey::Pubkey,
    rent::Rent,
};
use solana_program_test::{processor, ProgramTest, ProgramTestBanksClientExt};
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::{Transaction, TransactionError},
    program_error::ProgramError,
};
use spl_governance_addin_vesting::{
    entrypoint::process_instruction,
    error::VestingError,
    state::{VestingSchedule, VestingRecord},
    voter_weight::{ExtendedVoterWeightRecord, get_voter_weight_record_address},
    max_voter_weight::{MaxVoterWeightRecord, get_max_voter_weight_record_address},
    instruction as vesting_instruction,
};
use spl_token::{self, instruction as token_instruction, state::Account as TokenAccount};
use spl_governance::{
    instruction as governance_instruction,
    state::{
        enums::MintMaxVoteWeightSource,
        realm::get_realm_address,
    },
};

fn trx_instruction_error<T>(index: u8, error: T) -> TransactionError
where T: Into<ProgramError>
{
    let program_error: ProgramError = error.into();
    TransactionError::InstructionError(index, u64::from(program_error).into())
}

#[tokio::test]
async fn test_token_vesting_without_realm() {

    // Create program and test environment
    let program_id = Pubkey::from_str("VestingbGKPFXCWuBvfkegQfZyiNwAJb9Ss623VQ5DA").unwrap();
    let mint_authority = Keypair::new();
    let mint = Keypair::new();
    let spill = Keypair::new();

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

    for (account, owner) in [
        (&source_token_account, &source_account.pubkey()),
        (&vesting_token_account, &vesting_account_key),
        (&destination_token_account, &destination_account.pubkey()),
        (&new_destination_token_account, &new_destination_account.pubkey()),
    ] {
        banks_client.process_transaction(
            create_token_account(&payer, &mint, recent_blockhash, account, owner)
        ).await.unwrap()
    }

    // Create and process the vesting transactions
    let setup_instructions = [
        token_instruction::mint_to(
            &spl_token::id(), 
            &mint.pubkey(), 
            &source_token_account.pubkey(), 
            &mint_authority.pubkey(), 
            &[], 
            101,
        ).unwrap(),
        token_instruction::transfer(
            &spl_token::id(),
            &source_token_account.pubkey(),
            &vesting_token_account.pubkey(),
            &source_account.pubkey(),
            &[],
            1,
        ).unwrap(),
    ];
    
    // Process transaction on test network
    let mut setup_transaction = Transaction::new_with_payer(
        &setup_instructions,
        Some(&payer.pubkey()),
    );
    setup_transaction.partial_sign(&[&payer, &mint_authority, &source_account], recent_blockhash);
    banks_client.process_transaction(setup_transaction).await.unwrap();

    let schedules = vec![
        VestingSchedule {amount: 20, release_time:  0},
        VestingSchedule {amount: 20, release_time: 10},
        VestingSchedule {amount: 20, release_time: 20},
        VestingSchedule {amount: 20, release_time: 30},
        VestingSchedule {amount: 20, release_time: 40},
    ];

    let deposit_instructions = [
        vesting_instruction::deposit(
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
        vesting_instruction::change_owner(
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


    let mut close_transaction = Transaction::new_with_payer(
        &[
            vesting_instruction::close(
                &program_id,
                &spl_token::id(),
                &vesting_token_account.pubkey(),
                &destination_account.pubkey(),
                &spill.pubkey(),
            ).unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    close_transaction.partial_sign(&[&payer, &destination_account], recent_blockhash);
    assert_eq!(
        banks_client.process_transaction(close_transaction).await.unwrap_err().unwrap(),
        trx_instruction_error(0, VestingError::InvalidOwnerForVestingAccount)
    );


    let mut close_transaction = Transaction::new_with_payer(
        &[
            vesting_instruction::close(
                &program_id,
                &spl_token::id(),
                &vesting_token_account.pubkey(),
                &new_destination_account.pubkey(),
                &spill.pubkey(),
            ).unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    close_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
    assert_eq!(
        banks_client.process_transaction(close_transaction).await.unwrap_err().unwrap(),
        trx_instruction_error(0, VestingError::VestingNotEmpty)
    );


    // Checks that we can split some amount of vesting into separate accounts
    for (expected_result, splitted_schedule) in [
    //            0  10  20  30  40                 (Amount, Time)
        (Err(VestingError::InvalidSchedule),   vec![( 1, 10), ( 1,  0)]),
        (Err(VestingError::InvalidSchedule),   vec![( 1, 10), ( 1,  20), ( 1, 19)]),
        (Err(VestingError::InsufficientFunds), vec![(21,  0)]),
        (Ok(vec![20, 19, 20, 19, 20]),         vec![( 1, 13), ( 1, 39)]),
        (Ok(vec![20, 10,  0, 19, 20]),         vec![(29, 25)]),
        (Ok(vec![19,  8,  0, 18, 17]),         vec![( 1,  1), ( 1, 11), ( 1, 21), ( 1, 31), ( 1, 41), ( 1, 51), ( 1, 61)]),
        (Err(VestingError::InsufficientFunds), vec![(28, 29)]),
        (Ok(vec![19,  8,  0,  7,  0]),         vec![( 1, 30), (27, 1000)]),
        (Err(VestingError::InsufficientFunds), vec![(16+19, 30)]),
        (Ok(vec![18,  0,  0,  0,  0]),         vec![(16, 30)]),
    ] {
        println!("Splitted schedule: {:?}", splitted_schedule);
        let splitted_schedule : Vec<VestingSchedule> = splitted_schedule.into_iter().map(|(amount,release_time)| VestingSchedule {amount, release_time}).collect();
        fn make_schedule_from_amount(amounts:&Vec<u64>) -> Vec<VestingSchedule> {
            amounts.iter().enumerate().map(
                |(index, amount)| VestingSchedule {
                    amount: *amount,
                    release_time: index as u64 * 10
                }
            ).collect()
        }

        let splitted_vesting_owner = Keypair::new();
        let splitted_vesting_token_account = Keypair::new();
        let (splitted_vesting_account_key,_) = Pubkey::find_program_address(&[&splitted_vesting_token_account.pubkey().as_ref()], &program_id);

        banks_client.process_transaction(
            create_token_account(&payer, &mint, recent_blockhash, &splitted_vesting_token_account, &splitted_vesting_account_key)
        ).await.unwrap();

        let mut split_transaction = Transaction::new_with_payer(
            &[
                vesting_instruction::split(
                    &program_id,
                    &spl_token::id(),
                    &vesting_token_account.pubkey(),
                    &new_destination_account.pubkey(),
                    &splitted_vesting_token_account.pubkey(),
                    &splitted_vesting_owner.pubkey(),
                    &payer.pubkey(),
                    splitted_schedule.clone(),
                ).unwrap(),
            ],
            Some(&payer.pubkey()),
        );
        split_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
        let result = banks_client.process_transaction(split_transaction).await;
        let acc = banks_client.get_account(vesting_account_key).await.unwrap();
        let splitted_acc = banks_client.get_account(splitted_vesting_account_key).await.unwrap();
        let splitted_token = banks_client.get_packed_account_data::<TokenAccount>(splitted_vesting_token_account.pubkey()).await.unwrap();

        match expected_result {
            Ok(expected_amounts) => {
                result.unwrap();
                assert_eq!(
                    try_from_slice_unchecked::<VestingRecord>(&acc.as_ref().unwrap().data).unwrap().schedule,
                    make_schedule_from_amount(&expected_amounts)
                );
                let splitted_record = try_from_slice_unchecked::<VestingRecord>(&splitted_acc.as_ref().unwrap().data).unwrap();
                assert_eq!(splitted_record.owner, splitted_vesting_owner.pubkey());
                assert_eq!(splitted_record.realm, None);
                assert_eq!(splitted_record.token, splitted_vesting_token_account.pubkey());
                assert_eq!(splitted_record.schedule, splitted_schedule);
                assert_eq!(splitted_token.amount, splitted_schedule.iter().map(|v| v.amount).sum::<u64>());
                assert_eq!(splitted_token.owner, splitted_vesting_account_key);
                assert_eq!(splitted_token.mint, mint.pubkey());
            }
            Err(err) => {
                assert_eq!(result.unwrap_err().unwrap(), trx_instruction_error(0, err));
                assert_eq!(splitted_acc.is_none(), true);
                assert_eq!(splitted_token.amount, 0);
            }
        }
    }


    let recent_blockhash = banks_client.get_new_latest_blockhash(&recent_blockhash).await.unwrap();

    let withdraw_instrictions = [
        vesting_instruction::withdraw(
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

    let destination_token_data = banks_client.get_packed_account_data::<TokenAccount>(destination_token_account.pubkey()).await.unwrap();
    assert_eq!(destination_token_data.amount, 18);

    let vesting_token_data = banks_client.get_packed_account_data::<TokenAccount>(vesting_token_account.pubkey()).await.unwrap();
    assert_eq!(vesting_token_data.amount, 1);   // contains only 1 token transferred on start

    let acc = banks_client.get_account(vesting_account_key).await.unwrap();
    println!("Vesting: {:?}", acc);
    println!("    {:?}", try_from_slice_unchecked::<VestingRecord>(&acc.as_ref().unwrap().data).unwrap());


    let mut close_transaction = Transaction::new_with_payer(
        &[
            vesting_instruction::close(
                &program_id,
                &spl_token::id(),
                &vesting_token_account.pubkey(),
                &new_destination_account.pubkey(),
                &spill.pubkey(),
            ).unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    close_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
    banks_client.process_transaction(close_transaction).await.unwrap();

    assert_eq!(banks_client.get_account(vesting_account_key).await.unwrap(), None);

    let vesting_token_data = banks_client.get_packed_account_data::<TokenAccount>(vesting_token_account.pubkey()).await.unwrap();
    assert_eq!(vesting_token_data.owner, new_destination_account.pubkey());
    assert_eq!(vesting_token_data.amount, 1);
}

#[tokio::test]
async fn test_token_vesting_with_realm() {

    // Create program and test environment
    let program_id = Pubkey::from_str("VestingbGKPFXCWuBvfkegQfZyiNwAJb9Ss623VQ5DA").unwrap();
    let governance_id = Pubkey::from_str("5ZYgDTqLbYJ2UAtF7rbUboSt9Q6bunCQgGEwxDFrQrXb").unwrap();
    let mint_authority = Keypair::new();
    let mint = Keypair::new();
    let spill = Keypair::new();

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
        token_instruction::mint_to(
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
        governance_instruction::create_realm(
            &governance_id,
            &mint_authority.pubkey(),
            &mint.pubkey(),
            &payer.pubkey(),
            None, None, None,
            realm_name,
            1,
            MintMaxVoteWeightSource::SupplyFraction(10_000_000_000)
        ),
        governance_instruction::create_token_owner_record(
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
        vesting_instruction::deposit_with_realm(
            &program_id,
            &spl_token::id(),
            &vesting_token_account.pubkey(),
            &source_account.pubkey(),
            &source_token_account.pubkey(),
            &destination_account.pubkey(),
            &payer.pubkey(),
            schedules.clone(),
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
        governance_instruction::set_governance_delegate(
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
        vesting_instruction::set_vote_percentage_with_realm(
            &program_id,
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
        vesting_instruction::create_voter_weight_record(
            &program_id,
            &new_destination_account.pubkey(),
            &payer.pubkey(),
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
        vesting_instruction::change_owner_with_realm(
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


    let max_voter_weight_record_address = get_max_voter_weight_record_address(
        &program_id,
        &realm_address,
        &mint.pubkey(),
    );
    let voter_weight_record_address1 = get_voter_weight_record_address(
        &program_id,
        &realm_address,
        &mint.pubkey(),
        &destination_account.pubkey()
    );
    let voter_weight_record_address2 = get_voter_weight_record_address(
        &program_id,
        &realm_address,
        &mint.pubkey(),
        &new_destination_account.pubkey()
    );
    let voter_weight_record1 = banks_client.get_account_data_with_borsh::<ExtendedVoterWeightRecord>(voter_weight_record_address1).await.unwrap();
    println!("VoterWeightRecord1 before withdraw: {:?}", voter_weight_record1);
    let voter_weight_record2 = banks_client.get_account_data_with_borsh::<ExtendedVoterWeightRecord>(voter_weight_record_address2).await.unwrap();
    println!("VoterWeightRecord2 before withdraw: {:?}", voter_weight_record2);


    let mut close_record1_transaction = Transaction::new_with_payer(
        &[
            vesting_instruction::close_voter_weight_record(
                &program_id,
                &destination_account.pubkey(),
                &realm_address,
                &mint.pubkey(),
                &spill.pubkey(),
            ).unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    close_record1_transaction.partial_sign(&[&payer, &destination_account], recent_blockhash);
    banks_client.process_transaction(close_record1_transaction).await.unwrap();
    assert_eq!(banks_client.get_account(voter_weight_record_address1).await.unwrap(), None);
    
    let mut close_record2_transaction = Transaction::new_with_payer(
        &[
            vesting_instruction::close_voter_weight_record(
                &program_id,
                &new_destination_account.pubkey(),
                &realm_address,
                &mint.pubkey(),
                &spill.pubkey(),
            ).unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    close_record2_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
    assert_eq!(
        banks_client.process_transaction(close_record2_transaction).await.unwrap_err().unwrap(),
        trx_instruction_error(0, VestingError::VoterWeightRecordNotEmpty)
    );


    {
        let splitted_schedule = vec![VestingSchedule {amount: 28, release_time: 3}];
        let splitted_vesting_owner = Keypair::new();
        let splitted_vesting_token_account = Keypair::new();
        let (splitted_vesting_account_key,_) = Pubkey::find_program_address(&[&splitted_vesting_token_account.pubkey().as_ref()], &program_id);
        let splitted_voter_weight_record_address = get_voter_weight_record_address(
            &program_id,
            &realm_address,
            &mint.pubkey(),
            &splitted_vesting_owner.pubkey()
        );
    

        banks_client.process_transaction(
            create_token_account(&payer, &mint, recent_blockhash, &splitted_vesting_token_account, &splitted_vesting_account_key)
        ).await.unwrap();

        let mut split_transaction = Transaction::new_with_payer(
            &[
                vesting_instruction::split_with_realm(
                    &program_id,
                    &spl_token::id(),
                    &vesting_token_account.pubkey(),
                    &new_destination_account.pubkey(),
                    &splitted_vesting_token_account.pubkey(),
                    &splitted_vesting_owner.pubkey(),
                    &payer.pubkey(),
                    splitted_schedule.clone(),
                    &governance_id,
                    &realm_address,
                    &mint.pubkey(),
                ).unwrap(),
            ],
            Some(&payer.pubkey()),
        );
        split_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
        banks_client.process_transaction(split_transaction).await.unwrap();
        let acc = banks_client.get_account(vesting_account_key).await.unwrap();
        assert_eq!(
            try_from_slice_unchecked::<VestingRecord>(&acc.as_ref().unwrap().data).unwrap().schedule,
            vec![
                VestingSchedule {amount: 12, release_time: 0},
                VestingSchedule {amount:  0, release_time: 2},
                VestingSchedule {amount: 20, release_time: 5}
            ]
        );
        let splitted_acc = banks_client.get_account(splitted_vesting_account_key).await.unwrap();
        let splitted_record = try_from_slice_unchecked::<VestingRecord>(&splitted_acc.as_ref().unwrap().data).unwrap();
        assert_eq!(splitted_record.owner, splitted_vesting_owner.pubkey());
        assert_eq!(splitted_record.realm, Some(realm_address));
        assert_eq!(splitted_record.token, splitted_vesting_token_account.pubkey());
        assert_eq!(splitted_record.schedule, splitted_schedule);

        let splitted_token = banks_client.get_packed_account_data::<TokenAccount>(splitted_vesting_token_account.pubkey()).await.unwrap();
        assert_eq!(splitted_token.amount, splitted_schedule.iter().map(|v| v.amount).sum::<u64>());
        assert_eq!(splitted_token.owner, splitted_vesting_account_key);
        assert_eq!(splitted_token.mint, mint.pubkey());

        let splitted_voter_weight_record = banks_client.get_account_data_with_borsh::<ExtendedVoterWeightRecord>(splitted_voter_weight_record_address).await.unwrap();
        assert_eq!(splitted_voter_weight_record.total_amount, 28);
    }


    let withdraw_instrictions = [
        vesting_instruction::withdraw_with_realm(
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


    let vesting_token_data = banks_client.get_packed_account_data::<TokenAccount>(vesting_token_account.pubkey()).await.unwrap();
    assert_eq!(vesting_token_data.amount, 0);

    let destination_token_data = banks_client.get_packed_account_data::<TokenAccount>(destination_token_account.pubkey()).await.unwrap();
    assert_eq!(destination_token_data.amount, 32);


    let vesting_record = banks_client.get_account_data_with_borsh::<VestingRecord>(vesting_account_key).await.unwrap();
    println!("VestingRecord: {:?}", vesting_record);
    assert_eq!(vesting_record.schedule.iter().map(|v| v.amount).sum::<u64>(), 0u64);

    let voter_weight_record2 = banks_client.get_account_data_with_borsh::<ExtendedVoterWeightRecord>(voter_weight_record_address2).await.unwrap();
    println!("VoterWeightRecord: {:?}", voter_weight_record2);
    assert_eq!(voter_weight_record2.total_amount, 0);

    let max_voter_weight_record = banks_client.get_account_data_with_borsh::<MaxVoterWeightRecord>(max_voter_weight_record_address).await.unwrap();
    println!("MaxVoterWeightRecord: {:?}", max_voter_weight_record);
    assert_eq!(max_voter_weight_record.max_voter_weight, 28);


    let recent_blockhash = banks_client.get_new_latest_blockhash(&recent_blockhash).await.unwrap();


    let mut close_record2_transaction = Transaction::new_with_payer(
        &[
            vesting_instruction::close_voter_weight_record(
                &program_id,
                &new_destination_account.pubkey(),
                &realm_address,
                &mint.pubkey(),
                &spill.pubkey(),
            ).unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    close_record2_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
    banks_client.process_transaction(close_record2_transaction).await.unwrap();
    assert_eq!(banks_client.get_account(voter_weight_record_address2).await.unwrap(), None);


    let mut close_transaction = Transaction::new_with_payer(
        &[
            vesting_instruction::close(
                &program_id,
                &spl_token::id(),
                &vesting_token_account.pubkey(),
                &new_destination_account.pubkey(),
                &spill.pubkey(),
            ).unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    close_transaction.partial_sign(&[&payer, &new_destination_account], recent_blockhash);
    banks_client.process_transaction(close_transaction).await.unwrap();
    assert_eq!(banks_client.get_account(vesting_token_account.pubkey()).await.unwrap(), None);
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
        token_instruction::initialize_mint(
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
        token_instruction::initialize_account(
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
