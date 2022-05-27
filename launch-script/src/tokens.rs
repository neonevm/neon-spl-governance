use crate::errors::StateError;
use solana_sdk::{
    pubkey::{ Pubkey },
    instruction::{ Instruction },
    signer::{
        Signer,
    },
    program_pack::{ Pack },
};

use spl_token::state::{ Account, Mint, Multisig };

use governance_lib::{
    client::{Client, ClientResult},
};

pub fn get_mint_data(client: &Client, mint: &Pubkey) -> ClientResult<Option<Mint>> {
    client.get_account_data_pack::<Mint>(&spl_token::id(), mint)
}

pub fn get_account_data(client: &Client, account: &Pubkey) -> ClientResult<Option<Account>> {
    client.get_account_data_pack::<Account>(&spl_token::id(), account)
}

pub fn get_multisig_data(client: &Client, account: &Pubkey) -> ClientResult<Option<Multisig>> {
    client.get_account_data_pack::<Multisig>(&spl_token::id(), account)
}

pub fn assert_is_valid_account_data(d: &Account, address: &Pubkey, mint: &Pubkey, owner: &Pubkey) -> Result<(),StateError> {
    if d.mint != *mint {
        return Err(StateError::InvalidTokenAccountMint(*address, d.mint));
    }
    if d.owner != *owner {
        return Err(StateError::InvalidTokenAccountOwner(*address, d.owner));
    }
    Ok(())
}

pub fn create_mint_instructions(client: &Client, mint_pubkey: &Pubkey, mint_authority: &Pubkey,
        freeze_authority: Option<&Pubkey>, decimals: u8) -> ClientResult<Vec<Instruction>>
{
    Ok([
        solana_sdk::system_instruction::create_account(
            &client.payer.pubkey(),
            mint_pubkey,
            client.solana_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?,
            Mint::LEN as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint(
            &spl_token::id(),
            mint_pubkey,
            mint_authority,
            freeze_authority,
            decimals
        ).unwrap(),
    ].to_vec())
}

// pub fn create_mint(client: &Client, mint_keypair: &Keypair, mint_authority: &Pubkey,
//         freeze_authority: Option<&Pubkey>, decimals: u8) -> ClientResult<Signature>
// {
//     client.send_and_confirm_transaction(
//             &[
//                 solana_sdk::system_instruction::create_account(
//                     &client.payer.pubkey(),
//                     &mint_keypair.pubkey(),
//                     client.solana_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?,
//                     Mint::LEN as u64,
//                     &spl_token::id(),
//                 ),
//                 spl_token::instruction::initialize_mint(
//                     &spl_token::id(),
//                     &mint_keypair.pubkey(),
//                     mint_authority,
//                     freeze_authority,
//                     decimals
//                 ).unwrap(),
//             ],
//             &[mint_keypair],
//         )
// }

/*
pub fn create_account(client: &RpcClient, owner_keypair: &Keypair, mint_keypair: &Keypair, mint_pubkey: &Pubkey) -> Result<Pubkey,()> {

    let owner_pubkey: Pubkey = owner_keypair.pubkey();
    // let minter_pubkey: Pubkey = minter.pubkey();

    // let mint_keypair: Keypair = 
    //     if let Ok(keypair) = read_keypair_file(format!("/media/mich/speedwork/NeonLabs/artifacts/dev/token_mints/{}.keypair",token_info.symbol)) {
    //         keypair
    //     } else {
    //         return Err(())
    //     };
    // let mint_pubkey = mint_keypair.pubkey();

    // Create new Supply Account
    // let supply_keypair: Keypair = Keypair::new();
    // let supply_pubkey: Pubkey = supply_keypair.pubkey();
    // println!("Supply Account: {}", supply_pubkey);
    let associated_token_pubkey: Pubkey = spl_associated_token_account::get_associated_token_address(&owner_pubkey, &mint_pubkey);

    let create_supply_account_instruction: Instruction =
        spl_associated_token_account::create_associated_token_account(
            &owner_pubkey,
            &owner_pubkey,
            &mint_pubkey,
        );
        // solana_sdk::system_instruction::create_account(
        //     &owner_pubkey,
        //     &associated_token_pubkey,
        //     // 0,
        //     client.get_minimum_balance_for_rent_exemption(Account::LEN).unwrap(),
        //     Account::LEN as u64,
        //     &spl_token::id(),
        // );

    let initialize_supply_account2_instruction: Instruction =
        spl_token::instruction::initialize_account2(
            &spl_token::id(),
            &associated_token_pubkey,
            &mint_pubkey,
            &owner_pubkey,
        ).unwrap();


    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &vec![
                create_supply_account_instruction,
                // initialize_supply_account2_instruction,
            ],
            Some(&owner_pubkey),
            &[
                owner_keypair,
                // owner_keypair,
                // mint_keypair
            ],
            client.get_latest_blockhash().unwrap(),
        );

    let result = client.send_and_confirm_transaction(&transaction);
    println!("'Create Initialize Supply Account' Transaction Result: {:?}", result);

    Ok(associated_token_pubkey)
    // Ok((mint_pubkey, supply_pubkey))
}

pub fn mint_tokens(client: &RpcClient, mint_authority: &Keypair, mint_pubkey: &Pubkey, recipient_pubkey: &Pubkey, amount: u64) {

    // let mint_authority: &Keypair = &self.mint_authority.as_ref().unwrap();
    let mint_authority_pubkey: Pubkey = mint_authority.pubkey();

    let mint_to_instruction: Instruction =
        spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint_pubkey,
            &recipient_pubkey,
            &mint_authority_pubkey,
            &[ &mint_authority_pubkey ],
            amount,
        ).unwrap();

    let transaction: Transaction =
        Transaction::new_signed_with_payer(
            &vec![ mint_to_instruction ],
            Some(&mint_authority_pubkey),
            &[ mint_authority ],
            client.get_latest_blockhash().unwrap(),
        );
    
    let result = client.send_and_confirm_transaction(&transaction);
    println!("Mint result: {:?}", result);
}

pub fn create_accounts_mint_liquidity(client: &RpcClient, owner_keypair: &Keypair, mint_keypair: &Keypair, mint_pubkey: &Pubkey) {

    let amount: u64 = 10_000_000_000;
    // let amount: u64 = 2;

    // let client: SolClient =
    //     SolClient::new(NETWORK)
    //         .with_authority(WALLET_FILE_PATH)
    //         .with_mint_authority(MINTER_FILE_PATH);

    // let owner_keypair: Keypair = read_keypair_file(crate::WALLET_FILE_PATH).unwrap();
    let owner_pubkey: Pubkey = owner_keypair.pubkey();
    // println!("Solana Owner / Payer Pubkey: {}", owner_pubkey);

    let balance = client.get_balance(&owner_pubkey).unwrap();
    println!("Solana Owner / Payer Balance: {}", balance);
    if balance == 0 {
        println!("No Owner balance!!!");
        return;
    }

    // let eth_receiver_address: EthAddress = EthAddress::from_str(receivers[0]).unwrap();
    // println!("NeonEvm Receiver Address :  {}\n", eth_receiver_address);

    // let token_infos: Vec<TokenInfo> = {
    //     let token_list_string: String = std::fs::read_to_string("/media/mich/speedwork/NeonLabs/artifacts/tokenlist.json").unwrap();
    //     let token_list: TokenList = serde_json::from_str(&token_list_string).unwrap();
    //     token_list.filter_for_network(&NETWORK).unwrap()
    // };

    // for token_info in token_infos.iter() {
        // let mint_pubkey: Pubkey = Pubkey::from_str(&token_info.address_spl.as_ref().unwrap()).unwrap();
        // println!("'{}' [ {} ] : Mint Pubkey: {}", token_info.name, token_info.symbol, mint_pubkey);

        // let amount_expanded: u64 = amount.expand_to_decimals(token_info.decimals).unwrap();
        // let eth_erc20_address: EthAddress = EthAddress::from_str(&token_info.address).unwrap();
        // println!("eth_erc20_address: {}", eth_erc20_address);

        // let supply_keypair: Keypair = create_liquidity(&solana_client, &owner_keypair, &minter_keypair, &mint_pubkey, token_info, amount_expanded).unwrap();

        // transfer_liquidity(&solana_client, &NETWORK, &owner_keypair, &mint_pubkey, &supply_keypair.pubkey(), &token_info, &eth_receiver_address, &eth_erc20_address, amount_expanded).unwrap();

        // let evm_loader_program_id: Pubkey = Pubkey::from_str(NETWORK.get_evm_loader_program_id()).unwrap();
        // let recipient_pubkey: Pubkey = Erc20AccountIdentity::new(&eth_receiver_address,&eth_erc20_address,&mint_pubkey).derive_pubkey(&evm_loader_program_id);
        // let recipient_pubkey: Pubkey = client.create_derived_erc20_identity(&eth_receiver_address, &eth_erc20_address, &mint_pubkey);

        // match client.get_account(&recipient_pubkey) {
        //     Ok(_)   => {
        //         println!("'{}' [ {} ] : ERC20 Associated Token Account: {}", token_info.name, token_info.symbol, recipient_pubkey);
        //     },
        //     Err(_)  => {
        //         let result = client.create_associated_token_account(&mint_pubkey, &eth_receiver_address, &eth_erc20_address);
        //         match result {
        //             Ok(_)   =>
        //                 println!("'{}' [ {} ] : ERC20 Associated Token Account Created: {}", token_info.name, token_info.symbol, recipient_pubkey),
        //             Err(e)  =>
        //                 println!("Transaction error while creating associated token account {} for '{}' [ {} ].\n{:?}", recipient_pubkey, token_info.name, token_info.symbol, e),
        //         }
        //     }
        // };

        let recipient_pubkey: Pubkey = create_account(&client, &owner_keypair, &mint_keypair, &mint_pubkey).unwrap();
        println!("Recipient_pubkey: {}", recipient_pubkey);

        mint_tokens(client, &owner_keypair, mint_pubkey, &recipient_pubkey, amount);
        // match result {
        //     Ok(_)   =>
        //         println!("'{}' [ {} ] : {} Minted To {}", token_info.name, token_info.symbol, amount_expanded, recipient_pubkey),
        //     Err(e)  =>
        //         println!("Transaction error while minting for '{}' [ {} ].\n{:?}", token_info.name, token_info.symbol, e),
        // }
        // return;
    // }
}*/
