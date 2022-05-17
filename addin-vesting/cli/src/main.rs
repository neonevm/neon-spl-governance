// use std::str::FromStr;
use chrono::{DateTime, Duration};
use clap::{
    crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
};
use solana_clap_utils::{
    input_parsers::{keypair_of, pubkey_of, value_of, values_of},
    input_validators::{is_amount, is_keypair, is_pubkey, is_slot, is_url},
};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{ RpcProgramAccountsConfig, RpcAccountInfoConfig, },
    rpc_filter,
};
use solana_program::{
    borsh::try_from_slice_unchecked,
    msg, program_pack::Pack, pubkey::Pubkey, rent::Rent,
};
use solana_sdk::{
    self, commitment_config::CommitmentConfig, signature::Keypair, signature::Signer,
    account::Account,
    system_instruction,
    instruction::Instruction,
    transaction::Transaction,
    // signer::{ 
    //     keypair::read_keypair_file,
    // },
};
use spl_associated_token_account::{get_associated_token_address};
use spl_token;
use std::convert::TryInto;
use spl_governance_addin_vesting::{
    state::{ VestingRecord, VestingSchedule },
    instruction::{ deposit, deposit_with_realm, withdraw, withdraw_with_realm, change_owner, change_owner_with_realm, create_voter_weight_record, set_vote_percentage_with_realm },
    voter_weight::get_voter_weight_record_address,
};

// Lock the vesting contract
fn command_deposit_svc(
    rpc_client: RpcClient,
    vesting_addin_program_id: Pubkey,
    payer: Keypair,
    source_token_owner: Keypair,
    possible_source_token_pubkey: Option<Pubkey>,
    vesting_owner_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    schedules: Vec<VestingSchedule>,
    confirm: bool,
) {
    // If no source token account was given, use the associated source account
    let source_token_pubkey = match possible_source_token_pubkey {
        None => get_associated_token_address(&source_token_owner.pubkey(), &mint_pubkey),
        _ => possible_source_token_pubkey.unwrap(),
    };

    let vesting_token_keypair = Keypair::new();
    let vesting_token_pubkey = vesting_token_keypair.pubkey();

    let (vesting_pubkey,_) = Pubkey::find_program_address(&[&vesting_token_pubkey.as_ref()], &vesting_addin_program_id);

    let instructions = [
        system_instruction::create_account(
            &source_token_owner.pubkey(),
            &vesting_token_pubkey,
            Rent::default().minimum_balance(spl_token::state::Account::LEN),
            spl_token::state::Account::LEN as u64,
            &spl_token::id()
        ),
        spl_token::instruction::initialize_account(
            &spl_token::id(), 
            &vesting_token_pubkey,
            &mint_pubkey, 
            &vesting_pubkey
        ).unwrap(),
        deposit(
            &vesting_addin_program_id,
            &spl_token::id(),
            &vesting_token_pubkey,
            &source_token_owner.pubkey(),
            &source_token_pubkey,
            &vesting_owner_pubkey,
            &payer.pubkey(),
            schedules,
        )
        .unwrap(),
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));

    let latest_blockhash = rpc_client.get_latest_blockhash().unwrap();
    transaction.sign(&[&payer, &vesting_token_keypair, &source_token_owner], latest_blockhash);

    msg!("Vesting addin program id: {:?}", vesting_addin_program_id,);
    msg!("spl-token program id: {:?}", spl_token::id(),);
    msg!("Source token owner pubkey: {:?}", source_token_owner.pubkey(),);
    msg!("Source token pubkey: {:?}", source_token_pubkey,);
    msg!("Vesting owner pubkey: {:?}", vesting_owner_pubkey,);
    msg!("Payer: {:?}", payer.pubkey(),);
    msg!("The vesting account pubkey: {:?}", vesting_pubkey,);
    msg!("The vesting token pubkey: {:?}", vesting_token_pubkey,);

    if confirm {
        rpc_client
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &transaction,
                CommitmentConfig::confirmed(),
                // CommitmentConfig::finalized(),
            )
            .unwrap();
    } else {
        rpc_client.send_transaction(&transaction).unwrap();
    }
}

fn command_deposit_with_realm_svc(
    rpc_client: RpcClient,
    governance_program_id: Pubkey,
    vesting_addin_program_id: Pubkey,
    payer: Keypair,
    source_token_owner: Keypair,
    possible_source_token_pubkey: Option<Pubkey>,
    vesting_owner_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    realm_pubkey: Pubkey,
    schedules: Vec<VestingSchedule>,
    confirm: bool,
) {
    // If no source token account was given, use the associated source account
    let source_token_pubkey = match possible_source_token_pubkey {
        None => get_associated_token_address(&source_token_owner.pubkey(), &mint_pubkey),
        _ => possible_source_token_pubkey.unwrap(),
    };

    let vesting_token_keypair = Keypair::new();
    let vesting_token_pubkey = vesting_token_keypair.pubkey();

    let (vesting_pubkey,_) = Pubkey::find_program_address(&[&vesting_token_pubkey.as_ref()], &vesting_addin_program_id);

    let instructions = [
        system_instruction::create_account(
            &source_token_owner.pubkey(),
            &vesting_token_pubkey,
            Rent::default().minimum_balance(spl_token::state::Account::LEN),
            spl_token::state::Account::LEN as u64,
            &spl_token::id()
        ),
        spl_token::instruction::initialize_account(
            &spl_token::id(), 
            &vesting_token_pubkey,
            &mint_pubkey, 
            &vesting_pubkey
        ).unwrap(),
        deposit_with_realm(
            &vesting_addin_program_id,
            &spl_token::id(),
            &vesting_token_pubkey,
            &source_token_owner.pubkey(),
            &source_token_pubkey,
            &vesting_owner_pubkey,
            &payer.pubkey(),
            schedules,
            &governance_program_id,
            &realm_pubkey,
            &mint_pubkey,
        )
        .unwrap(),
    ];

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));

    let latest_blockhash = rpc_client.get_latest_blockhash().unwrap();
    transaction.sign(&[&payer, &vesting_token_keypair, &source_token_owner], latest_blockhash);

    msg!("Vesting addin program id: {:?}", vesting_addin_program_id,);
    msg!("spl-token program id: {:?}", spl_token::id(),);
    msg!("Source token owner pubkey: {:?}", source_token_owner.pubkey(),);
    msg!("Source token pubkey: {:?}", source_token_pubkey,);
    msg!("Vesting owner pubkey: {:?}", vesting_owner_pubkey,);
    msg!("Payer: {:?}", payer.pubkey(),);
    msg!("Governance program id: {:?}", governance_program_id,);
    msg!("The vesting account pubkey: {:?}", vesting_pubkey,);
    msg!("The vesting token pubkey: {:?}", vesting_token_pubkey,);

    if confirm {
        rpc_client
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &transaction,
                CommitmentConfig::confirmed(),
                // CommitmentConfig::finalized(),
            )
            .unwrap();
    } else {
        rpc_client.send_transaction(&transaction).unwrap();
    }
}

fn command_withdraw_svc(
    rpc_client: RpcClient,
    vesting_addin_program_id: Pubkey,
    payer: Keypair,
    vesting_owner: Keypair,
    vesting_token_pubkey: Pubkey,
    destination_token_pubkey: Pubkey,
) {

    let withdraw_instruction = withdraw(
        &vesting_addin_program_id,
        &spl_token::id(),
        &vesting_token_pubkey,
        &destination_token_pubkey,
        &vesting_owner.pubkey(),
    )
    .unwrap();

    let mut transaction = Transaction::new_with_payer(&[withdraw_instruction], Some(&payer.pubkey()));

    let latest_blockhash = rpc_client.get_latest_blockhash().unwrap();
    transaction.sign(&[&payer, &vesting_owner], latest_blockhash);

    rpc_client.send_transaction(&transaction).unwrap();
}

fn command_withdraw_with_realm_svc(
    rpc_client: RpcClient,
    governance_program_id: Pubkey,
    vesting_addin_program_id: Pubkey,
    payer: Keypair,
    vesting_owner: Keypair,
    vesting_token_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    realm_pubkey: Pubkey,
    destination_token_pubkey: Pubkey,
) {

    let withdraw_instruction = withdraw_with_realm(
        &vesting_addin_program_id,
        &spl_token::id(),
        &vesting_token_pubkey,
        &destination_token_pubkey,
        &vesting_owner.pubkey(),
        &governance_program_id,
        &realm_pubkey,
        &mint_pubkey,
    )
    .unwrap();

    let mut transaction = Transaction::new_with_payer(&[withdraw_instruction], Some(&payer.pubkey()));

    let latest_blockhash = rpc_client.get_latest_blockhash().unwrap();
    transaction.sign(&[&payer, &vesting_owner], latest_blockhash);

    rpc_client.send_transaction(&transaction).unwrap();
}

fn command_change_owner(
    rpc_client: RpcClient,
    vesting_addin_program_id: Pubkey,
    payer: Keypair,
    vesting_owner: Keypair,
    vesting_token_pubkey: Pubkey,
    new_vesting_owner_pubkey: Pubkey,
) {

    let change_owner_instruction = change_owner(
        &vesting_addin_program_id,
        &vesting_token_pubkey,
        &vesting_owner.pubkey(),
        &new_vesting_owner_pubkey,
    )
    .unwrap();

    let mut transaction = Transaction::new_with_payer(&[change_owner_instruction], Some(&payer.pubkey()));

    let latest_blockhash = rpc_client.get_latest_blockhash().unwrap();
    transaction.sign(
        &[&payer, &vesting_owner],
        latest_blockhash,
    );

    rpc_client.send_transaction(&transaction).unwrap();
}

fn command_change_owner_with_realm(
    rpc_client: RpcClient,
    governance_program_id: Pubkey,
    vesting_addin_program_id: Pubkey,
    payer: Keypair,
    vesting_owner: Keypair,
    vesting_token_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    realm_pubkey: Pubkey,
    new_vesting_owner_pubkey: Pubkey,
) {

    let mut instructions: Vec<Instruction> = Vec::new();

    let new_voter_weight_record_pubkey = get_voter_weight_record_address(&vesting_addin_program_id, &realm_pubkey, &mint_pubkey, &new_vesting_owner_pubkey);

    let new_voter_weight_record_data_result = rpc_client.get_account_data(&new_voter_weight_record_pubkey);
    if new_voter_weight_record_data_result.is_err() || new_voter_weight_record_data_result.unwrap().is_empty() {

        let create_voter_weight_record_instruction = create_voter_weight_record(
            &vesting_addin_program_id,
            &new_vesting_owner_pubkey,
            &payer.pubkey(),
            &governance_program_id,
            &realm_pubkey,
            &mint_pubkey,
        )
        .unwrap();
        instructions.push(create_voter_weight_record_instruction);
    }

    let change_owner_instruction = change_owner_with_realm(
        &vesting_addin_program_id,
        &vesting_token_pubkey,
        &vesting_owner.pubkey(),
        &new_vesting_owner_pubkey,
        &governance_program_id,
        &realm_pubkey,
        &mint_pubkey,
    )
    .unwrap();
    instructions.push(change_owner_instruction);

    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));

    let latest_blockhash = rpc_client.get_latest_blockhash().unwrap();
    transaction.sign(
        &[&payer, &vesting_owner],
        latest_blockhash,
    );

    rpc_client.send_transaction(&transaction).unwrap();
}

fn command_create_voter_weight_record(
    rpc_client: RpcClient,
    governance_program_id: Pubkey,
    vesting_addin_program_id: Pubkey,
    payer: Keypair,
    record_owner_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    realm_pubkey: Pubkey,
) {

    let instruction = create_voter_weight_record(
        &vesting_addin_program_id,
        &record_owner_pubkey,
        &payer.pubkey(),
        &governance_program_id,
        &realm_pubkey,
        &mint_pubkey,
    )
    .unwrap();

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));

    let latest_blockhash = rpc_client.get_latest_blockhash().unwrap();
    transaction.sign(
        &[&payer],
        latest_blockhash,
    );

    rpc_client.send_transaction(&transaction).unwrap();
}

fn command_set_vote_percentage_with_realm(
    rpc_client: RpcClient,
    governance_program_id: Pubkey,
    vesting_addin_program_id: Pubkey,
    payer: Keypair,
    vesting_authority: Keypair,
    vesting_owner_pubkey: Pubkey,
    mint_pubkey: Pubkey,
    realm_pubkey: Pubkey,
    percentage: u16,
) {

    let instruction = set_vote_percentage_with_realm(
        &vesting_addin_program_id,
        &vesting_owner_pubkey,
        &vesting_authority.pubkey(),
        &governance_program_id,
        &realm_pubkey,
        &mint_pubkey,
        percentage,
    )
    .unwrap();

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));

    let latest_blockhash = rpc_client.get_latest_blockhash().unwrap();
    transaction.sign(
        &[&payer,&vesting_authority],
        latest_blockhash,
    );

    rpc_client.send_transaction(&transaction).unwrap();
}

fn command_info(
    rpc_client: RpcClient,
    vesting_addin_program_id: Pubkey,
    vesting_token_pubkey: Pubkey,
) {
    msg!("\n---------------VESTING--CONTRACT--INFO-----------------\n");
    // msg!("RPC URL: {:?}", &rpc_url);
    msg!("Program ID: {:?}", &vesting_addin_program_id);

    let (vesting_pubkey,_) = Pubkey::find_program_address(&[&vesting_token_pubkey.as_ref()], &vesting_addin_program_id);
    msg!("Vesting Account Pubkey: {:?}", &vesting_pubkey);

    let vesting_record_account_data = rpc_client.get_account_data(&vesting_pubkey).unwrap();
    let vesting_record: VestingRecord = try_from_slice_unchecked(&vesting_record_account_data).unwrap();
    msg!("Vesting Token Account Pubkey: {:?}", &vesting_token_pubkey);
    report_vesting_record_info(&vesting_record);
}

fn report_vesting_record_info(vesting_record: &VestingRecord) {
    msg!("Vesting Owner Address: {:?}", &vesting_record.owner);
    msg!("Vesting Mint Address:  {:?}", &vesting_record.mint);
    msg!("Vesting Token Address: {:?}", &vesting_record.token);
    msg!("Vesting Realm: {:?}", &vesting_record.realm);

    let schedules = &vesting_record.schedule;

    for i in 0..schedules.len() {
        msg!("\nSCHEDULE {:?}", i);
        msg!("Release Height: {:?}", &schedules[i].release_time);
        msg!("Amount: {:?}", &schedules[i].amount);
    }
}

fn main() {
    let matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::with_name("rpc_url")
                .long("url")
                .value_name("URL")
                .default_value("http://localhost:8899")
                .validator(is_url)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the url of the rpc client (solana network).",
                ),
        )
        .arg(
            Arg::with_name("governance_program_id")
                .long("governance_program_id")
                .value_name("ADDRESS")
                .default_value("82pQHEmBbW6CQS8GzLP3WE2pCgMUPSW2XzpuSih3aFDk")
                .validator(is_pubkey)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the address (public key) of the governance program.",
                ),
        )
        .arg(
            Arg::with_name("vesting_program_id")
                .long("vesting_program_id")
                .value_name("ADDRESS")
                .default_value("Hu548Kzvfo9C9zATuXVpnmxYRUCJxrsXLdiKjxuTczim")
                .validator(is_pubkey)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the address (public key) of the vesting addin program.",
                ),
        )
        .subcommand(SubCommand::with_name("deposit").about("Create a new vesting contract with an optional release schedule")        
            .arg(
                Arg::with_name("source_owner")
                    .long("source_owner")
                    .value_name("KEYPAIR")
                    .required(true)
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the source account owner. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("source_token_address")
                    .long("source_token_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the source token account address.",
                    ),
            )
            .arg(
                Arg::with_name("vesting_owner")
                    .long("vesting_owner")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the address (publickey) of the vesting record owner.",
                    ),
            )
            .arg(
                Arg::with_name("mint_address")
                    .long("mint_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the address (publickey) of the mint for the token that should be used.",
                    ),
            )
            .arg(
                Arg::with_name("realm_address")
                    .long("realm_address")
                    .value_name("ADDRESS")
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the address (publickey) of the governance realm.",
                    ),
            )
            .arg(
                Arg::with_name("amounts")
                    .long("amounts")
                    .value_name("AMOUNT")
                    .required(true)
                    .validator(is_amount)
                    .takes_value(true)
                    .multiple(true)
                    .use_delimiter(true)
                    .value_terminator("!")
                    .allow_hyphen_values(true)
                    .help(
                        "Amounts of tokens to transfer via the vesting \
                        contract. Multiple inputs separated by a comma are
                        accepted for the creation of multiple schedules. The sequence of inputs \
                        needs to end with an exclamation mark ( e.g. 1,2,3,! )",
                    ),
            )
            // scheduled vesting
            .arg(
                Arg::with_name("release-times")
                    .long("release-times")
                    .conflicts_with("release-frequency")
                    .value_name("SLOT")
                    .validator(is_slot)
                    .takes_value(true)
                    .multiple(true)
                    .use_delimiter(true)
                    .value_terminator("!")
                    .allow_hyphen_values(true)
                    .help(
                        "Release times in unix timestamp to decide when the contract is \
                        unlockable. Multiple inputs separated by a comma are
                        accepted for the creation of multiple schedules. The sequence of inputs \
                        needs to end with an exclamation mark ( e.g. 1,2,3,! ).",
                    ),
            )
            // linear vesting
            .arg(
                Arg::with_name("release-frequency")
                    .long("release-frequency")
                    .value_name("RELEASE_FREQUENCY")
                    .takes_value(true)
                    .conflicts_with("release-times")                                        
                    .help(
                        "Frequency of release amount. \
                        You start on 1sth of Nov and end on 5th of Nov. \
                        With 1 day frequency it will vest from total amount 5 times \
                        splitted linearly.
                        Duration must be ISO8601 duration format. Example, P1D.
                        Internally all dates will be transformed into schedule.",
                    ),
            )
            .arg(
                Arg::with_name("start-date-time")
                    .long("start-date-time")
                    .value_name("START_DATE_TIME")
                    .takes_value(true)
                    .help(
                        "First time of release in linear vesting. \
                        Must be RFC 3339 and ISO 8601 sortable date time. \
                        Example, 2022-01-06T20:11:18Z",
                    ),
            )
            .arg(
                Arg::with_name("end-date-time")
                    .long("end-date-time")
                    .value_name("END_DATE_TIME")
                    .takes_value(true)
                    .help(
                        "Last time of release in linear vesting. \
                        If frequency will go over last release time, \
                        tokens will be released later than end date. 
                        Must be RFC 3339 and ISO 8601 sortable date time. \
                        Example, 2022-17-06T20:11:18Z",
                    ),
            )
            .arg(
                Arg::with_name("payer")
                    .long("payer")
                    .value_name("KEYPAIR")
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the transaction fee payer account address. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("confirm")
                    .long("confirm")
                    .value_name("CONFIRM")
                    .takes_value(true)
                    .default_value("true")
                    .help(
                        "Specify whether to wait transaction confirmation"
                    ),
            )
        )
        .subcommand(SubCommand::with_name("withdraw").about("Unlock & Withdraw a vesting contract. This will only release \
        the schedules that have reached maturity.")
            .arg(
                Arg::with_name("payer")
                    .long("payer")
                    .value_name("KEYPAIR")
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the transaction fee payer account address. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("vesting_owner")
                    .long("vesting_owner")
                    .value_name("KEYPAIR")
                    .required(true)
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the vesting owner account address. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("vesting_address")
                    .long("vesting_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the vesting token address (publickey).",
                    ),
            )
            .arg(
                Arg::with_name("destination_address")
                    .long("destination_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the destination token address (publickey).",
                    ),
            )
        )
        .subcommand(SubCommand::with_name("change-owner").about("Change the owner of a vesting contract")
            .arg(
                Arg::with_name("payer")
                    .long("payer")
                    .value_name("KEYPAIR")
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the transaction fee payer account address. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("vesting_owner")
                    .long("vesting_owner")
                    .value_name("KEYPAIR")
                    .required(true)
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the vesting owner account address. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("vesting_address")
                    .long("vesting_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the vesting token address (publickey).",
                    ),
            )
            .arg(
                Arg::with_name("new_vesting_owner")
                    .long("new_vesting_owner")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the new vesting owner address (publickey).",
                    ),
            )
        )
        .subcommand(SubCommand::with_name("create-voter-weight-record").about("Create Voter Weight Record")
            .arg(
                Arg::with_name("payer")
                    .long("payer")
                    .value_name("KEYPAIR")
                    .required(true)
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the transaction fee payer account address. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("record_owner")
                    .long("record_owner")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the record owner address (publickey).",
                    ),
            )
            .arg(
                Arg::with_name("mint_address")
                    .long("mint_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the address (publickey) of the mint for the token that should be used.",
                    ),
            )
            .arg(
                Arg::with_name("realm_address")
                    .long("realm_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the address (publickey) of the governance realm.",
                    ),
            )
        )
        .subcommand(SubCommand::with_name("set-vote-percentage").about("Set vote percentage of a vesting contract for a Realm")
            .arg(
                Arg::with_name("payer")
                    .long("payer")
                    .value_name("KEYPAIR")
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the transaction fee payer account address. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("vesting_authority")
                    .long("vesting_authority")
                    .value_name("KEYPAIR")
                    .required(true)
                    .validator(is_keypair)
                    .takes_value(true)
                    .help(
                        "Specify the vesting authority account address. \
                        This may be a keypair file, the ASK keyword. \
                        Defaults to the client keypair.",
                    ),
            )
            .arg(
                Arg::with_name("vesting_owner")
                    .long("vesting_owner")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the vesting owner address (publickey).",
                    ),
            )
            .arg(
                Arg::with_name("mint_address")
                    .long("mint_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the address (publickey) of the mint for the token that should be used.",
                    ),
            )
            .arg(
                Arg::with_name("realm_address")
                    .long("realm_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the address (publickey) of the governance realm.",
                    ),
            )
            .arg(
                Arg::with_name("percentage")
                    .long("percentage")
                    .value_name("PERCENTAGE")
                    .required(true)
                    .validator(is_amount)
                    .takes_value(true)
                    .help(
                        "Deposited tokens percentage of voting.",
                    ),
            )
        )
        .subcommand(SubCommand::with_name("info").about("Print information about a vesting contract")
            .arg(
                Arg::with_name("vesting_address")
                    .long("vesting_address")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the vesting token address (publickey).",
                    ),
            )
        )
        .subcommand(SubCommand::with_name("info-owner").about("Print information about vesting contracts of a vesting owner")
            .arg(
                Arg::with_name("vesting_owner")
                    .long("vesting_owner")
                    .value_name("ADDRESS")
                    .required(true)
                    .validator(is_pubkey)
                    .takes_value(true)
                    .help(
                        "Specify the vesting owner address (publickey).",
                    ),
            )
        )
        .get_matches();

    let rpc_url = value_t!(matches, "rpc_url", String).unwrap();
    let rpc_client = RpcClient::new(rpc_url);

    let governance_program_id = pubkey_of(&matches, "governance_program_id").unwrap();
    let vesting_addin_program_id = pubkey_of(&matches, "vesting_program_id").unwrap();

    let _ = match matches.subcommand() {
        ("deposit", Some(arg_matches)) => {
            let source_keypair = keypair_of(arg_matches, "source_owner").unwrap();
            let source_token_pubkey = pubkey_of(arg_matches, "source_token_address");
            let vesting_owner_pubkey = pubkey_of(arg_matches, "vesting_owner").unwrap();

            let mint_pubkey = pubkey_of(arg_matches, "mint_address").unwrap();
            let realm_opt: Option<Pubkey> = pubkey_of(arg_matches, "realm_address");

            let payer_keypair = keypair_of(arg_matches, "payer").unwrap_or( keypair_of(arg_matches, "source_owner").unwrap() );

            // Parsing schedules
            let mut schedule_amounts: Vec<u64> = values_of(arg_matches, "amounts").unwrap();
            let confirm: bool = value_of(arg_matches, "confirm").unwrap();
            let release_frequency: Option<String> = value_of(arg_matches, "release-frequency");

            let schedule_times = if release_frequency.is_some() {
                // best found in rust
                let release_frequency: iso8601_duration::Duration =
                    release_frequency.unwrap().parse().unwrap();
                let release_frequency: u64 = Duration::from_std(release_frequency.to_std())
                    .unwrap()
                    .num_seconds()
                    .try_into()
                    .unwrap();
                if schedule_amounts.len() > 1 {
                    panic!("Linear vesting must have one amount which will split into parts per period")
                }
                let start: u64 = DateTime::parse_from_rfc3339(
                    &value_of::<String>(arg_matches, "start-date-time").unwrap(),
                )
                .unwrap()
                .timestamp()
                .try_into()
                .unwrap();
                let end: u64 = DateTime::parse_from_rfc3339(
                    &value_of::<String>(arg_matches, "end-date-time").unwrap(),
                )
                .unwrap()
                .timestamp()
                .try_into()
                .unwrap();
                let total = schedule_amounts[0];
                let part = (((total as u128) * (release_frequency as u128))
                    / ((end - start) as u128))
                    .try_into()
                    .unwrap();
                schedule_amounts.clear();
                let mut linear_vesting = Vec::new();

                let q = total / part;
                let r = total % part;

                for n in 0..q {
                    linear_vesting.push(start + n * release_frequency);
                    schedule_amounts.push(part);
                }

                if r != 0 {
                    schedule_amounts[(q - 1) as usize] += r;
                }

                if linear_vesting.len() > 365 {
                    panic!("Total count of vesting periods is more than 365. Not sure if you want to do that.")
                }

                assert_eq!(schedule_amounts.iter().sum::<u64>(), total);

                linear_vesting
            } else {
                values_of(arg_matches, "release-times").unwrap()
            };

            if schedule_amounts.len() != schedule_times.len() {
                eprintln!("error: Number of amounts given is not equal to number of release heights given.");
                std::process::exit(1);
            }
            let mut schedules: Vec<VestingSchedule> = Vec::with_capacity(schedule_amounts.len());
            for (&a, &h) in schedule_amounts.iter().zip(schedule_times.iter()) {
                schedules.push(VestingSchedule {
                    release_time: h,
                    amount: a,
                });
            }

            if let Some(realm_pubkey) = realm_opt {
                command_deposit_with_realm_svc(
                    rpc_client,
                    governance_program_id,
                    vesting_addin_program_id,
                    payer_keypair,
                    source_keypair,
                    source_token_pubkey,
                    vesting_owner_pubkey,
                    mint_pubkey,
                    realm_pubkey,
                    schedules,
                    confirm,
                )
            } else {
                command_deposit_svc(
                    rpc_client,
                    vesting_addin_program_id,
                    payer_keypair,
                    source_keypair,
                    source_token_pubkey,
                    vesting_owner_pubkey,
                    mint_pubkey,
                    schedules,
                    confirm,
                )
            }
        }
        ("withdraw", Some(arg_matches)) => {

            let vesting_owner_keypair = keypair_of(arg_matches, "vesting_owner").unwrap();
            let vesting_token_pubkey = pubkey_of(arg_matches, "vesting_address").unwrap();

            let destination_token_pubkey = pubkey_of(arg_matches, "destination_address").unwrap();

            let payer_keypair = keypair_of(arg_matches, "payer").unwrap_or( keypair_of(arg_matches, "vesting_owner").unwrap() );

            let (vesting_pubkey,_) = Pubkey::find_program_address(&[&vesting_token_pubkey.as_ref()], &vesting_addin_program_id);

            let vesting_record_account_data = rpc_client.get_account_data(&vesting_pubkey).unwrap();
            let vesting_record: VestingRecord = try_from_slice_unchecked(&vesting_record_account_data).unwrap();

            if let Some(realm_pubkey) = vesting_record.realm {
                let mint_pubkey: Pubkey = vesting_record.mint;

                command_withdraw_with_realm_svc(
                    rpc_client,
                    governance_program_id,
                    vesting_addin_program_id,
                    payer_keypair,
                    vesting_owner_keypair,
                    vesting_token_pubkey,
                    mint_pubkey,
                    realm_pubkey,
                    destination_token_pubkey,
                )
            } else {
                command_withdraw_svc(
                    rpc_client,
                    vesting_addin_program_id,
                    payer_keypair,
                    vesting_owner_keypair,
                    vesting_token_pubkey,
                    destination_token_pubkey,
                )
            };
        }
        ("change-owner", Some(arg_matches)) => {

            let vesting_owner_keypair = keypair_of(arg_matches, "vesting_owner").unwrap();
            let vesting_token_pubkey = pubkey_of(arg_matches, "vesting_address").unwrap();

            let new_vesting_owner_pubkey = pubkey_of(arg_matches, "new_vesting_owner").unwrap();
            
            let payer_keypair = keypair_of(arg_matches, "payer").unwrap_or( keypair_of(arg_matches, "vesting_owner").unwrap() );

            let (vesting_pubkey,_) = Pubkey::find_program_address(&[&vesting_token_pubkey.as_ref()], &vesting_addin_program_id);

            let vesting_record_account_data = rpc_client.get_account_data(&vesting_pubkey).unwrap();
            let vesting_record: VestingRecord = try_from_slice_unchecked(&vesting_record_account_data).unwrap();

            if let Some(realm_pubkey) = vesting_record.realm {
                let mint_pubkey: Pubkey = vesting_record.mint;

                command_change_owner_with_realm(
                    rpc_client,
                    governance_program_id,
                    vesting_addin_program_id,
                    payer_keypair,
                    vesting_owner_keypair,
                    vesting_token_pubkey,
                    mint_pubkey,
                    realm_pubkey,
                    new_vesting_owner_pubkey,
                )
            } else {
                command_change_owner(
                    rpc_client,
                    vesting_addin_program_id,
                    payer_keypair,
                    vesting_owner_keypair,
                    vesting_token_pubkey,
                    new_vesting_owner_pubkey,
                )
            }
        }
        ("create-voter-weight-record", Some(arg_matches)) => {

            let record_owner_pubkey = pubkey_of(arg_matches, "record_owner").unwrap();
            
            let mint_pubkey = pubkey_of(arg_matches, "mint_address").unwrap();
            let realm_pubkey = pubkey_of(arg_matches, "realm_address").unwrap();
            
            let payer_keypair = keypair_of(arg_matches, "payer").unwrap();

            command_create_voter_weight_record(
                rpc_client,
                governance_program_id,
                vesting_addin_program_id,
                payer_keypair,
                record_owner_pubkey,
                mint_pubkey,
                realm_pubkey,
            )
        }
        ("set-vote-percentage", Some(arg_matches)) => {

            let vesting_authority = keypair_of(arg_matches, "vesting_authority").unwrap();
            let mint_pubkey = pubkey_of(arg_matches, "mint_address").unwrap();
            let realm_pubkey = pubkey_of(arg_matches, "realm_address").unwrap();

            let vesting_owner_pubkey = pubkey_of(arg_matches, "vesting_owner").unwrap();
            
            let percentage: u16 = value_of(arg_matches, "percentage").unwrap();

            let payer_keypair = keypair_of(arg_matches, "payer").unwrap_or( keypair_of(arg_matches, "vesting_authority").unwrap() );

            command_set_vote_percentage_with_realm(
                rpc_client,
                governance_program_id,
                vesting_addin_program_id,
                payer_keypair,
                vesting_authority,
                vesting_owner_pubkey,
                mint_pubkey,
                realm_pubkey,
                percentage,
            )
        }
        ("info", Some(arg_matches)) => {
            let vesting_token_pubkey = pubkey_of(arg_matches, "vesting_address").unwrap();
            command_info(rpc_client, vesting_addin_program_id, vesting_token_pubkey)
        }
        ("info-owner", Some(arg_matches)) => {
            let vesting_owner_pubkey = pubkey_of(arg_matches, "vesting_owner").unwrap();

            let records: Vec<(Pubkey,Account)> =
                rpc_client.get_program_accounts_with_config(
                    &vesting_addin_program_id,
                    RpcProgramAccountsConfig {
                        filters: Some(vec![
                            rpc_filter::RpcFilterType::Memcmp(
                                rpc_filter::Memcmp {
                                    offset: 0,
                                    bytes: rpc_filter::MemcmpEncodedBytes::Bytes({
                                        let mut fd: Vec<u8> = vec![1];
                                        fd.append(&mut vesting_owner_pubkey.to_bytes().to_vec());
                                        fd
                                    }),
                                    encoding: None,
                                },
                            )
                        ]),
                        account_config: RpcAccountInfoConfig {
                            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                            data_slice: None,
                            commitment: None,
                        },
                        with_context: Some(false),
                    }
                ).unwrap();
            
            for (vesting_account_pubkey, vesting_account) in records {
                let vesting_record: VestingRecord = try_from_slice_unchecked(&vesting_account.data).unwrap();
                msg!("\nVesting Account Pubkey: {:?}", &vesting_account_pubkey);
                report_vesting_record_info(&vesting_record);
            }
        }
        _ => unreachable!(),
    };
}
