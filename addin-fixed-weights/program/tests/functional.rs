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
    signer::keypair::read_keypair_file,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::{Transaction, TransactionError},
    program_error::ProgramError,
};
use spl_governance_addin_fixed_weights::{
    entrypoint::process_instruction,
    config::VOTER_LIST,
    error::VoterWeightAddinError,
    instruction::{
        setup_max_voter_weight_record,
        setup_voter_weight_record,
        set_vote_percentage_with_realm,
        get_voter_weight_address,
    },
};
use spl_token::{self, instruction::initialize_mint};
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
    error::GovernanceError,
};
use spl_governance_addin_api::{
    voter_weight::VoterWeightRecord,
};

fn trx_instruction_error<T>(index: u8, error: T) -> TransactionError
where T: Into<ProgramError>
{
    let program_error: ProgramError = error.into();
    TransactionError::InstructionError(index, u64::from(program_error).into())
}

#[tokio::test]
async fn test_fixed_weights() {

    // Create program and test environment
    let program_id = Pubkey::from_str("FixedWeightsXCWuBvfkegQfZyiNwAJb9Ss623VQ5DA").unwrap();
    let governance_id = Pubkey::from_str("5ZYgDTqLbYJ2UAtF7rbUboSt9Q6bunCQgGEwxDFrQrXb").unwrap();
    let mint_authority = Keypair::new();
    let mint = Keypair::new();
    let source_account = Keypair::new();

    let mut program_test = ProgramTest::new(
        "spl_governance_addin_fixed_weights",
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
    banks_client.process_transaction(
        mint_init_transaction(
            &payer,
            &mint,
            &mint_authority,
            recent_blockhash
        )
    ).await.unwrap();

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
        setup_max_voter_weight_record(
            &program_id,
            &realm_address,
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

    for (owner,_) in VOTER_LIST.iter() {
        let mut transaction = Transaction::new_with_payer(
            &[
                setup_voter_weight_record(
                    &program_id,
                    &realm_address,
                    &mint.pubkey(),
                    &owner,
                    &payer.pubkey(),
                ),
                create_token_owner_record(
                    &governance_id,
                    &realm_address,
                    &owner,
                    &mint.pubkey(),
                    &payer.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.partial_sign(&[&payer], recent_blockhash);
        banks_client.process_transaction(transaction).await.unwrap();
    }

    // Check that we can't create record for account missed in VOTER_LIST
    {
        let not_owner = {
            let mut keypair = Keypair::new();
            while VOTER_LIST.iter().any(|(o,_)| o == &keypair.pubkey()) {
                keypair = Keypair::new();
            };
            keypair.pubkey()
        };
        let mut transaction = Transaction::new_with_payer(
            &[
                setup_voter_weight_record(
                    &program_id,
                    &realm_address,
                    &mint.pubkey(),
                    &not_owner,
                    &payer.pubkey(),
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.partial_sign(&[&payer], recent_blockhash);
        assert_eq!(
            banks_client.process_transaction(transaction).await.unwrap_err().unwrap(),
            trx_instruction_error(0, VoterWeightAddinError::WrongTokenOwner)
        );
    }

    let owner = read_keypair_file("../../artifacts/tst18qx7Kd3ELAsM3Qxn4nKNRZeg26Zi7GKGHaeWFm6.keypair").unwrap();
    let weight = VOTER_LIST.iter().filter_map(|(o,w)| if o == &owner.pubkey() {Some(w)} else {None}).next().unwrap();
    let set_vote_percentage_transaction = |authority: &Keypair, percentage: u16| {
        let mut transaction = Transaction::new_with_payer(
            &[
                set_vote_percentage_with_realm(
                    &program_id,
                    &mint.pubkey(),
                    &owner.pubkey(),
                    &authority.pubkey(),
                    &governance_id,
                    &realm_address,
                    percentage,
                ),
            ],
            Some(&payer.pubkey()),
        );
        transaction.partial_sign(&[&payer, &authority], recent_blockhash);
        transaction
    };

    // Check SetVotePercentage on different values
    {
        for percentage in [0u16, 3000u16, 10000u16, 10001u16] {
            let transaction = set_vote_percentage_transaction(&owner, percentage);
            let result = banks_client.process_transaction(transaction).await;
    
            if percentage > 10000 {
                assert_eq!(
                    result.unwrap_err().unwrap(),
                    trx_instruction_error(0, VoterWeightAddinError::InvalidPercentage)
                );
            } else {
                assert_eq!(result.is_ok(), true);

                let voter_weight_record_address = get_voter_weight_address(
                    &program_id,
                    &realm_address,
                    &mint.pubkey(),
                    &owner.pubkey()
                ).0;
                let voter_weight_record_account = banks_client.get_account(voter_weight_record_address).await.unwrap().unwrap();
                let voter_weight_record: VoterWeightRecord = try_from_slice_unchecked(&voter_weight_record_account.data).unwrap();
        
                let expected_weight: u64 = (*weight as u128)
                    .checked_mul(percentage.into()).unwrap()
                    .checked_div(10000).unwrap().try_into().unwrap();
        
                assert_eq!(voter_weight_record.voter_weight, expected_weight);
            }
        }
    }

    // Not owner or delegate can't change percentage
    {
        let transaction = set_vote_percentage_transaction(&payer, 9999);
        assert_eq!(
            banks_client.process_transaction(transaction).await.unwrap_err().unwrap(),
            trx_instruction_error(0, GovernanceError::GoverningTokenOwnerOrDelegateMustSign),
        );
    }

    // Delegate can change too
    {
        let mut set_governance_delegate_transaction = Transaction::new_with_payer(
            &[
                set_governance_delegate(
                    &governance_id,
                    &owner.pubkey(),
                    &realm_address,
                    &mint.pubkey(),
                    &owner.pubkey(),
                    &Some(payer.pubkey()),
                ),
            ],
            Some(&payer.pubkey()),
        );
        set_governance_delegate_transaction.partial_sign(&[&payer, &owner], recent_blockhash);
        banks_client.process_transaction(set_governance_delegate_transaction).await.unwrap();

        let transaction = set_vote_percentage_transaction(&payer, 9998);
        banks_client.process_transaction(transaction).await.unwrap();
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
