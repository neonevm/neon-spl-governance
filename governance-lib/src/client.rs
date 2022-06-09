use {
    crate::errors::GovernanceLibError,
    borsh::BorshDeserialize,
    solana_sdk::{
        account::Account,
        account_utils::StateMut,
        borsh::try_from_slice_unchecked,
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        instruction::{AccountMeta, Instruction},
        transaction::Transaction,
        signer::{Signer, keypair::Keypair},
        signers::Signers,
        signature::Signature,
        program_pack::{Pack, IsInitialized},
        bpf_loader, bpf_loader_deprecated,
        bpf_loader_upgradeable::{self, UpgradeableLoaderState},
        loader_upgradeable_instruction::UpgradeableLoaderInstruction,
    },
    std::fmt,
    solana_client::{
        rpc_config::RpcSendTransactionConfig,
        rpc_client::RpcClient,
    },
};

pub struct Client<'a> {
    pub url: String,
    pub payer: &'a Keypair,
    pub solana_client: RpcClient,
}

pub type ClientResult<T> = std::result::Result<T, GovernanceLibError>;

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

    pub fn send_transaction(&self, transaction: &Transaction) -> ClientResult<Signature> {
        //self.solana_client.send_and_confirm_transaction(&transaction)
        self.solana_client.send_and_confirm_transaction_with_spinner_and_config(transaction, 
                self.solana_client.commitment(),
                RpcSendTransactionConfig {skip_preflight: true, ..RpcSendTransactionConfig::default()}).map_err(|e| e.into())
    }

    pub fn create_transaction_with_payer_only(
            &self,
            instructions: &[Instruction],
    ) -> ClientResult<Transaction> {
        self.create_transaction::<[&dyn solana_sdk::signature::Signer;0]>(
                instructions,
                &[],
            )
    }

    pub fn create_transaction<T: Signers>(
            &self,
            instructions: &[Instruction],
            signing_keypairs: &T,
    ) -> ClientResult<Transaction> {
        let mut transaction: Transaction =
            Transaction::new_with_payer(
                instructions,
                Some(&self.payer.pubkey()),
            );

        let blockhash = self.solana_client.get_latest_blockhash().unwrap();
        transaction.partial_sign(&[self.payer], blockhash);
        transaction.sign(signing_keypairs, blockhash);

        Ok(transaction)
    }

    pub fn send_and_confirm_transaction_with_payer_only(
            &self,
            instructions: &[Instruction],
    ) -> ClientResult<Signature> {
        self.send_and_confirm_transaction::<[&dyn solana_sdk::signature::Signer;0]>(
                instructions,
                &[],
            )
    }

    pub fn send_and_confirm_transaction<T: Signers>(
            &self,
            instructions: &[Instruction],
            signing_keypairs: &T,
    ) -> ClientResult<Signature> {
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
                RpcSendTransactionConfig {skip_preflight: true, ..RpcSendTransactionConfig::default()}).map_err(|e| e.into())
    }

    pub fn get_account(&self, account_key: &Pubkey) -> ClientResult<Option<Account>> {
        let account_info = self.solana_client.get_account_with_commitment(
                account_key, self.solana_client.commitment())?.value;
        Ok(account_info)
    }

    pub fn get_account_data_pack<T: Pack + IsInitialized>(
            &self,
            owner_program_id: &Pubkey,
            account_key: &Pubkey,
    ) -> ClientResult<Option<T>> {
        if let Some(account_info) = self.get_account(account_key)? {
            if account_info.data.is_empty() {
                return Err(GovernanceLibError::StateError(*account_key, "Account is empty".to_string()));
            }
            if account_info.owner != *owner_program_id {
                return Err(GovernanceLibError::StateError(*account_key,
                        format!("Invalid account owner: expect {}, actual {}", owner_program_id, account_info.owner)));
            }
        
            let account: T = T::unpack(&account_info.data)?;
            if !account.is_initialized() {
                return Err(GovernanceLibError::StateError(*account_key, "Uninitialized account".to_string()));
            }
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    pub fn get_account_data_borsh<T: BorshDeserialize + IsInitialized>(
            &self,
            owner_program_id: &Pubkey,
            account_key: &Pubkey,
    ) -> ClientResult<Option<T>> {
        if let Some(account_info) = self.get_account(account_key)? {
            if account_info.data.is_empty() {
                return Err(GovernanceLibError::StateError(*account_key, "Account is empty".to_string()));
            }
            if account_info.owner != *owner_program_id {
                return Err(GovernanceLibError::StateError(*account_key,
                        format!("Invalid account owner: expect {}, actual {}", owner_program_id, account_info.owner)));
            }
        
            let account: T = try_from_slice_unchecked(&account_info.data)?;
            if !account.is_initialized() {
                return Err(GovernanceLibError::StateError(*account_key, "Uninitialized account".to_string()));
            }
            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    pub fn get_program_upgrade_authority(
            &self,
            program_id: &Pubkey,
    ) -> ClientResult<Option<Pubkey>> {
        let programdata_address = self.get_program_data_address(program_id)?;
        let buffer_account = &self.solana_client.get_account(&programdata_address)?;
        if let Ok(UpgradeableLoaderState::ProgramData {upgrade_authority_address, ..}) = buffer_account.state() {
            Ok(upgrade_authority_address)
        } else {
            Err(GovernanceLibError::StateError(programdata_address, "Invalid associated PDA".to_string()))
        }
    }

    pub fn get_program_data_address(
            &self,
            program_id: &Pubkey,
    ) -> ClientResult<Pubkey> {
        let program_info = &self.solana_client.get_account(program_id)?;

        if program_info.owner == bpf_loader_upgradeable::id() {
            if let Ok(UpgradeableLoaderState::Program {programdata_address}) = program_info.state() {
                Ok(programdata_address)
            } else {
                Err(GovernanceLibError::StateError(*program_id, "Account is not upgradeable".to_string()))
            }
        } else {
            Err(GovernanceLibError::StateError(*program_id, "Unable to load program data: invalid owner".to_string()))
        }
    }

    pub fn set_program_upgrade_authority_instruction(
            &self,
            program_id: &Pubkey,
            upgrade_authority: &Pubkey,
            new_upgrade_authority: Option<&Pubkey>,
    ) -> ClientResult<Instruction> {
        let program_data_address = self.get_program_data_address(program_id)?;
        let mut accounts = vec![
            AccountMeta::new(program_data_address, false),
            AccountMeta::new_readonly(*upgrade_authority, true),
        ];
        if let Some(new_upgrade_authority) = new_upgrade_authority {
            accounts.push(AccountMeta::new_readonly(*new_upgrade_authority, false));
        }
    
        Ok(Instruction::new_with_bincode(
            bpf_loader_upgradeable::id(),
            &UpgradeableLoaderInstruction::SetAuthority,
            accounts,
        ))
    }

    pub fn get_program_data(
            &self,
            program_id: &Pubkey,
    ) -> ClientResult<Vec<u8>> {
        let program_info = &self.solana_client.get_account(program_id)?;

        if program_info.owner == bpf_loader::id() || program_info.owner == bpf_loader_deprecated::id() {
            Ok(program_info.data.clone())
        } else if program_info.owner == bpf_loader_upgradeable::id() {
            if let Ok(UpgradeableLoaderState::Program {programdata_address}) = program_info.state() {
                let buffer_account = &self.solana_client.get_account(&programdata_address)?;
                if let Ok(UpgradeableLoaderState::ProgramData {..}) = buffer_account.state() {
                    let offset = UpgradeableLoaderState::programdata_data_offset().unwrap_or(0);
                    let program_data = &buffer_account.data[offset..];
                    Ok(program_data.to_vec())
                } else {
                    Err(GovernanceLibError::StateError(programdata_address, "Invalid associated PDA".to_string()))
                }
            } else if let Ok(UpgradeableLoaderState::Buffer {..}) = program_info.state() {
                let offset = UpgradeableLoaderState::buffer_data_offset().unwrap_or(0);
                let program_data = &program_info.data[offset..];
                Ok(program_data.to_vec())
            } else {
                Err(GovernanceLibError::StateError(*program_id, "Account is not upgradeable".to_string()))
            }
        } else {
            Err(GovernanceLibError::StateError(*program_id, "Unable to load program data: invalid owner".to_string()))
        }
    }

    pub fn account_exists(&self, address: &Pubkey) -> bool {
        self.solana_client.get_account(address).is_ok()
    }

/*    pub fn _add_signatory(&self, realm: &Realm, _governance: &Governance, proposal: &Proposal, token_owner: &TokenOwner) -> Result<Signature,GovernanceLibError> {
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
