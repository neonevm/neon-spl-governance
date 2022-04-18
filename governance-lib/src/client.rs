use {
    borsh::BorshDeserialize,
    solana_sdk::{
        borsh::try_from_slice_unchecked,
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        instruction::Instruction,
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
        signers::Signers,
        signature::Signature,
        program_pack::{Pack, IsInitialized},
    },
    std::fmt,
    solana_client::{
        rpc_config::RpcSendTransactionConfig,
        rpc_client::RpcClient,
        client_error::ClientError,
    },
};

pub struct Client<'a> {
    pub url: String,
    pub payer: &'a Keypair,
    pub solana_client: RpcClient,
}

impl<'a> fmt::Debug for Client<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Client")
            .field("url", &self.url)
            .field("payer", &self.payer.pubkey())
            .finish()
    }
}

impl<'a> fmt::Display for Client<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Client")
            .field("url", &self.url)
            .field("payer", &self.payer.pubkey())
            .finish()
    }
}

impl<'a> Client<'a> {

    pub fn new(url: &str, payer: &'a Keypair) -> Self {
        Client {
            url: url.to_string(),
            solana_client: RpcClient::new_with_commitment(url.to_string(),CommitmentConfig::confirmed()),
            payer,
        }
    }

    pub fn send_and_confirm_transaction_with_payer_only(
            &self,
            instructions: &[Instruction],
    ) -> Result<Signature, ClientError> {
        self.send_and_confirm_transaction::<[&dyn solana_sdk::signature::Signer;0]>(
                instructions,
                &[],
            )
    }

    pub fn send_and_confirm_transaction<T: Signers>(
            &self,
            instructions: &[Instruction],
            signing_keypairs: &T,
    ) -> Result<Signature, ClientError> {
        let mut transaction: Transaction =
            Transaction::new_with_payer(
                instructions,
                Some(&self.payer.pubkey()),
            );

        let blockhash = self.solana_client.get_latest_blockhash().unwrap();
        transaction.partial_sign(&[self.payer], blockhash);
        transaction.sign(signing_keypairs, blockhash);
        
        //self.solana_client.send_and_confirm_transaction(&transaction)
        self.solana_client.send_and_confirm_transaction_with_spinner_and_config(&transaction, 
                self.solana_client.commitment(),
                RpcSendTransactionConfig {skip_preflight: true, ..RpcSendTransactionConfig::default()})
    }

    pub fn get_account_data_pack<T: Pack + IsInitialized>(
            &self,
            owner_program_id: &Pubkey,
            account_key: &Pubkey,
    ) -> Result<Option<T>, ClientError> {
        let account_info = &self.solana_client.get_account_with_commitment(
                &account_key, self.solana_client.commitment())?.value;

        if let Some(account_info) = account_info {
            if account_info.data.is_empty() {
                panic!("Account {} is empty", account_key);
            }
            if account_info.owner != *owner_program_id {
                panic!("Invalid account owner for {}: expected {}, actual {}",
                        account_key, owner_program_id, account_info.owner);
            }
        
            let account: T = T::unpack(&account_info.data).unwrap(); //try_from_slice_unchecked(&account_info.data).unwrap();
            if !account.is_initialized() {
                panic!("Unitialized account {}", account_key);
            }
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    pub fn get_account_data<T: BorshDeserialize + IsInitialized>(
            &self,
            owner_program_id: &Pubkey,
            account_key: &Pubkey,
    ) -> Result<Option<T>, ClientError> {
        let account_info = &self.solana_client.get_account_with_commitment(
                &account_key, self.solana_client.commitment())?.value;

        if let Some(account_info) = account_info {
            if account_info.data.is_empty() {
                panic!("Account {} is empty", account_key);
            }
            if account_info.owner != *owner_program_id {
                panic!("Invalid account owner for {}: expected {}, actual {}",
                        account_key, owner_program_id, account_info.owner);
            }
        
            let account: T = try_from_slice_unchecked(&account_info.data).unwrap();
            if !account.is_initialized() {
                panic!("Unitialized account {}", account_key);
            }
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    pub fn account_exists(&self, address: &Pubkey) -> bool {
        self.solana_client.get_account(address).is_ok()
    }

/*    pub fn _add_signatory(&self, realm: &Realm, _governance: &Governance, proposal: &Proposal, token_owner: &TokenOwner) -> Result<Signature,ClientError> {
        let realm_authority_pubkey: Pubkey = realm.authority.pubkey();
        // let signatory_record_address = get_signatory_record_address(&self.spl_governance_program_address, &proposal.address, &token_owner.authority.pubkey());

        let add_signatory_instruction: Instruction =
            add_signatory(
                &self.spl_governance_program_address,
                &proposal.address,
                &token_owner.token_owner_record_address,
                &realm_authority_pubkey,
                &realm_authority_pubkey,
                &token_owner.authority.pubkey(),
            );
        
        let transaction: Transaction =
            Transaction::new_signed_with_payer(
                &[
                    add_signatory_instruction,
                ],
                Some(&realm_authority_pubkey),
                &[
                    realm.authority,
                ],
                self.solana_client.get_latest_blockhash().unwrap(),
            );
        
        self.solana_client.send_and_confirm_transaction(&transaction)
            // .map(|_|
            //       Proposal {
            //           address: proposal.address,
            //           data: self.get_proposal_v2(&realm.data.community_mint, &realm.data.name, &governance.data.governed_account, governance.data.proposals_count as u8),
            //       }
            // )
    }*/

}
